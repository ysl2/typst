//! Collisionless placement of objects.

use super::{
    max, min, value_no_nans, value_approx, ApproxEq, BezPath, CubicBez, Dim,
    Point, PathSeg, ParamCurve, ParamCurveExtrema, ParamCurveSolve,
};

/// A data structure for fast, collisionless placement of objects into a group
/// of bezier shapes.
#[derive(Debug, Clone)]
pub struct BezColliderGroup {
    /// The segment in the order they should be tried in.
    segments: Vec<BezColliderSegment>,
}

/// A width-monotonic vertical segment defined by a left and right border.
#[derive(Debug, Clone)]
struct BezColliderSegment {
    /// The left border of the segment.
    /// (Start point is at the top, end point at the bottom)
    left: CubicBez,
    /// The right border of the segment.
    /// (Start point is at the top, end point at the bottom)
    right: CubicBez,
}

impl BezColliderGroup {
    /// Create a new empty collider group.
    pub fn new(path: &BezPath, tolerance: f64) -> BezColliderGroup {
        // TODO: Also split at intersections.
        let (monotonics, splits) = split_monotonics(path, tolerance);

        let mut rows = split_rows(&monotonics, &splits, tolerance);
        for row in rows.iter_mut() {
            // Does not matter whether we use start x or end x because the
            // segments should not have intersections at this point.
            row.sort_by(|a, b| value_no_nans(&a.start().x, &b.start().x));
        }

        todo!("collider group new")
    }

    /// Finds the top-most position in the group to place an object with
    /// dimensions `dim`. Returns the origin point for the object.
    ///
    /// The point is selected such that:
    /// - An object with dimensions `dim` fits at that baseline anchor point
    ///   without colliding with any of the shapes in the group.
    /// - The whole object is placed below `top`
    ///   (that is `point.y - dim.height >= top`).
    pub fn place(&self, dim: Dim, _top: f64, tolerance: f64) -> Option<Point> {
        search_place(&self.segments, dim, tolerance)
    }
}

/// Split the path into monotonic subsegments and return alongside all
/// y-coordinates at which curves start, end or were splitted.
fn split_monotonics(path: &BezPath, tolerance: f64) -> (Vec<PathSeg>, Vec<f64>) {
    let mut monotonics = vec![];
    let mut splits = vec![];

    // Split curves into monotonic subsegments.
    for seg in path.segments() {
        splits.push(seg.start().y);

        let mut t_start = 0.0;
        for t in seg.extrema() {
            monotonics.push(seg.subsegment(t_start .. t));
            splits.push(seg.eval(t).y);
            t_start = t;
        }

        monotonics.push(seg.subsegment(t_start .. 1.0));
        splits.push(seg.end().y);
    }

    // Make the splits `y`-unique.
    splits.sort_by(value_no_nans);
    splits.dedup_by(|a, b| a.approx_eq(&b, tolerance));

    (monotonics, splits)
}

/// Split monotonics segments into rows of subsegments such that no segment
/// crosses a vertical split.
fn split_rows(
    monotonics: &[PathSeg],
    splits: &[f64],
    tolerance: f64
) -> Vec<Vec<PathSeg>> {
    let mut rows = vec![vec![]; splits.len() - 1];

    // Split curves at y values.
    for &seg in monotonics {
        let seg = if seg.start().y < seg.end().y { seg } else { seg.reverse() };
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

    rows
}

/// Search for the top-most position in the first possible segment.
fn search_place(
    segments: &[BezColliderSegment],
    dim: Dim,
    tolerance: f64,
) -> Option<Point> {
    for (f, first) in segments.iter().enumerate() {
        let mut top = first.top() + dim.height;
        let first_max_bot = first.bot() + dim.height;

        for (l, last) in segments.iter().enumerate().skip(f) {
            // The real top and bottom ends of the search interval for the
            // object's origin point are inset by the height and depth of
            // the object - lower or higher would make the object stick out.
            let last_max_bot  = last.bot() - dim.depth;
            let bot = min(first_max_bot, last_max_bot);

            // If the object is higher than the available space, it cannot
            // fit.
            if top > bot {
                println!("info: skipping last segment");
                continue;
            }

            let segments = &segments[f ..= l];
            let found = search_bisect(dim, top, bot, segments, tolerance);
            if found.is_some() {
                return found;
            }

            // If the first segment is exhausted, we can leave the inner
            // loop.
            if last_max_bot > first_max_bot {
                println!("info: leaving first segment");
                break;
            }

            top = bot;
        }
    }

    None
}

/// Search for a vertical position to place an object with dimensions `dim`
/// at origin positions between `top` and `bot`.
///
/// At least one segment must be given.
///
/// The top end of the object must fall into the first segment and the bot
/// end into last segment for all values between `top` and `bot`. As a
/// consequence, all inner segments are fully filled by the object,
/// vertically.
fn search_bisect(
    dim: Dim,
    mut top: f64,
    mut bot: f64,
    segments: &[BezColliderSegment],
    tolerance: f64,
) -> Option<Point> {
    const MAX_ITERS: usize = 20;

    let len   = segments.len();
    let first = &segments[0];
    let mid   = &segments[1 .. (len - 1).max(1)];
    let last  = &segments[len - 1];

    assert!(top + dim.depth >= last.top(), "does not end in last segment");
    assert!(bot - dim.height <= first.bot(), "does not start in first segment");

    // The offset from the origin point to the corner (top or bottom) at which
    // the curve is tighter. The bool `widening` should be true when the curve
    // is widening the segment from with growing y-value.
    let tightest_offset = |widening: bool| -> f64 {
        if widening { -dim.height } else { dim.depth }
    };

    let left_first_offset  = tightest_offset(first.left_widening());
    let left_last_offset   = tightest_offset(last.left_widening());
    let right_first_offset = tightest_offset(first.right_widening());
    let right_last_offset  = tightest_offset(last.right_widening());

    let left_mid_x = mid
        .iter()
        .map(|seg| max(seg.left.p0.x, seg.left.p3.x))
        .max_by(value_no_nans)
        .unwrap_or(f64::NEG_INFINITY);

    let right_mid_x = mid
        .iter()
        .map(|seg| min(seg.right.p0.x, seg.right.p3.x))
        .min_by(value_no_nans)
        .unwrap_or(f64::INFINITY);

    // Left x, right x and width if the object's origin is placed at `y`.
    let lrxw_at_y = |y: f64| {
        // TODO: Don't compute twice if first == last.
        let left_first_x = find_one_x(first.left, y + left_first_offset);
        let left_last_x  = find_one_x(last.left,  y + left_last_offset);
        let left_x = left_first_x.max(left_mid_x).max(left_last_x);

        let right_first_x = find_one_x(first.right, y + right_first_offset);
        let right_last_x  = find_one_x(last.right,  y + right_last_offset);
        let right_x = right_first_x.min(right_mid_x).min(right_last_x);

        let width = right_x - left_x;
        (left_x, right_x, width)
    };

    let (top_left_x, _, mut top_width) = lrxw_at_y(top);
    let (_,          _, mut bot_width) = lrxw_at_y(bot);

    // If it already fits at the top, we're good.
    if dim.width <= top_width {
        println!("info: fits at the top");
        return Some(Point::new(top_left_x, top));
    }

    // If it does not fit at the top and also not at the bottom, it won't
    // fit at all, since the width function is monotonous.
    if dim.width > bot_width {
        println!("info: object is too wide");
        return None;
    }

    let mut iter = 1;
    loop {
        // Determine the next `y` value by linear interpolation between the
        // min and max bounds.
        let ratio = (dim.width - top_width) / (bot_width - top_width);
        let y = top + ratio * (bot - top);
        let (left_x, _, width) = lrxw_at_y(y);

        // Check whether we converged to a good spot.
        if width.approx_eq(&dim.width, tolerance) {
            println!("info: converged in {}. iteration", iter);
            return Some(Point::new(left_x, y));
        }

        // Adjust the bounds by replacing the bad bound with the better
        // estimate.
        if dim.width < width {
            bot = y;
            bot_width = width;
        } else {
            top = y;
            top_width = width;
        }

        if iter > MAX_ITERS {
            println!("warning: bisection search did not converge");
            return None;
        }

        iter += 1;
    }
}

/// Tries to to find an `x` position at which the given `curve` has the given
/// `y` value. The `y` value is clamped into the valid y-range for the curve.
///
/// The curve must be monotonic and the min-max rectangle defined by start and
/// end point must be a bounding box for the curve.
fn find_one_x(curve: CubicBez, y: f64) -> f64 {
    const EPS: f64 = 1e-4;

    if y < curve.p0.y + EPS {
        return curve.p0.x;
    } else if y > curve.p3.y - EPS {
        return curve.p3.x;
    }

    match curve.solve_x_for_y(y).as_slice() {
        &[] => panic!("there should be at least one root"),
        &[x] => x,
        xs => panic!("curve is not monotone and has multiple roots: {:?}", xs),
    }
}

impl BezColliderSegment {
    /// The top end of this segment.
    fn top(&self) -> f64 {
        self.left.p0.y
    }

    /// The bottom end of this segment.
    fn bot(&self) -> f64 {
        self.left.p3.y
    }

    /// Whether the left border is monotonously widening the segment.
    fn left_widening(&self) -> bool {
        self.left.p0.x >= self.left.p3.x
    }

    /// Whether the right border is monotonously widening the segment.
    fn right_widening(&self) -> bool {
        self.right.p0.x <= self.right.p3.x
    }
}

impl_approx_eq!(BezColliderGroup [segments]);
impl_approx_eq!(BezColliderSegment [left, right]);

#[cfg(test)]
mod tests {
    use super::super::{BezPath, Rect, Vec2};
    use super::*;

    #[allow(unused)]
    fn dim_rect(point: Point, dim: Dim) -> Rect {
        Rect::from_points(
            point - Vec2::new(0.0, dim.height),
            point + Vec2::new(dim.width, dim.depth),
        )
    }

    fn shape(path: &str) -> BezPath {
        BezPath::from_svg(path).unwrap()
    }

    fn left_right_collider(shape: &BezPath) -> BezColliderGroup {
        let curves: Vec<_> = shape.segments().collect();
        BezColliderGroup {
            segments: vec![
                BezColliderSegment {
                    left: curves[0].reverse().to_cubic(),
                    right: curves[2].to_cubic(),
                },
            ],
        }
    }

    #[test]
    fn test_place_into_trapez() {
        let shape = shape("M20 100L40 20H80L100 100H20Z");

        let dim = Dim::new(50.0, 10.0, 5.0);
        let correct = Point::new(35.0, 40.0 + dim.height);

        let found = left_right_collider(&shape).place(dim, 25.0, 1e-2);
        assert_approx_eq!(found, Some(correct));
    }

    #[test]
    fn test_place_into_silo() {
        let shape = shape("
            M20 100C20 100 28 32 40 20C52 8.00005 66 8.5 80 20C94 31.5 100 100
            100 100H20Z
        ");

        let dim = Dim::new(70.0, 10.0, 20.0);
        let approx_correct = Point::new(25.0, 66.0 + dim.height);

        let found = left_right_collider(&shape).place(dim, 0.0, 1e-2);
        assert_approx_eq!(found, Some(approx_correct), tolerance = 1.0);
    }

    #[test]
    fn test_place_into_tailplane() {
        let shape = shape("M38 100L16 20H52.5L113 100H38Z");

        let dim = Dim::new(40.0, 10.0, 20.0);
        let approx_correct = Point::new(31.0, 75.0 - dim.depth);

        let found = left_right_collider(&shape).place(dim, 0.0, 1e-2);
        assert_approx_eq!(found, Some(approx_correct), tolerance = 1.0);
    }

    fn hat_shape() -> BezPath {
        shape("M65.5 27.5H21.5L29 64.5L15.5 104.5H98L80 64.5L65.5 27.5Z")
    }

    fn hat_collider() -> BezColliderGroup {
        let shape = hat_shape();
        let curves: Vec<_> = shape.segments().collect();
        BezColliderGroup {
            segments: vec![
                BezColliderSegment {
                    left: curves[1].to_cubic(),
                    right: curves[5].reverse().to_cubic(),
                },
                BezColliderSegment {
                    left: curves[2].to_cubic(),
                    right: curves[4].reverse().to_cubic(),
                },
            ]
        }
    }

    #[test]
    fn test_place_into_top_of_hat() {
        let dim = Dim::new(35.0, 15.0, 15.0);
        let approx_correct = Point::new(28.0, 58.0 - dim.depth);

        let found = hat_collider().place(dim, 0.0, 1e-2);
        assert_approx_eq!(found, Some(approx_correct), tolerance = 1.0);
    }

    #[test]
    fn test_place_into_mid_of_hat() {
        let dim = Dim::new(43.0, 15.0, 15.0);
        let approx_correct = Point::new(29.0, 44.0 + dim.height);

        let found = hat_collider().place(dim, 0.0, 1e-2);
        assert_approx_eq!(found, Some(approx_correct), tolerance = 0.1);
    }

    #[test]
    fn test_place_into_bot_of_hat() {
        let dim = Dim::new(65.0, 10.0, 2.0);
        let approx_correct = Point::new(23.0, 83.0 + dim.height);

        let found = hat_collider().place(dim, 0.0, 1e-2);
        assert_approx_eq!(found, Some(approx_correct), tolerance = 1.0);
    }

    #[test]
    fn test_build_banner_collider() {
        let shape = shape("
            M29.0452 86.5001C27.5159 93.9653 26.1564 102.373 25 111.793L13
            19H106.5L100.5 111.793C99.5083 103.022 97.8405 94.485 95.65
            86.5C81.4874 34.8747 45.4731 6.3054 29.0452 86.5001Z
        ");

        let _collider = BezColliderGroup::new(&shape, 1e-2);
    }
}
