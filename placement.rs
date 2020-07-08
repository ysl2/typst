//! Collisionless placement of objects.

use arrayvec::ArrayVec;
use std::cmp::Ordering;
use super::{
    value_no_nans, value_approx, ApproxEq, BezPath, Monotone, PathSeg,
    ParamCurve, ParamCurveExtrema, ParamCurveSolve, Point, Range, Rect, Size,
    TranslateScale, Vec2,
};

/// A data structure for fast, collisionless placement of objects into a group
/// of bezier shapes.
#[derive(Debug, Clone)]
pub struct PlacementGroup {
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

impl PlacementGroup {
    /// Create a new placement group from a path.
    ///
    /// The tolerance is used to determine whether two `y` coordinates can be
    /// considered equal or whether a row has to be created between them.
    pub fn new(path: &BezPath, tolerance: f64) -> PlacementGroup {
        let mut rows = vec![];
        let mut regions = vec![];

        // TODO: Multiple paths, inside & outside.
        // TODO: Also split at intersections.

        let (monotonics, splits) = split_monotonics(path, tolerance);
        let border_rows = split_into_rows(&monotonics, &splits, tolerance);

        for mut borders in border_rows {
            borders.sort_by(|a, b| value_no_nans(
                &a.start().midpoint(a.end()).x,
                &b.start().midpoint(b.end()).x,
            ));

            let start = regions.len();
            let top = borders[0].start().y;
            let bot = borders[0].end().y;

            for c in borders.chunks_exact(2) {
                regions.push(Region { left: c[0], right: c[1] });
            }

            rows.push(Row {
                top,
                bot,
                idxs: start .. regions.len(),
            });
        }

        PlacementGroup { rows, regions }
    }

    /// Find the top-and-left-most position in the group to place an object with
    /// the given `size`.
    ///
    /// Specifically, the following guarantees are made:
    /// - When an object with the given `size` is placed such that its top-left
    ///   corner coincides with the point, it does not collide with any shape in
    ///   this group.
    /// - The returned point `p` lies to the right and bottom of `min` (`p.x >=
    ///   min.x` and `p.y >= min.y`).
    /// - There exists no point further to the left or to the top for which the
    ///   previous two guarantees are fulfilled.
    pub fn place(
        &self,
        min: Point,
        size: Size,
        accuracy: f64,
    ) -> Option<Point> {
        let s = self.find_first_row(min.y)?;

        // Walk over the rows where the top edge of the object can fall into.
        // The first candidate row is determined by the min-point's
        // `y`-coordinate.
        for (i, top_row) in self.rows.iter().enumerate().skip(s) {
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
                for (range, top_region, bot_region) in self.ranges(i, j, min.x) {
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

    /// Find the first row which contains the `y` coordinate or is below it.
    fn find_first_row(&self, y: f64) -> Option<usize> {
        match self.rows.binary_search_by(|row| {
            if row.top > y {
                Ordering::Greater
            } else if row.bot <= y {
                Ordering::Less
            } else {
                Ordering::Equal
            }
        }) {
            Ok(i) => Some(i),
            Err(i) if i < self.rows.len() => Some(i),
            _ => None,
        }
    }

    /// Returns all ranges and the top & bottom region they fall into,
    /// respectively, for top row `i` and bottom row `j`.
    fn ranges(
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

    /// Returns all regions contained in row `i`.
    fn regions(&self, i: usize) -> &[Region] {
        &self.regions[self.rows[i].idxs.clone()]
    }
}

/// Split the path into monotonic subsegments and return them and alongside all
/// y-coordinates at which subsegments start and end.
fn split_monotonics(
    path: &BezPath,
    tolerance: f64,
) -> (Vec<Monotone<PathSeg>>, Vec<f64>) {
    let mut monotonics = vec![];
    let mut splits = vec![];

    // Split curves into monotonic subsegments.
    for seg in path.segments() {
        splits.push(seg.start().y);

        for r in seg.extrema_ranges() {
            splits.push(seg.eval(r.end).y);
            monotonics.push(Monotone(seg.subsegment(r)));
        }
    }

    // Make the splits `y`-unique.
    splits.sort_by(value_no_nans);
    splits.dedup_by(|a, b| a.approx_eq(&b, tolerance));

    (monotonics, splits)
}

/// Split monotonics segments into rows of subsegments such that no segment
/// crosses a vertical split.
fn split_into_rows(
    monotonics: &[Monotone<PathSeg>],
    splits: &[f64],
    tolerance: f64,
) -> Vec<Vec<Monotone<PathSeg>>> {
    let len = splits.len();
    let mut rows = vec![vec![]; if len > 0 { len - 1 } else { 0 }];

    // Split curves at y values.
    for &seg in monotonics {
        let seg = if seg.start().y > seg.end().y {
            seg.reverse()
        } else {
            seg
        };

        let top = seg.start().y;
        let bot = seg.end().y;

        let find_k_for_y = |y| {
            splits.binary_search_by(|v| value_approx(&v, &y, tolerance))
                .expect("splits does not contain y")
        };

        // Find start and end values in split list.
        let k0 = find_k_for_y(top);
        let k1 = find_k_for_y(bot);
        assert!(k0 <= k1);

        match k1 - k0 {
            // The segment is horizontal and thus uninteresting.
            0 => {}

            // The segment does not need to be subdivided.
            1 => rows[k0].push(seg),

            // The segment has to be subdivided.
            _ => {
                let mut t_start = 0.0;
                for ki in k0 + 1 .. k1 {
                    let t = match seg.solve_t_for_y(splits[ki]).as_slice() {
                        &[t] => t,
                        _ => panic!("curve is not monotonic"),
                    };

                    rows[ki - 1].push(seg.subsegment(t_start .. t));
                    t_start = t;
                }

                rows[k1 - 1].push(seg.subsegment(t_start .. 1.0));
            }
        }
    }

    // Delete empty rows.
    rows.retain(|r| !r.is_empty());

    rows
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

#[cfg(test)]
mod tests {
    use super::super::{BezPath, Rect};
    use super::*;

    fn _boxed(point: Point, size: Size) -> Rect {
        Rect::from_points(point, point + size.to_vec2())
    }

    fn svg(path: &str) -> BezPath {
        BezPath::from_svg(path).unwrap()
    }

    fn hat() -> BezPath {
        svg("M65.5 27.5H21.5L29 64.5L15.5 104.5H98L80 64.5L65.5 27.5Z")
    }

    fn skewed_vase() -> BezPath {
        svg("M65 100C23.5 65 59 48 16 20H52.5C90.6 29.07 113 66.5 113 100H65Z")
    }

    fn weird_high_heel() -> BezPath {
        svg("
            M65 26L45 26C45 26 52.3727 60.5 25 81.2597C5.38123 96.1388 22 141
            22 141H63V81.2597L100.273 108.89V141H158.5C158.5 141 164.282 85.5
            105 82.5C82.0353 81.3379 65 26 65 26Z
        ")
    }

    #[test]
    fn test_build_skewed_vase_group() {
        let shape = skewed_vase();
        let group = PlacementGroup::new(&shape, 1e-2);
        assert_eq!(group.rows.len(), 1);
        assert_eq!(group.regions.len(), 1);
    }

    #[test]
    fn test_build_banner_group() {
        let shape = svg("
            M29.0452 86.5001C27.5159 93.9653 26.1564 102.373 25 111.793L13
            19H106.5L100.5 111.793C99.5083 103.022 97.8405 94.485 95.65
            86.5C81.4874 34.8747 45.4731 6.3054 29.0452 86.5001Z
        ");
        let group = PlacementGroup::new(&shape, 1e-2);
        assert_eq!(group.rows.len(), 3);
        assert_eq!(group.regions.len(), 5);
    }

    #[test]
    fn test_build_strange_tower_group() {
        let shape = svg("
            M72 26H28C28 26 36.2035 48.2735 35.5 63C34.7133 79.4679 22 103 22
            103H49.5V63L74.5 81.5V103H104.5C104.5 103 91.2926 90.5292 80.5
            64.5C72 44 72 26 72 26Z
        ");
        let group = PlacementGroup::new(&shape, 1e-2);
        assert_eq!(group.rows.len(), 5);
        assert_eq!(group.regions.len(), 8);
    }

    #[test]
    fn test_place_into_trapez() {
        let shape = svg("M20 100L40 20H80L100 100H20Z");
        let group = PlacementGroup::new(&shape, 1e-2);
        assert_approx_eq!(
            group.place(Point::ZERO, Size::new(50.0, 15.0), 1e-2),
            Some(Point::new(35.0, 40.0)),
            tolerance = 1e-2,
        );
    }

    #[test]
    fn test_place_into_trapez_with_min_x() {
        let shape = svg("M20 100L40 20H80L100 100H20Z");
        let group = PlacementGroup::new(&shape, 1e-2);
        assert_approx_eq!(
            group.place(Point::new(60.0, 30.0), Size::new(25.0, 10.0), 1e-2),
            Some(Point::new(60.0, 40.0)),
            tolerance = 1e-2,
        );
    }

    #[test]
    fn test_place_into_trapez_with_min_y() {
        let shape = svg("M20 100L40 20H80L100 100H20Z");
        let group = PlacementGroup::new(&shape, 1e-2);
        assert_approx_eq!(
            group.place(Point::new(30.0, 56.0), Size::new(30.0, 10.0), 1e-2),
            Some(Point::new(31.0, 56.0)),
            tolerance = 1e-2,
        );
    }

    #[test]
    fn test_place_into_silo() {
        let shape = svg("
            M20 100C20 100 28 32 40 20C52 8.00005 66 8.5 80 20C94 31.5 100 100
            100 100H20Z
        ");
        let group = PlacementGroup::new(&shape, 1e-2);
        assert_approx_eq!(
            group.place(Point::ZERO, Size::new(70.0, 30.0), 1e-2),
            Some(Point::new(25.5, 65.0)),
            tolerance = 0.5,
        );
    }

    #[test]
    fn test_place_into_tailplane() {
        let shape = svg("M38 100L16 20H52.5L113 100H38Z");
        let group = PlacementGroup::new(&shape, 1e-2);
        assert_approx_eq!(
            group.place(Point::ZERO, Size::new(40.0, 30.0), 1e-2),
            Some(Point::new(31.0, 45.0)),
            tolerance = 1.0,
        );
    }

    #[test]
    fn test_place_into_top_of_hat() {
        let group = PlacementGroup::new(&hat(), 1e-2);
        assert_approx_eq!(
            group.place(Point::ZERO, Size::new(35.0, 30.0), 1e-2),
            Some(Point::new(28.0, 28.0)),
            tolerance = 1.0,
        );
    }

    #[test]
    fn test_place_into_mid_of_hat() {
        let group = PlacementGroup::new(&hat(), 1e-2);
        assert_approx_eq!(
            group.place(Point::ZERO, Size::new(43.0, 30.0), 1e-2),
            Some(Point::new(29.0, 44.0)),
            tolerance = 0.1,
        );
    }

    #[test]
    fn test_place_into_bot_of_hat() {
        let group = PlacementGroup::new(&hat(), 1e-2);
        assert_approx_eq!(
            group.place(Point::ZERO, Size::new(65.0, 12.0), 1e-2),
            Some(Point::new(23.0, 83.0)),
            tolerance = 1.0,
        );
    }

    #[test]
    fn test_place_into_skewed_vase() {
        let group = PlacementGroup::new(&skewed_vase(), 1e-2);
        assert_approx_eq!(
            group.place(Point::ZERO, Size::new(50.0, 17.0), 1e-2),
            Some(Point::new(41.5, 44.0)),
            tolerance = 0.25,
        );
    }

    #[test]
    fn test_place_into_top_of_weird_high_heel() {
        let group = PlacementGroup::new(&weird_high_heel(), 1e-2);
        assert_approx_eq!(
            group.place(Point::ZERO, Size::new(32.0, 12.0), 1e-2),
            Some(Point::new(44.0, 52.0)),
            tolerance = 1.0,
        );
    }

    #[test]
    fn test_place_into_left_of_weird_high_heel() {
        let group = PlacementGroup::new(&weird_high_heel(), 1e-2);
        assert_approx_eq!(
            group.place(Point::new(0.0, 60.0), Size::new(46.0, 17.0), 1e-2),
            Some(Point::new(17.0, 94.0)),
            tolerance = 0.5,
        );
    }

    #[test]
    fn test_place_into_right_of_weird_high_heel() {
        let group = PlacementGroup::new(&weird_high_heel(), 1e-2);
        assert_approx_eq!(
            group.place(Point::ZERO, Size::new(50.0, 17.0), 1e-2),
            Some(Point::new(100.0, 106.0)),
            tolerance = 1.0,
        );
    }
}
