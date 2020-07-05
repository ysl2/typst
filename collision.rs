//! Collisionless placement of objects.

use arrayvec::ArrayVec;
use std::cmp::Ordering;
use std::ops::Range;
use super::{
    find_intersections, value_no_nans, value_approx, Affine, ApproxEq,
    BezPath, Monotone, PathSeg, PathSegExt, ParamCurve, ParamCurveExtrema,
    ParamCurveSolve, Point, Rect, Size,
};

/// A data structure for fast, collisionless placement of objects into a group
/// of bezier shapes.
#[derive(Debug, Clone)]
pub struct PlacementGroup {
    /// The rows containing subslice range of its slots.
    rows: Vec<Row>,
    /// The slots row-by-row.
    slots: Vec<Slot>,
}

/// A top- and bot-bounded row of slots.
#[derive(Debug, Clone)]
struct Row {
    /// The y-coordinate of the top end of the segment.
    top: f64,
    /// The y-coordinate of the bottom end of the segment.
    bot: f64,
    /// Which slots belong to this row.
    idxs: Range<usize>,
}

/// A slot defined by a left and right border.
#[derive(Debug, Clone)]
struct Slot {
    /// The left border of the slot.
    left: Monotone<PathSeg>,
    /// The right border of the slot.
    right: Monotone<PathSeg>,
}

impl PlacementGroup {
    /// Create a new placement group from a path.
    ///
    /// The tolerance is used to determine whether two `y` coordinates can be
    /// considered equal or whether a row has to be created between them.
    pub fn new(path: &BezPath, tolerance: f64) -> PlacementGroup {
        let mut rows = vec![];
        let mut slots = vec![];

        // TODO: Multiple paths, inside & outside.
        // TODO: Also split at intersections.

        let (monotonics, splits) = split_monotonics(path, tolerance);
        let border_rows = split_into_rows(&monotonics, &splits, tolerance);

        for mut borders in border_rows {
            borders.sort_by(|a, b| value_no_nans(
                &a.start().midpoint(a.end()).x,
                &b.start().midpoint(b.end()).x,
            ));

            let start = slots.len();
            let top = borders[0].start().y;
            let bot = borders[0].end().y;

            for c in borders.chunks_exact(2) {
                slots.push(Slot { left: c[0], right: c[1] });
            }

            rows.push(Row {
                top,
                bot,
                idxs: start .. slots.len(),
            });
        }

        PlacementGroup { rows, slots }
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

        // Walk over the top rows where the top edge of the object lies in. The
        // first candidate row is determined by the min-points `y`-coordinate.
        for (t, tr) in self.rows.iter().enumerate().skip(s) {
            let min_top = tr.top.max(min.y);
            let max_bot = tr.bot;
            assert!(min_top <= max_bot);

            // Walk over the bottom rows where an object starting in `t` can
            // end in.
            for (b, br) in self.rows.iter().enumerate().skip(t) {
                // Too far to the top - is a middle row.
                if min_top + size.height > br.bot {
                    continue;
                }

                // Too far to the bottom.
                if max_bot + size.height < br.top {
                    break;
                }

                let mut best: Option<Point> = None;

                // Walk through the horizontal ranges where an object that
                // starts in `t` and ends in `b` can be placed (these depend
                // also on the rows in between `t` and `b`).
                for (r, f, l) in self.ranges(t, b) {
                    // Try to place the object in the range `r`, starting in `f`
                    // and ending in `l`.
                    let point = self.try_place_into(r, f, l, size, accuracy);

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

    /// Returns all ranges and corresponding top & bottom slots where objects
    /// can be placed with their top edge in `t` and their bottom edge in `b`.
    fn ranges(
        &self,
        t: usize,
        b: usize,
    ) -> impl Iterator<Item=(Range<f64>, &Slot, &Slot)> {
        assert!(t <= b);

        let mut ts = self.slots(t);
        let mut bs = self.slots(b);
        let mut ms: Vec<_> = (t + 1 .. b)
            .map(|m| self.slots(m))
            .collect();

        // Compute the subranges where there is a slot for all rows - which is
        // basically the intersection between the row's slots.
        let mut done = false;
        std::iter::from_fn(move || {
            while !done {
                let mut start = f64::NEG_INFINITY;
                let mut end = f64::INFINITY;
                let mut min = None;

                let mut check = |r: Range<f64>, v| {
                    start = start.max(r.start);
                    if r.end < end {
                        min = Some(v);
                        end = r.end;
                    }
                };

                let (f, l) = (&ts[0], &bs[0]);
                check(f.outer(), &mut ts);
                check(l.outer(), &mut bs);

                for m in &mut ms {
                    check(m[0].inner(), m);
                }

                let min = min.unwrap();
                *min = &min[1..];
                done = min.is_empty();

                if start < end {
                    return Some((start .. end, f, l));
                }
            }

            None
        })
    }

    /// Try to place the object into the given range, starting in slot `f` and
    /// ending in slot `l`.
    fn try_place_into(
        &self,
        range: Range<f64>,
        first: &Slot,
        last: &Slot,
        size: Size,
        accuracy: f64,
    ) -> Option<Point> {
        todo!("try_place_into")
    }

    /// Returns all slots contained in row `i`.
    fn slots(&self, i: usize) -> &[Slot] {
        &self.slots[self.rows[i].idxs.clone()]
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
            Monotone(seg.reverse())
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

impl Slot {
    /// The slot's top end.
    fn top(&self) -> f64 {
        self.left.start().y
    }

    /// The slot's bottom end.
    fn bot(&self) -> f64 {
        self.left.end().y
    }

    /// The horizontal range which surrounds the borders.
    fn outer(&self) -> Range<f64> {
        self.left_min() .. self.right_max()
    }

    /// The horizontal range which is surrounded by the borders.
    fn inner(&self) -> Range<f64> {
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

    /// Whether the left border is monotonously widening the slot.
    fn left_widening(&self) -> bool {
        self.left.start().x >= self.left.end().x
    }

    /// Whether the right border is monotonously widening the slot.
    fn right_widening(&self) -> bool {
        self.right.start().x <= self.right.end().x
    }
}

#[cfg(test)]
mod tests {
    use super::super::{BezPath, Rect};
    use super::*;

    fn _boxed(point: Point, size: Size) -> Rect {
        Rect::from_points(point, point + size.to_vec2())
    }

    fn shape(path: &str) -> BezPath {
        BezPath::from_svg(path).unwrap()
    }

    fn skewed_vase_shape() -> BezPath {
        shape("
            M65 100C23.5 65 59 48 16 20H52.5C90.6055 29.0694 113 66.4999 113
            100H65Z
        ")
    }

    fn hat_shape() -> BezPath {
        shape("M65.5 27.5H21.5L29 64.5L15.5 104.5H98L80 64.5L65.5 27.5Z")
    }

    fn border_group(shape: &BezPath) -> PlacementGroup {
        let curves: Vec<_> = shape.segments().collect();
        let left = Monotone(curves[0].reverse());
        let right = Monotone(curves[2]);
        PlacementGroup {
            rows: vec![Row {
                top: left.start().y,
                bot: left.end().y,
                idxs: 0 .. 1,
            }],
            slots: vec![Slot { left, right }],
        }
    }

    fn hat_group() -> PlacementGroup {
        let shape = hat_shape();
        let curves: Vec<_> = shape.segments().collect();

        let left1  = Monotone(curves[1]);
        let right1 = Monotone(curves[5].reverse());
        let left2  = Monotone(curves[2]);
        let right2 = Monotone(curves[4].reverse());

        PlacementGroup {
            rows: vec![
                Row {
                    top: left1.start().y,
                    bot: left1.end().y,
                    idxs: 0 .. 1,
                },
                Row {
                    top: left2.start().y,
                    bot: left2.end().y,
                    idxs: 1 .. 2,
                },
            ],
            slots: vec![
                Slot { left: left1, right: right1 },
                Slot { left: left2, right: right2 },
            ],
        }
    }

    #[test]
    fn test_build_skewed_vase_group() {
        let shape = skewed_vase_shape();
        let group = PlacementGroup::new(&shape, 1e-2);
        assert_eq!(group.rows.len(), 1);
        assert_eq!(group.slots.len(), 1);
    }

    #[test]
    fn test_build_banner_group() {
        let shape = shape("
            M29.0452 86.5001C27.5159 93.9653 26.1564 102.373 25 111.793L13
            19H106.5L100.5 111.793C99.5083 103.022 97.8405 94.485 95.65
            86.5C81.4874 34.8747 45.4731 6.3054 29.0452 86.5001Z
        ");
        let group = PlacementGroup::new(&shape, 1e-2);
        assert_eq!(group.rows.len(), 3);
        assert_eq!(group.slots.len(), 5);
    }

    #[test]
    fn test_build_strange_tower_group() {
        let shape = shape("
            M72 26H28C28 26 36.2035 48.2735 35.5 63C34.7133 79.4679 22 103 22
            103H49.5V63L74.5 81.5V103H104.5C104.5 103 91.2926 90.5292 80.5
            64.5C72 44 72 26 72 26Z
        ");
        let group = PlacementGroup::new(&shape, 1e-2);
        assert_eq!(group.rows.len(), 5);
        assert_eq!(group.slots.len(), 8);
    }

    #[test]
    fn test_place_into_trapez() {
        let shape = shape("M20 100L40 20H80L100 100H20Z");
        let group = border_group(&shape);

        assert_approx_eq!(
            group.place(Point::ZERO, Size::new(50.0, 15.0), 1e-2),
            Some(Point::new(35.0, 40.0)),
            tolerance = 1e-2,
        );
    }

    #[test]
    fn test_place_into_silo() {
        let shape = shape("
            M20 100C20 100 28 32 40 20C52 8.00005 66 8.5 80 20C94 31.5 100 100
            100 100H20Z
        ");
        let group = border_group(&shape);
        assert_approx_eq!(
            group.place(Point::ZERO, Size::new(70.0, 30.0), 1e-2),
            Some(Point::new(25.5, 65.0)),
            tolerance = 0.5,
        );
    }

    #[test]
    fn test_place_into_tailplane() {
        let shape = shape("M38 100L16 20H52.5L113 100H38Z");
        let group = border_group(&shape);
        assert_approx_eq!(
            group.place(Point::ZERO, Size::new(40.0, 30.0), 1e-2),
            Some(Point::new(31.0, 45.0)),
            tolerance = 1.0,
        );
    }

    #[test]
    fn test_place_into_skewed_vase() {
        let shape = skewed_vase_shape();
        let group = PlacementGroup::new(&shape, 1e-2);
        assert_approx_eq!(
            group.place(Point::ZERO, Size::new(50.0, 17.0), 1e-2),
            Some(Point::new(41.5, 44.0)),
            tolerance = 0.25,
        );
    }

    #[test]
    fn test_place_into_weird_placement() {
        let shape = shape("
            M65 26L45 26C45 26 52.3727 60.5 25 81.2597C5.38123 96.1388 22 141
            22 141H63V81.2597L100.273 108.89V141H158.5C158.5 141 164.282 85.5
            105 82.5C82.0353 81.3379 65 26 65 26Z
        ");
        let group = PlacementGroup::new(&shape, 1e-2);
        let pos = group.place(Point::new(0.0, 60.0), Size::new(46.0, 17.0), 1e-2)
            .unwrap();

        render!(shape);
        render!(_boxed(pos, Size::new(46.0, 17.0)), color="blue");
        render!(pos, color="green");
        // save!("_things/collision/amazed.png");
        // show!();
    }

    #[test]
    fn test_place_into_top_of_hat() {
        assert_approx_eq!(
            hat_group().place(Point::ZERO, Size::new(35.0, 30.0), 1e-2),
            Some(Point::new(28.0, 28.0)),
            tolerance = 1.0,
        );
    }

    #[test]
    fn test_place_into_mid_of_hat() {


        render!(hat_shape());
        let pos= hat_group().place(Point::ZERO, Size::new(43.0, 30.0), 1e-2).unwrap();
        render!(_boxed(pos, Size::new(43.0, 30.0)), color="blue");
        render!(pos, color="green");
        show!();


        assert_approx_eq!(
            hat_group().place(Point::ZERO, Size::new(43.0, 30.0), 1e-2),
            Some(Point::new(29.0, 44.0)),
            tolerance = 0.1,
        );
    }

    #[test]
    fn test_place_into_bot_of_hat() {
        assert_approx_eq!(
            hat_group().place(Point::ZERO, Size::new(65.0, 12.0), 1e-2),
            Some(Point::new(23.0, 83.0)),
            tolerance = 1.0,
        );
    }
}
