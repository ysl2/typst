use arrayvec::ArrayVec;
use smallvec::SmallVec;
use crate::geom::cmp::{value_no_nans, value_approx, position};
use super::*;

/// A data structure for fast, collisionless placement of objects into a group
/// of bezier shapes.
///
/// You can add free areas and blocked areas to the group. Objects can be placed
/// into the union of the free areas minus the union of the blocked areas.
#[derive(Debug, Clone)]
pub struct ShapeGroup {
    /// The rows which are made up of subslice ranges of the regions.
    rows: Vec<Row>,
    /// The regions row-by-row.
    regions: Vec<Region>,
    /// The accuracy used to construct this group.
    accuracy: f64,
}

/// A top- and bot-bounded row of regions.
#[derive(Debug, Clone)]
struct Row {
    /// The y-coordinate of the top end of the segment.
    top: f64,
    /// The y-coordinate of the bottom end of the segment.
    bot: f64,
    /// Which regions belong to this row.
    idxs: std::ops::Range<usize>,
}

/// A region defined by a left and right border.
#[derive(Debug, Clone)]
struct Region {
    /// The left border of the region.
    left: Monotone<PathSeg>,
    /// The right border of the region.
    right: Monotone<PathSeg>,
}

// Types for shape group construction.
#[derive(Copy, Clone)]
enum Kind { Old, New }
type Splits = Vec<f64>;
type Segment = Monotone<PathSeg>;
type Monotones = Vec<(Segment, Kind)>;

impl ShapeGroup {
    /// Create a new shape group.
    pub fn new(accuracy: f64) -> ShapeGroup {
        ShapeGroup {
            rows: vec![],
            regions: vec![],
            accuracy,
        }
    }

    /// Add a new area into which objects can be placed (`blocks = false`) /
    /// which objects need to evade (`blocks = true`).
    ///
    /// **Note:** When blocking objects are added all path segments which do not
    /// fall into previously added non-blocking paths are discarded because they
    /// have no immediate effect. Adding a non-blocking path later will not
    /// bring them back. It is recommended to add non-blocking paths first and
    /// blocking ones later.
    pub fn add(&mut self, path: &BezPath, blocks: bool) {
        // Split path into monotone subsegments and combine these with the old
        // border segments (which are already monotone). Accumulates all `y`
        // values at which curves need to be split such that all regions have
        // two non-intersecting borders in the same vertical range.
        let (monotone, splits) = self.split_monotone(path);

        // Applies the splits and returns rows of borders, which then need to be
        // coalesced into regions.
        let border_rows = self.apply_splits(monotone, splits);

        // Combine borders into pairs such that in the end all regions in the
        // shape will be created.
        self.create_regions(border_rows, blocks);
    }

    /// Split the old borders and the new path into monotone segments.
    fn split_monotone(&self, path: &BezPath) -> (Monotones, Splits) {
        let mut splits = vec![];
        let mut monotone = vec![];

        // Re-add the splits for the existing rows.
        for row in &self.rows {
            splits.push(row.top);
            splits.push(row.bot);
        }

        // Re-add the existing montone segments.
        for region in &self.regions {
            monotone.push((region.left, Kind::Old));
            monotone.push((region.right, Kind::Old));
        }

        let old_curves = monotone.len();

        // Split into monotone subsegments.
        for seg in path.segments() {
            for r in seg.extrema_ranges() {
                let subseg = Monotone(seg.subsegment(r));
                let (y1, y2) = (subseg.start().y, subseg.end().y);
                let subseg = if y1 > y2 { subseg.reverse() } else { subseg };
                monotone.push((subseg, Kind::New));
                splits.push(y1);
                splits.push(y2);
            }
        }

        // Split at intersection points.
        for (i, (a, _)) in monotone.iter().enumerate().skip(old_curves) {
            for (b, _) in &monotone[..i] {
                for p in a.intersect::<[_; 3]>(b, self.accuracy) {
                    splits.push(p.y);
                }
            }
        }

        // Make the splits unique.
        splits.sort_by(value_no_nans);
        splits.dedup_by(|a, b| a.approx_eq(&b, self.accuracy));

        (monotone, splits)
    }

    /// Create rows of borders by splitting the monotones.
    fn apply_splits(
        &self,
        monotone: Monotones,
        splits: Splits,
    ) -> Vec<Monotones> {
        // Fit the segments into rows of borders.
        let len = splits.len().saturating_sub(1);
        let mut borders = vec![vec![]; len];

        for (seg, kind) in monotone {
            let (top, bot) = (seg.start().y, seg.end().y);
            let find_k = |y| splits
                .binary_search_by(|v| value_approx(&v, &y, self.accuracy))
                .expect("splits should contain y");

            // Find out in which row the segment starts and in which it ends.
            let i = find_k(top);
            let j = find_k(bot);
            debug_assert!(i <= j);

            // Check into how many rows the segment falls.
            match j - i {
                // The segment is horizontal and thus uninteresting.
                0 => {}

                // The segment falls into one row.
                1 => borders[i].push((seg, kind)),

                // The segment falls into multiple rows. Add one subsegment for
                // each row.
                _ => {
                    let mut t0 = 0.0;

                    for k in i + 1 .. j {
                        let t = seg.solve_one_t_for_y(splits[k]);
                        borders[k - 1].push((seg.subsegment(t0 .. t), kind));
                        t0 = t;
                    }

                    borders[j - 1].push((seg.subsegment(t0 .. 1.0), kind));
                }
            }
        }

        borders
    }

    /// Create and store the rows & regions from the border rows.
    fn create_regions(&mut self, border_rows: Vec<Monotones>, new_blocks: bool) {
        self.rows.clear();
        self.regions.clear();

        // Coalesce borders into regions.
        for mut row in border_rows {
            let start = self.regions.len();

            let any = try_or!(row.first(), continue);
            let top = any.0.start().y;
            let bot = any.0.end().y;

            let mut left = None;
            let mut in_old = false;
            let mut in_new = false;

            // Sort the borders from left to right.
            //
            // Use the midpoints of the curve because the x-coordinate can be
            // equal at start and end, but in the middle they should be
            // different because we would have found an intersection otherwise.
            row.sort_by(|a, b| value_no_nans(&a.0.eval(0.5).x, &b.0.eval(0.5).x));

            for (border, kind) in row {
                match kind {
                    Kind::Old => in_old = !in_old,
                    Kind::New => in_new = !in_new,
                }

                // Check whether we are inside of the group or outside now.
                let inside = (!new_blocks && in_new) || (!in_new && in_old);

                if inside {
                    if left.is_none() {
                        left = Some(border);
                    }
                } else {
                    if let Some(left) = left.take() {
                        let right = border;
                        if !left.approx_eq(&right, self.accuracy) {
                            self.regions.push(Region { left, right: border });
                        }
                    }
                }
            }

            let end = self.regions.len();
            if end > start {
                self.rows.push(Row { top, bot, idxs: start .. end });
            }
        }
    }
}

impl ShapeGroup {
    /// Try to place an object into the shape group.
    ///
    /// This will find the top- and leftmost position in the shape group to
    /// place an object with the given size. The object will not collide with
    /// any shape in the group when placed at the returned point and it will be
    /// placed to the right and top of `min`.
    ///
    /// In the following image, the blue rectangle would be placed at the red
    /// point.
    ///
    /// <svg width="200" height="150" viewBox="0 0 200 150" fill="none">
    /// <path d="M56 141L20 9H81L180 141H56Z" stroke="black" stroke-width="2"/>
    /// <rect x="45" y="48" width="66" height="50" fill="#52A1FF"/>
    /// <circle cx="45" cy="48" r="4" fill="#EC2B2B"/>
    /// </svg>
    pub fn place(&self, min: Point, size: Size) -> Option<Point> {
        // Find out at which row we need to start our search.
        let start = self.find_first_row(min.y)?;

        for (i, top_row) in self.rows.iter().enumerate().skip(start) {
            let min_top = top_row.top.max(min.y);
            for (j, bot_row) in self.rows.iter().enumerate().skip(i) {
                // Too far to the top - is a middle row.
                if min_top + size.height > bot_row.bot {
                    continue;
                }

                // Too far to the bottom - cannot end here.
                if top_row.bot + size.height < bot_row.top {
                    break;
                }

                // The topmost solution found in this row combination.
                let mut topmost: Option<Point> = None;

                for (t, m, b) in self.combinations(i, j) {
                    // Ensure that the object is placed to the right and bottom
                    // of `min`.
                    let top = min.y.max(t.top());
                    let mut r = min.x.max(m.start) .. m.end;

                    // Shrink the range when we have a middle row because we
                    // then know that the bottom end of the top region and
                    // top end of the bottom region are tight.
                    if i != j {
                        let left = r.start
                            .max(t.left.end().x)
                            .max(b.left.start().x);

                        let right = r.end
                            .min(t.right.end().x)
                            .min(b.right.start().x);

                        r = left .. right;
                    }

                    let point = self.try_place(top, r, t, b, size);
                    if let Some(p) = point {
                        if topmost.map(|tm| p.y < tm.y).unwrap_or(true) {
                            topmost = point;
                        }
                    }
                }

                if topmost.is_some() {
                    return topmost;
                }
            }
        }

        None
    }

    /// Try to place the object into the given combination of regions.
    fn try_place(
        &self,
        top: f64,
        r: Range,
        t: &Region,
        b: &Region,
        size: Size,
    ) -> Option<Point> {
        // Ensure that the range is wide enough to hold the object.
        if r.end - r.start + self.accuracy < size.width {
            return None;
        }

        // The rectangle occupied by the object when placed at `p`.
        let bounds = |p| Rect::from_points(p, p + size.to_vec2())
            .inset((-2.0 * self.accuracy, 0.0));

        // Check placing directly at the top.
        let top_x = r.start
            .max(t.left.solve_max_x(top .. top + size.height))
            .max(b.left.solve_max_x(top .. top + size.height));

        let top_point = Point::new(top_x, top);
        let rect = bounds(top_point);

        if t.fits_right(rect) && b.fits_right(rect) {
            return Some(top_point);
        }

        // If it does not fit at the top, we have to try all ways in which the#
        // object could hit the borders and find the topmost one.
        let mut points = ArrayVec::<[Point; 11]>::new();

        // Check placing such that the object hits one of the curves at
        // the top and the bottom.
        let mx = TranslateScale::translate(Vec2::new(-size.width, 0.0));
        let my = TranslateScale::translate(Vec2::new(0.0, -size.height));
        let pairs = [
            (t.left, mx * t.right),
            (t.left, mx * my * b.right),
            (my * b.left, mx * t.right)
        ];

        for (left, right) in &pairs {
            // Skip left segments which are completely to the left of min.
            if left.right_point().x > r.start {
                points.extend(left.intersect::<[_; 3]>(right, self.accuracy));
            }
        }

        // Check placing such that the object hits one of the curves at the top
        // and one end of the range in the middle.
        let x1 = r.end - size.width;
        points.push(Point::new(x1, t.left.solve_one_y_for_x(x1)));

        let x2 = r.start;
        points.push(Point::new(x2, t.right.solve_one_y_for_x(x2 + size.width)));

        // Check the points from top to bottom and left to right.
        points.sort_by(|a, b| {
            value_approx(&a.y, &b.y, self.accuracy)
                .then_with(|| value_no_nans(&a.x, &b.x))
        });

        // Find and verify the best position.
        for p in points {
            let rect = bounds(p);
            let fits =
                top < rect.y0 + self.accuracy
                && rect.y1 < b.bot() + self.accuracy
                && rect.x0 > r.start
                && rect.x1 < r.end
                && t.fits(rect)
                && b.fits(rect);

            if fits {
                return Some(p);
            }
        }

        None
    }
}

impl ShapeGroup {
    /// Find all horizontal ranges that are fully inside the shape group in the
    /// given vertical range.
    ///
    /// In the following image, this would return the blue ranges when given the
    /// vertical range defined by the two red lines.
    ///
    /// <svg width="300" height="160" viewBox="0 0 300 160" fill="none">
    /// <path d="M32 154L67 6H259L228 154H177L117 35L108 154H32Z" stroke="black" stroke-width="2"/>
    /// <rect x="58" y="46" width="53" height="79" fill="#52A1FF"/>
    /// <rect x="162" y="46" width="72" height="79" fill="#52A1FF"/>
    /// <line y1="45" x2="300" y2="45" stroke="#EC2B2B" stroke-width="2"/>
    /// <line y1="125" x2="300" y2="125" stroke="#EC2B2B" stroke-width="2"/>
    /// </svg>
    pub fn ranges<'a>(
        &'a self,
        vr: Range
    ) -> impl Iterator<Item=Range> + 'a {
        struct MaybeIterator<I>(Option<I>);

        impl<I: Iterator<Item=Range>> Iterator for MaybeIterator<I> {
            type Item = Range;

            fn next(&mut self) -> Option<Range> {
                self.0.as_mut().and_then(|iter| iter.next())
            }
        }

        let maybe_i = self.find_row(vr.start);
        let maybe_j = self.find_row(vr.end);

        MaybeIterator(maybe_i.and_then(move |i| maybe_j.map(move |j| {
            self.combinations(i, j)
                .map(move |(t, r, b)| {
                    let tr = t.range(vr.clone());
                    let br = b.range(vr.clone());

                    r.start.max(tr.start).max(br.start)
                    .. r.end.min(tr.end).min(br.end)
                })
        })))
    }

    /// All overlapping combinations of top regions in row `i`, middle ranges
    /// and bottom regions in rows `i` and `j`, which are inside the shape.
    fn combinations(
        &self,
        i: usize,
        j: usize,
    ) -> impl Iterator<Item=(&Region, Range, &Region)> {
        let mut done = false;
        let mut top_regions = self.regions(i);
        let mut bot_regions = self.regions(j);
        let mut mid_regions: SmallVec<[_; 3]> = (i + 1 .. j)
            .map(|m| self.regions(m))
            .collect();

        // Ensure that the rows are contiguous.
        let mut last_bot = self.rows[i].bot;
        for row in &self.rows[i ..= j] {
            if row.top > last_bot + self.accuracy {
                done = true;
                break;
            }
            last_bot = row.bot;
        }

        // Compute the subranges which are inside the shape for the top region,
        // all middle rows and the bottom region.
        // This computes the intersection of the top & bottom regions outer
        // ranges with the middle regions inner ranges.
        std::iter::from_fn(move || loop {
            if done {
                return None;
            }

            let (t, b) = (&top_regions[0], &bot_regions[0]);
            let (tr, br) = (t.max_range(), b.max_range());

            let mut start = tr.start.max(br.start);
            let mut end = tr.end.min(br.end);
            let mut min = if tr.end < br.end {
                &mut top_regions
            } else {
                &mut bot_regions
            };

            for m in &mut mid_regions {
                let range = m[0].min_range();
                min = if range.end < end { m } else { min };
                start = start.max(range.start);
                end = end.min(range.end);
            }

            *min = &min[1..];
            done = min.is_empty();

            if start < end {
                return Some((t, start .. end, b));
            }
        })
    }
}

impl ShapeGroup {
    /// Find the row which contains the y-coordinate.
    fn find_row(&self, y: f64) -> Option<usize> {
        self.binary_search_row(y).ok()
    }

    /// Find the row which contains the y-coordinate or the topmost one below it.
    fn find_first_row(&self, y: f64) -> Option<usize> {
        match self.binary_search_row(y) {
            Ok(i) => Some(i),
            Err(i) if i < self.rows.len() => Some(i),
            _ => None,
        }
    }

    /// Binary search for the row which contains the `y` position.
    fn binary_search_row(&self, y: f64) -> Result<usize, usize> {
        self.rows.binary_search_by(|row| position(row.top .. row.bot, y))
    }

    /// Returns all regions contained in row `i`.
    fn regions(&self, i: usize) -> &[Region] {
        &self.regions[self.rows[i].idxs.clone()]
    }
}

impl ShapeGroup {
    /// Returns a path that can be used to render this group.
    pub fn renderable_path(&self) -> BezPath {
        let mut path = BezPath::new();
        for r in &self.regions {
            let top = PathSeg::Line(Line::new(r.left.start(), r.right.start()));
            let bot = PathSeg::Line(Line::new(r.right.end(), r.left.end()));

            path.extend(BezPath::from_path_segments(
                [top, r.right.0, bot, r.left.0.reverse()].iter().copied()
            ));
        }
        path
    }
}

impl Region {
    /// The region's top end.
    fn top(&self) -> f64 {
        self.left.start().y
    }

    /// The region's bottom end.
    fn bot(&self) -> f64 {
        self.left.end().y
    }

    /// The free horizontal range at this vertical range.
    fn range(&self, vr: Range) -> Range {
        self.left.solve_max_x(vr.clone()) .. self.right.solve_min_x(vr)
    }

    /// The maximal horizontal range (which surrounds the borders).
    fn max_range(&self) -> Range {
        self.left.left_point().x .. self.right.right_point().x
    }

    /// The minimal horizontal range (which is surrounded by the borders).
    fn min_range(&self) -> Range {
        self.left.right_point().x .. self.right.left_point().x
    }

    /// Whether the object fits in between the two borders.
    fn fits(&self, rect: Rect) -> bool {
        self.fits_left(rect) && self.fits_right(rect)
    }

    /// Whether the rect is to the right of the left border.
    fn fits_left(&self, rect: Rect) -> bool {
        rect.x0 > self.left.solve_max_x(rect.y0 .. rect.y1)
    }

    /// Whether the rect is to the left of the right border.
    fn fits_right(&self, rect: Rect) -> bool {
        rect.x1 < self.right.solve_min_x(rect.y0 .. rect.y1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RECT: &str       = "M32 35H92V95H32V35Z";
    const GAP_RECTS: &str  = "M17 21H77V31H17V21ZM17 37H77V47H17V37Z";
    const TRAPEZ: &str     = "M20 100L40 20H80L100 100H20Z";
    const SILO: &str       = "M20 100C20 100 28 32 40 20C52 8 66 8.5 80 20C94 31.5 100 100 100 100H20Z";
    const RTAILPLANE: &str = "M38 100L16 20H52.5L113 100H38Z";
    const LTAILPLANE: &str = "M20 100L65.5 20H99L83 100H20Z";
    const SKEWED: &str     = "M65 100C23.5 65 59 48 16 20H52.5C90.6 29 113 66.5 113 100H65Z";
    const HAT: &str        = "M65.5 27.5H21.5L29 64.5L15.5 104.5H98L80 64.5L65.5 27.5Z";
    const HIGH_HEEL: &str  = "M65 26L45 26C45 26 52.3727 60.5 25 81.2597C5.38123 96.1388 22 141 22 141H63V81.2597L100.273 108.89V141H158.5C158.5 141 164.282 85.5 105 82.5C82.0353 81.3379 65 26 65 26Z";
    const BUNTING: &str    = "M29.0452 86.5C27.5159 93.9653 26.1564 102.373 25 111.793L13 19H106.5L100.5 111.793C99.5083 103.022 97.8405 94.485 95.65 86.5C81.4874 34.8747 45.4731 6.3054 29.0452 86.5Z";
    const BIRD: &str       = "M42.5 88.5L8.5 60.5L21.5 52.5L31.5 20H99L42.5 88.5Z";
    const HAND: &str       = "M42.5 88.5L8.5 60.5V52.5H21.5L8.5 20H71.5L56.5 32.5L63 80L42.5 88.5Z" ;
    const ARROW: &str      = "M118 112L81 124L56 108L82 70H105L100 96L118 112Z";
    const ICEBERG: &str    = "M20 100L60.5 26.5L84 20L100 59L92.5 100H20Z";
    const CANYON: &str     = "M100 80.5H43L20.5 50.25L11.5 20H102L100 80.5Z";

    const RANGE_EXAMPLE: &str           = "M32 154L67 6H259L228 154H177L117 35L108 154H32Z";
    const COMPLEX_COMBINATIONS: &str    = "M15 13L10 53V113H115L107 55L97 16L15 13ZM28 86C23.8897 77.4238 24.0788 67.3044 32 62C37.5441 58.2875 43.1394 57.8052 49 61C58.0072 65.9101 57.8465 78.5969 52 87C48.1487 92.5355 43.5461 96.6998 37 95C32.0314 93.7098 30.2107 90.6126 28 86ZM75 92C63.9003 81.7541 77 54 77 54L92 63C92 63 96.7092 73.5217 97 81C97.2695 87.9287 99.6519 94.9456 94 99C87.457 103.694 80.9136 97.4587 75 92Z";
    const SHAPE_SELF_INTERSECTING: &str = "M35 10C54.93 0.66 81.26 8.94 88 30C94.74 51 91.6 83.97 79 92C66.41 100.025 56.5 96 52 77.5C47.5 59 88.95 52.2009 106 59C123 65.8 112.6 105.201 97 115C75.5 128.5 58.35 129.26 35 115C10.16 99.83 4 72.1173 12 44C16.36 28.6 20.6 16.7567 35 10Z";
    const CURVE_SELF_INTERSECTING: &str = "M91 25C-44.3443 133 174.934 133 27 25H91Z";

    fn path(svg: &str) -> BezPath {
        BezPath::from_svg(svg).unwrap()
    }

    // ---------------------------------------------------------------------- //
    // These tests check shape group construction from one or multiple shapes.

    macro_rules! test_build {
        ($name:ident
            paths: [$($path:expr => $blocks:expr),* $(,)?],
            accuracy: $accuracy:expr,
            rows: $rows:expr,
            regions: $regions:expr,
        ) => {
            #[test]
            fn $name() {
                #[allow(unused_mut)]
                let mut group = ShapeGroup::new($accuracy);
                $(group.add(&path($path), $blocks);)*
                assert_eq!(group.rows.len(), $rows);
                assert_eq!(group.regions.len(), $regions);
            }
        }
    }

    test_build! {
        test_build_group_without_any_shapes_is_empty
            paths: [],
            accuracy: 1e-2,
            rows: 0,
            regions: 0,
    }

    test_build! {
        test_build_group_from_one_simple_shape_that_has_only_one_region
            paths: [TRAPEZ => false],
            accuracy: 1e-2,
            rows: 1,
            regions: 1,
    }

    test_build! {
        test_build_group_with_only_blocking_shapes_is_empty
            paths: [BUNTING => true, RTAILPLANE => true],
            accuracy: 1e-2,
            rows: 0,
            regions: 0,
    }

    test_build! {
        test_build_group_from_union_of_shape_and_contained_shape
            paths: [TRAPEZ => false, SILO => false],
            accuracy: 1e-2,
            rows: 2,
            regions: 2,
    }

    test_build! {
        test_build_group_from_union_of_overlapping_shapes
            paths: [LTAILPLANE => false, RTAILPLANE => false],
            accuracy: 1e-2,
            rows: 4,
            regions: 5,
    }

    test_build! {
        test_build_group_from_union_of_nonoverlapping_shapes
            paths: [BIRD => false, ARROW => false],
            accuracy: 1e-2,
            rows: 8,
            regions: 9,
    }

    test_build! {
        test_build_group_from_difference_of_overlapping_shapes
            paths: [BUNTING => false, RTAILPLANE => true],
            accuracy: 1e-2,
            rows: 9,
            regions: 15,
    }

    test_build! {
        test_build_group_from_difference_of_non_overlapping_shapes
            paths: [BIRD => false, ARROW => true],
            accuracy: 1e-2,
            rows: 4,
            regions: 4,
    }

    test_build! {
        test_build_group_from_shape_with_self_intersection
            paths: [SHAPE_SELF_INTERSECTING => false],
            accuracy: 0.1,
            rows: 14,
            regions: 21,
    }

    test_build! {
        test_build_group_from_shape_with_self_intersecting_curve
            paths: [CURVE_SELF_INTERSECTING => false],
            accuracy: 1e-2,
            rows: 4,
            regions: 4,
    }

    test_build! {
        test_build_group_from_difference_of_shape_with_itself
            paths: [BIRD => false, BIRD => true],
            accuracy: 1e-2,
            rows: 0,
            regions: 0,
    }

    // ---------------------------------------------------------------------- //
    // These tests check all different combinations of borders & regions which
    // influence placement.

    // In the following table there is a list of which test tests what. The
    // `left` column specifies which part of the left border blocks the shape
    // and `right` which part of the right border. When there is an `x` in the
    // `right` column, the right border does not block at all, which means that
    // the object can be placed at the top. The `top = bot` column says of
    // the object was placed into a single row.
    //
    // left | right | top = bot | tests
    // -----|-------|-----------|-----------------------------------------
    // top  |   x   |   true    | trapez_top, hat_top
    // top  |  top  |   true    | trapez_mid, silo, hat_bot, high_heel_top,
    //                          | self-intersecting
    // top  |  bot  |   true    | ltailplane
    // bot  |  top  |   true    | rtailplane, skewed
    // top  |   x   |   false   | bird
    // mid  |   x   |   false   | hand
    // bot  |   x   |   false   | bunting, canyon
    // top  |  top  |   false   | high_heel_left
    // top  |  mid  |   false   | arrow
    // top  |  bot  |   false   | iceberg
    // mid  |  top  |   false   | hat_mid
    // bot  |  top  |   false   | high_heel_right

    macro_rules! test_place {
        ($name:ident
            path: $path:expr,
            min: $min:expr,
            size: $size:expr,
            point: $point:expr,
            accuracy: $accuracy:expr,
            tolerance: $tolerance:expr,
        ) => {
            #[test]
            fn $name() {
                let shape = path($path);
                let mut group = ShapeGroup::new($accuracy);
                group.add(&shape, false);
                let result = group.place($min, $size);
                assert_approx_eq!(result, $point, tolerance = $tolerance);
            }
        }
    }

    test_place! {
        test_place_into_rect_fits
            path: RECT,
            min: Point::ZERO,
            size: Size::new(40.0, 20.0),
            point: Some(Point::new(32.0, 35.0)),
            accuracy: 1e-2,
            tolerance: 1e-2,
    }

    test_place! {
        test_place_into_rect_fits_exactly
            path: RECT,
            min: Point::ZERO,
            size: Size::new(60.0, 60.0),
            point: Some(Point::new(32.0, 35.0)),
            accuracy: 1e-2,
            tolerance: 1e-2,
    }

    test_place! {
        test_place_into_rect_does_not_fit
            path: RECT,
            min: Point::ZERO,
            size: Size::new(30.0, 61.0),
            point: None,
            accuracy: 1e-2,
            tolerance: 1e-2,
    }

    test_place! {
        test_place_into_gap_rects_does_not_fit
            path: GAP_RECTS,
            min: Point::ZERO,
            size: Size::new(30.0, 20.0),
            point: None,
            accuracy: 1e-2,
            tolerance: 1e-2,
    }

    test_place! {
        test_place_into_trapez
            path: TRAPEZ,
            min: Point::ZERO,
            size: Size::new(50.0, 15.0),
            point: Some(Point::new(35.0, 40.0)),
            accuracy: 1e-2,
            tolerance: 1e-2,
    }

    test_place! {
        test_place_into_trapez_top
            path: TRAPEZ,
            min: Point::ZERO,
            size: Size::new(20.0, 12.0),
            point: Some(Point::new(40.0, 20.0)),
            accuracy: 1e-2,
            tolerance: 1e-2,
    }

    test_place! {
        test_place_into_trapez_with_min_x
            path: TRAPEZ,
            min: Point::new(60.0, 30.0),
            size: Size::new(25.0, 10.0),
            point: Some(Point::new(60.0, 40.0)),
            accuracy: 1e-2,
            tolerance: 1e-2,
    }

    test_place! {
        test_place_into_trapez_with_min_y
            path: TRAPEZ,
            min: Point::new(30.0, 56.0),
            size: Size::new(30.0, 10.0),
            point: Some(Point::new(31.0, 56.0)),
            accuracy: 1e-2,
            tolerance: 1e-2,
    }

    test_place! {
        test_place_into_trapez_top_with_min_x
            path: TRAPEZ,
            min: Point::new(60.0, 30.0),
            size: Size::new(20.0, 10.0),
            point: Some(Point::new(60.0, 30.0)),
            accuracy: 1e-2,
            tolerance: 1e-2,
    }

    test_place! {
        test_place_into_silo
            path: SILO,
            min: Point::ZERO,
            size: Size::new(70.0, 30.0),
            point: Some(Point::new(25.5, 65.0)),
            accuracy: 1e-2,
            tolerance: 0.5,
    }

    test_place! {
        test_place_into_rtailplane
            path: RTAILPLANE,
            min: Point::ZERO,
            size: Size::new(40.0, 30.0),
            point: Some(Point::new(31.0, 45.0)),
            accuracy: 1e-2,
            tolerance: 1.0,
    }

    test_place! {
        test_place_into_ltailplane
            path: LTAILPLANE,
            min: Point::ZERO,
            size: Size::new(38.0, 15.0),
            point: Some(Point::new(54.0, 40.0)),
            accuracy: 1e-2,
            tolerance: 1.0,
    }

    test_place! {
        test_place_into_skewed
            path: SKEWED,
            min: Point::ZERO,
            size: Size::new(50.0, 17.0),
            point: Some(Point::new(41.5, 44.0)),
            accuracy: 1e-2,
            tolerance: 0.25,
    }

    test_place! {
        test_place_into_hat_top
            path: HAT,
            min: Point::ZERO,
            size: Size::new(35.0, 30.0),
            point: Some(Point::new(28.0, 28.0)),
            accuracy: 1e-2,
            tolerance: 1.0,
    }

    test_place! {
        test_place_into_hat_mid
            path: HAT,
            min: Point::ZERO,
            size: Size::new(43.0, 30.0),
            point: Some(Point::new(29.0, 44.0)),
            accuracy: 1e-2,
            tolerance: 0.1,
    }

    test_place! {
        test_place_into_hat_bot
            path: HAT,
            min: Point::ZERO,
            size: Size::new(65.0, 12.0),
            point: Some(Point::new(23.0, 83.0)),
            accuracy: 1e-2,
            tolerance: 1.0,
    }

    test_place! {
        test_place_into_high_heel_top
            path: HIGH_HEEL,
            min: Point::ZERO,
            size: Size::new(32.0, 12.0),
            point: Some(Point::new(44.0, 52.0)),
            accuracy: 1e-2,
            tolerance: 1.0,
    }

    test_place! {
        test_place_into_high_heel_left
            path: HIGH_HEEL,
            min: Point::new(0.0, 60.0),
            size: Size::new(46.0, 17.0),
            point: Some(Point::new(17.0, 94.0)),
            accuracy: 1e-2,
            tolerance: 0.5,
    }

    test_place! {
        test_place_into_high_heel_right
            path: HIGH_HEEL,
            min: Point::ZERO,
            size: Size::new(50.0, 17.0),
            point: Some(Point::new(100.0, 106.0)),
            accuracy: 1e-2,
            tolerance: 1.0,
    }

    test_place! {
        test_place_into_bunting
            path: BUNTING,
            min: Point::ZERO,
            size: Size::new(28.0, 19.0),
            point: Some(Point::new(15.5, 19.0)),
            accuracy: 1e-2,
            tolerance: 1.0,
    }

    test_place! {
        test_place_into_bird
            path: BIRD,
            min: Point::ZERO,
            size: Size::new(26.0, 39.0),
            point: Some(Point::new(32.0, 20.0)),
            accuracy: 1e-2,
            tolerance: 1.0,
    }

    test_place! {
        test_place_into_hand
            path: HAND,
            min: Point::ZERO,
            size: Size::new(31.0, 42.0),
            point: Some(Point::new(21.5, 20.0)),
            accuracy: 1e-2,
            tolerance: 1.0,
    }

    test_place! {
        test_place_into_arrow
            path: ARROW,
            min: Point::ZERO,
            size: Size::new(30.0, 15.0),
            point: Some(Point::new(70.0, 87.5)),
            accuracy: 1e-2,
            tolerance: 0.1,
    }

    test_place! {
        test_place_into_iceberg
            path: ICEBERG,
            min: Point::ZERO,
            size: Size::new(53.0, 24.0),
            point: Some(Point::new(42.5, 59.0)),
            accuracy: 1e-2,
            tolerance: 0.25,
    }

    test_place! {
        test_place_into_canyon
            path: CANYON,
            min: Point::ZERO,
            size: Size::new(53.0, 44.0),
            point: Some(Point::new(31.0, 20.0)),
            accuracy: 1e-2,
            tolerance: 1.0,
    }

    test_place! {
        test_place_into_shape_self_intersecting_with_min
            path: SHAPE_SELF_INTERSECTING,
            min: Point::new(50.0, 48.0),
            size: Size::new(22.0, 17.0),
            point: Some(Point::new(91.0, 66.0)),
            accuracy: 1e-2,
            tolerance: 1.0,
    }

    // ---------------------------------------------------------------------- //
    // These tests check the combinations & ranges.

    #[test]
    fn test_middle_ranges_for_complex_combinations() {
        let mut group = ShapeGroup::new(1e-2);
        group.add(&path(COMPLEX_COMBINATIONS), false);
        assert_approx_eq!(
            group.combinations(1, 20).map(|t| t.1).collect::<Vec<_>>(),
            vec![10.0 .. 25.35, 56.1 .. 70.5, 97.7 .. 106.5],
            tolerance = 0.1,
        );
    }

    #[test]
    fn test_bunting_ranges() {
        let mut group = ShapeGroup::new(1e-2);
        group.add(&path(BUNTING), false);
        assert_approx_eq!(
            group.ranges(30.0 .. 54.0).collect::<Vec<_>>(),
            vec![17.5 .. 39.5, 81.5 .. 104.5],
            tolerance = 0.5,
        );
    }

    #[test]
    fn test_example_ranges() {
        let mut group = ShapeGroup::new(1e-2);
        group.add(&path(RANGE_EXAMPLE), false);
        assert_approx_eq!(
            group.ranges(46.0 .. 126.0).collect::<Vec<_>>(),
            vec![58.0 .. 111.0, 162.0 .. 234.0],
            tolerance = 1.0,
        );
    }

    #[test]
    fn test_ranges_with_vertical_ranges_out_of_row() {
        let mut group = ShapeGroup::new(1e-2);
        group.add(&path(RECT), false);
        assert_eq!(group.ranges(30.0 .. 60.0).next(), None);
    }

    #[test]
    fn test_ranges_with_vertical_gap() {
        let mut group = ShapeGroup::new(1e-2);
        group.add(&path(GAP_RECTS), false);
        assert_eq!(group.ranges(25.0 .. 40.0).next(), None);
    }
}
