use arrayvec::ArrayVec;
use super::range::value_relative_to_range;
use super::*;

/// A data structure for fast, collisionless placement of objects into a group
/// of bezier shapes.
///
/// You can add free areas and blocked areas to the group. Objects can be placed
/// into the union of the free areas minus the union of the blocked areas.
#[derive(Debug, Clone)]
pub struct ShapeGroup {
    /// The rows containing subslice range of its regions.
    rows: Vec<Row>,
    /// The regions row-by-row.
    regions: Vec<Region>,
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

/// Whether a segment is old or new in an add operation.
#[derive(Debug, Copy, Clone)]
enum Kind {
    Old,
    New,
}

type Splits = Vec<f64>;
type Segment = Monotone<PathSeg>;
type Monotones = Vec<(Segment, Kind)>;

impl ShapeGroup {
    /// Create a new shape group.
    pub fn new() -> ShapeGroup {
        ShapeGroup {
            rows: vec![],
            regions: vec![],
        }
    }

    /// Add a new area into which objects can be placed (`blocks = false`) /
    /// which objects need to evade (`blocks = true`)
    pub fn add(&mut self, path: &BezPath, accuracy: f64, blocks: bool) {
        // Split path into monotone subsegments and combine these with the old
        // border segments (which are already monotone). Accumulates all `y`
        // values at which curves need to be split such that all regions have
        // two non-intersecting borders in the same vertical range.
        let (monotone, splits) = self.split_monotone(path, accuracy);

        // Applies the splits and returns rows of borders, which then need to be
        // coalesced into regions.
        let border_rows = Self::apply_splits(monotone, splits, accuracy);

        // Combine borders into pairs such that in the end all regions in the
        // shape will be created.
        self.create_regions(border_rows, blocks);
    }

    /// Split the old borders and the new path into monotone segments.
    fn split_monotone(&self, path: &BezPath, accuracy: f64) -> (Monotones, Splits) {
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

        // Splits at intersection points.
        for (i, (a, _)) in monotone.iter().enumerate().skip(old_curves) {
            for (b, _) in &monotone[..i] {
                for p in a.intersect::<[_; 3]>(b, accuracy) {
                    splits.push(p.y);
                }
            }
        }

        // Make the splits unique.
        splits.sort_by(value_no_nans);
        splits.dedup_by(|a, b| a.approx_eq(&b, accuracy));

        (monotone, splits)
    }

    /// Create rows of borders by splitting the monotones.
    fn apply_splits(
        monotone: Monotones,
        splits: Splits,
        accuracy: f64,
    ) -> Vec<Monotones> {
        // Fit the segments into rows of borders.
        let len = splits.len().saturating_sub(1);
        let mut borders = vec![vec![]; len];

        for (seg, kind) in monotone {
            let (y1, y2) = (seg.start().y, seg.end().y);
            let find_k = |y| splits
                .binary_search_by(|v| value_approx(&v, &y, accuracy))
                .expect("splits should contain y");

            // Find out in which row the segment start and in which it ends.
            let k1 = find_k(y1);
            let k2 = find_k(y2);
            assert!(k1 <= k2);

            // Check into how many rows the segment falls.
            match k2 - k1 {
                // The segment is horizontal and thus uninteresting.
                0 => {}

                // The segment falls into one row.
                1 => borders[k1].push((seg, kind)),

                // The segment falls into multiple rows. Add one subsegment for
                // each row.
                _ => {
                    let mut t0 = 0.0;

                    for ki in k1 + 1 .. k2 {
                        let t = seg.solve_t_for_y(splits[ki])[0];
                        borders[ki - 1].push((seg.subsegment(t0 .. t), kind));
                        t0 = t;
                    }

                    borders[k2 - 1].push((seg.subsegment(t0 .. 1.0), kind));
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
        for row in border_rows {
            let start = self.regions.len();

            let any = try_opt_or!(row.first(), continue);
            let top = any.0.start().y;
            let bot = any.0.end().y;

            let mut left = None;
            let mut in_old = false;
            let mut in_new = false;

            for (border, kind) in row {
                match kind {
                    Kind::Old => in_old = !in_old,
                    Kind::New => in_new = !in_new,
                }

                // Check whether we are inside of the group or outside now.
                let inside = (!new_blocks && in_new) || (!in_new && in_old);
                if inside {
                    left = Some(border);
                } else if let Some(left) = left {
                    self.regions.push(Region { left, right: border })
                }
            }

            let idxs = start .. self.regions.len();
            self.rows.push(Row { top, bot, idxs });
        }
    }
}

impl ShapeGroup {
    /// Place an object into the shape group.
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
    pub fn place(
        &self,
        min: Point,
        size: Size,
        accuracy: f64,
    ) -> Option<Point> {
        // Find out which row contains the minimum y coordinate or is the first
        // one below it.
        let start = self.find_topmost_row(min.y)?;

        // Walk over the rows where the top edge of the object can fall into.
        // The first candidate row is determined by the min-point's
        // `y`-coordinate.
        for (i, top_row) in self.rows.iter().enumerate().skip(start) {
            let min_top = top_row.top.max(min.y);
            let max_bot = top_row.bot;
            assert!(min_top <= max_bot);

            // Walk over the bottom rows where an object starting in `top_row`
            // can end in.
            for (j, bot_row) in self.rows.iter().enumerate().skip(i) {
                // Too far to the top - is a middle row.
                if min_top + size.height > bot_row.bot {
                    continue;
                }

                // Too far to the bottom - cannot end here.
                if max_bot + size.height < bot_row.top {
                    break;
                }

                let mut best: Option<Point> = None;

                // Walk through the horizontal ranges where an object that
                // starts in `t` and ends in `b` can be placed (these depend
                // also on the rows in between `t` and `b`).
                for (range, top_region, bot_region) in self.region_ranges(i, j, min.x) {
                    let point = self.try_place_into(
                        range,
                        min.y,
                        size,
                        top_region,
                        bot_region,
                        accuracy
                    );

                    if let Some(p) = point {
                        if best.map(|b| p.y < b.y).unwrap_or(true) {
                            best = point;
                        }
                    }
                }

                if best.is_some() {
                    return best;
                }
            }
        }

        None
    }

    /// Find the topmost row which contains the `y` coordinate or is below it.
    fn find_topmost_row(&self, y: f64) -> Option<usize> {
        match self.rows.binary_search_by(|row| {
            value_relative_to_range(row.top .. row.bot, y)
        }) {
            Ok(i) => Some(i),
            Err(i) if i < self.rows.len() => Some(i),
            _ => None,
        }
    }

    /// Returns all ranges and the top & bottom region they fall into,
    /// respectively, for top row `i` and bottom row `j`.
    fn region_ranges(
        &self,
        i: usize,
        j: usize,
        min_x: f64,
    ) -> impl Iterator<Item=(Range, &Region, &Region)> {
        assert!(i <= j);

        let mut done = false;
        let mut top_regions = self.regions(i);
        let mut bot_regions = self.regions(j);
        let mut mid_regions: Vec<_> = (i + 1 .. j)
            .map(|m| self.regions(m))
            .collect();

        // Compute the subranges where there is a region for all rows - which is
        // basically the intersection between the row's regions.
        std::iter::from_fn(move || loop {
            if done {
                return None;
            }

            let (t, b) = (&top_regions[0], &bot_regions[0]);
            let (to, bo) = (t.outer(), b.outer());

            let mut start = min_x.max(to.start).max(bo.start);
            let mut end = to.end.min(bo.end);
            let mut min = if to.end < bo.end {
                &mut top_regions
            } else {
                &mut bot_regions
            };

            for m in &mut mid_regions {
                let range = m[0].inner();
                min = if range.end < end { m } else { min };
                start = start.max(range.start);
                end = end.min(range.end);
            }

            *min = &min[1..];
            done = min.is_empty();

            if start < end {
                return Some((start .. end, t, b));
            }
        })
    }

    /// Try to place the object into the given range, starting in region `top_region`
    /// and ending in region `bot_region`.
    fn try_place_into(
        &self,
        range: Range,
        min_y: f64,
        size: Size,
        top_region: &Region,
        bot_region: &Region,
        accuracy: f64,
    ) -> Option<Point> {
        // The object cannot fit if the range is not wide enough.
        if range.end - range.start + accuracy < size.width {
            return None;
        }

        // The rectangle occupied by the object when placed at `p`.
        let rect = |p| {
            Rect::from_points(p, p + size.to_vec2())
                .inset((-2.0 * accuracy, 0.0))
        };

        let solve_max_x = |seg: &Monotone<PathSeg>, range: Range| {
            solve_one_x(seg, range.start, accuracy)
                .max(solve_one_x(seg, range.end, accuracy))
        };

        let solve_min_x = |seg: &Monotone<PathSeg>, range: Range| {
            solve_one_x(seg, range.start, accuracy)
                .min(solve_one_x(seg, range.end, accuracy))
        };

        // Check that the rectangle does not collide with the left borders.
        let check_left = |rect: Rect| {
            rect.x0 > range.start
            && rect.x0 > solve_max_x(&top_region.left, rect.y0 .. rect.y1)
            && rect.x0 > solve_max_x(&bot_region.left, rect.y0 .. rect.y1)
        };

        // Check that the rectangle does not collide with the right borders.
        let check_right = |rect: Rect| {
            rect.x1 < range.end
            && rect.x1 < solve_min_x(&top_region.right, rect.y0 .. rect.y1)
            && rect.x1 < solve_min_x(&bot_region.right, rect.y0 .. rect.y1)
        };

        // Check that the rectangle does not collide with the top & bottom end
        // of the row.
        let min_top = top_region.top().max(min_y);
        let check_top_bot = |rect: Rect| {
            min_top < rect.y0 + accuracy && bot_region.bot() > rect.y1 - accuracy
        };

        // ------------------------------------------------------------------ //
        // Try placing directly at the top border.

        // Find out the x-position for placing at the top border.
        let x = range.start
            .max(solve_max_x(&top_region.left, min_top .. min_top + size.height))
            .max(solve_max_x(&bot_region.left, min_top .. min_top + size.height));

        // If it fits at the top, it ain't getting better.
        let point = Point::new(x, min_top);
        if check_right(rect(point)) {
            return Some(point);
        }

        // ------------------------------------------------------------------ //
        // If it won't fit at the top, search for the left and top-most place.
        // The best current candidate is `best`.

        let mut best: Option<Point> = None;
        let mut check = |point: Point| {
            // Check that we even want that solution before verifying it.
            if let Some(b) = best {
                if b.y < point.y {
                    return;
                }

                if b.y.approx_eq(&point.y, accuracy) && b.x < point.x {
                    return;
                }
            }

            let rect = rect(point);
            if check_left(rect) && check_right(rect) && check_top_bot(rect) {
                best = Some(point);
            }
        };

        let mx = TranslateScale::translate(Vec2::new(-size.width, 0.0));
        let my = TranslateScale::translate(Vec2::new(0.0, -size.height));

        // ------------------------------------------------------------------ //
        // Try such that curve fits tightly with one of the borders and with a
        // middle row on the other side.

        let mut check_border_mid = |seg: &Monotone<PathSeg>, x| {
            match seg.solve_t_for_x(x).as_slice() {
                [] => {}
                [t] => check(seg.eval(*t)),
                _ => panic!("curve is not monotone"),
            }
        };

        let left = range.start.max(top_region.left.end().x).max(bot_region.left.start().x);
        let right = range.end.min(top_region.right.end().x).max(bot_region.right.start().x);

        check_border_mid(&top_region.left, right - size.width);
        check_border_mid(&(mx * top_region.right), left);

        // ------------------------------------------------------------------ //
        // Try such that curves fit tightly with borders.

        let mut check_all = |points: ArrayVec<[Point; 3]>| {
            for p in points {
                check(p);
            }
        };

        check_all(top_region.left.intersect(&(mx * top_region.right), accuracy));
        check_all(top_region.left.intersect(&(mx * my * bot_region.right), accuracy));
        check_all((my * bot_region.left).intersect(&(mx * top_region.right), accuracy));

        best
    }
}

impl ShapeGroup {
    /// Finds all horizontal ranges that are fully inside the shape group in the
    /// given vertical range.
    ///
    /// In the following image, this would return the blue ranges when given
    /// the vertical range defined by the two red lines.
    ///
    /// <svg width="300" height="160" viewBox="0 0 300 160" fill="none">
    /// <rect x="58" y="46" width="53" height="79" fill="#52A1FF"/>
    /// <rect x="162" y="46" width="72" height="79" fill="#52A1FF"/>
    /// <path d="M32 154L67 6H259L228 154H177L117 35L108 154H32Z" stroke="black" stroke-width="2"/>
    /// <line y1="45" x2="300" y2="45" stroke="#EC2B2B" stroke-width="2"/>
    /// <line y1="125" x2="300" y2="125" stroke="#EC2B2B" stroke-width="2"/>
    /// </svg>
    pub fn ranges(&self, vrange: Range) -> impl IntoIterator<Item=Range> {
        #![allow(unused)]
        todo!("ranges");
        std::iter::empty()
    }
}

impl ShapeGroup {
    /// Returns all regions contained in row `i`.
    fn regions(&self, i: usize) -> &[Region] {
        &self.regions[self.rows[i].idxs.clone()]
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

    /// The horizontal range which surrounds the borders.
    fn outer(&self) -> Range {
        self.left_min() .. self.right_max()
    }

    /// The horizontal range which is surrounded by the borders.
    fn inner(&self) -> Range {
        self.left_max() .. self.right_min()
    }

    /// The maximum x value of the left border.
    fn left_max(&self) -> f64 {
        self.left.start().x.max(self.left.end().x)
    }

    /// The minimum x value of the left border.
    fn left_min(&self) -> f64 {
        self.left.start().x.min(self.left.end().x)
    }

    /// The maximum x value of the right border.
    fn right_max(&self) -> f64 {
        self.right.start().x.max(self.right.end().x)
    }

    /// The minimum x value of the right border.
    fn right_min(&self) -> f64 {
        self.right.start().x.min(self.right.end().x)
    }
}

/// Tries to to find an `x` position at which the curve has the given `y` value.
/// The `y` value is clamped into the valid y-range for the curve.
///
/// The curve must be monotonic and the min-max rectangle defined by start and
/// end point must be a bounding box for the curve.
fn solve_one_x<C>(seg: &Monotone<C>, y: f64, accuracy: f64) -> f64
where
    C: ParamCurveSolve
{
    let start = seg.start();
    if y < start.y + accuracy {
        return start.x;
    }

    let end = seg.end();
    if y > end.y - accuracy {
        return end.x;
    }

    match seg.solve_x_for_y(y).as_slice() {
        [x] => *x,
        [] => panic!("there should be at least one root"),
        _ => panic!("curve is not monotone"),
    }
}

#[cfg(test)]
mod tests {
    use super::super::BezPath;
    use super::*;

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
    const ARROW: &str      = "M90 61.5L53.5 74L28.5 58L54.5 20H77.5L72 45.5L90 61.5Z";
    const ICEBERG: &str    = "M20 100L60.5 26.5L84 20L100 59L92.5 100H20Z";
    const CANYON: &str     = "M100 80.5H43L20.5 50.25L11.5 20H102L100 80.5Z";

    macro_rules! test {
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
                let shape = BezPath::from_svg($path).unwrap();
                let mut group = ShapeGroup::new();
                group.add(&shape, $accuracy, false);
                let result = group.place($min, $size, $accuracy);
                assert_approx_eq!(result, Some($point), tolerance = $tolerance);
            }
        }
    }

    test! {
        test_place_into_trapez
            path: TRAPEZ,
            min: Point::ZERO,
            size: Size::new(50.0, 15.0),
            point: Point::new(35.0, 40.0),
            accuracy: 1e-2,
            tolerance: 1e-2,
    }

    test! {
        test_place_into_trapez_with_min_x
            path: TRAPEZ,
            min: Point::new(60.0, 30.0),
            size: Size::new(25.0, 10.0),
            point: Point::new(60.0, 40.0),
            accuracy: 1e-2,
            tolerance: 1e-2,
    }

    test! {
        test_place_into_trapez_with_min_y
            path: TRAPEZ,
            min: Point::new(30.0, 56.0),
            size: Size::new(30.0, 10.0),
            point: Point::new(31.0, 56.0),
            accuracy: 1e-2,
            tolerance: 1e-2,
    }

    test! {
        test_place_into_silo
            path: SILO,
            min: Point::ZERO,
            size: Size::new(70.0, 30.0),
            point: Point::new(25.5, 65.0),
            accuracy: 1e-2,
            tolerance: 0.5,
    }

    test! {
        test_place_into_rtailplane
            path: RTAILPLANE,
            min: Point::ZERO,
            size: Size::new(40.0, 30.0),
            point: Point::new(31.0, 45.0),
            accuracy: 1e-2,
            tolerance: 1.0,
    }

    test! {
        test_place_into_ltailplane
            path: LTAILPLANE,
            min: Point::ZERO,
            size: Size::new(38.0, 15.0),
            point: Point::new(54.0, 40.0),
            accuracy: 1e-2,
            tolerance: 1.0,
    }

    test! {
        test_place_into_skewed
            path: SKEWED,
            min: Point::ZERO,
            size: Size::new(50.0, 17.0),
            point: Point::new(41.5, 44.0),
            accuracy: 1e-2,
            tolerance: 0.25,
    }

    test! {
        test_place_into_top_of_hat
            path: HAT,
            min: Point::ZERO,
            size: Size::new(35.0, 30.0),
            point: Point::new(28.0, 28.0),
            accuracy: 1e-2,
            tolerance: 1.0,
    }

    test! {
        test_place_into_mid_of_hat
            path: HAT,
            min: Point::ZERO,
            size: Size::new(43.0, 30.0),
            point: Point::new(29.0, 44.0),
            accuracy: 1e-2,
            tolerance: 0.1,
    }

    test! {
        test_place_into_bot_of_hat
            path: HAT,
            min: Point::ZERO,
            size: Size::new(65.0, 12.0),
            point: Point::new(23.0, 83.0),
            accuracy: 1e-2,
            tolerance: 1.0,
    }

    test! {
        test_place_into_top_of_high_heel
            path: HIGH_HEEL,
            min: Point::ZERO,
            size: Size::new(32.0, 12.0),
            point: Point::new(44.0, 52.0),
            accuracy: 1e-2,
            tolerance: 1.0,
    }

    test! {
        test_place_into_left_of_high_heel
            path: HIGH_HEEL,
            min: Point::new(0.0, 60.0),
            size: Size::new(46.0, 17.0),
            point: Point::new(17.0, 94.0),
            accuracy: 1e-2,
            tolerance: 0.5,
    }

    test! {
        test_place_into_right_of_high_heel
            path: HIGH_HEEL,
            min: Point::ZERO,
            size: Size::new(50.0, 17.0),
            point: Point::new(100.0, 106.0),
            accuracy: 1e-2,
            tolerance: 1.0,
    }

    test! {
        test_place_into_bunting
            path: BUNTING,
            min: Point::ZERO,
            size: Size::new(28.0, 19.0),
            point: Point::new(15.5, 19.0),
            accuracy: 1e-2,
            tolerance: 1.0,
    }

    test! {
        test_place_into_bird
            path: BIRD,
            min: Point::ZERO,
            size: Size::new(26.0, 39.0),
            point: Point::new(32.0, 20.0),
            accuracy: 1e-2,
            tolerance: 1.0,
    }

    test! {
        test_place_into_hand
            path: HAND,
            min: Point::ZERO,
            size: Size::new(31.0, 42.0),
            point: Point::new(21.5, 20.0),
            accuracy: 1e-2,
            tolerance: 1.0,
    }

    test! {
        test_place_into_arrow
            path: ARROW,
            min: Point::ZERO,
            size: Size::new(30.0, 15.0),
            point: Point::new(42.0, 39.0),
            accuracy: 1e-2,
            tolerance: 1.0,
    }

    test! {
        test_place_into_iceberg
            path: ICEBERG,
            min: Point::ZERO,
            size: Size::new(53.0, 24.0),
            point: Point::new(43.0, 58.0),
            accuracy: 1e-2,
            tolerance: 1.0,
    }

    test! {
        test_place_into_canyon
            path: CANYON,
            min: Point::ZERO,
            size: Size::new(53.0, 44.0),
            point: Point::new(31.0, 20.0),
            accuracy: 1e-2,
            tolerance: 1.0,
    }
}
