//! Collisionless placement of objects.

use super::{
    max, min, value_no_nans, ApproxEq, Bez, BezShape, Dim, Length, Point,
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
    left: Bez,
    /// The right border of the segment.
    /// (Start point is at the top, end point at the bottom)
    right: Bez,
}

impl BezColliderGroup {
    /// Create a new empty collider group.
    pub fn new(shape: &BezShape) -> BezColliderGroup {
        use super::flo::*;

        const TOLERANCE: f32 = 1e-2;

        let mut lines = vec![];

        for curve in shape.curves() {
            lines.push(curve.start.y);
            lines.push(curve.end.y);

            let ts = flo_curve(curve).find_extremities();
            for t in ts {
                lines.push(curve.point_for_t(t as f32).y)
            }
        }

        lines.sort_by(value_no_nans);
        lines.dedup_by(|a, b| a.approx_eq(&b, TOLERANCE));

        let mut segments = vec![];

        for chunk in lines.windows(2) {
            let top = chunk[0];
            let bot = chunk[1];

            let mut borders = vec![];

            for curve in shape.curves() {
                let top_hits = curve.solve_t_for_y(top);
                let bot_hits = curve.solve_t_for_y(bot);

                if top_hits.len() == 1 && bot_hits.len() == 1 {
                    let t_start = top_hits[0];
                    let t_end = bot_hits[0];

                    let t_min = t_start.min(t_end) as f64;
                    let t_max = t_start.max(t_end) as f64;

                    let mut section = unflo_bez(flo_curve(curve).section(t_min, t_max));
                    if section.start.y > section.end.y {
                        section = section.rev();
                    }

                    borders.push(section);
                }
            }

            borders.sort_by(|a, b| value_no_nans(&a.start.x, &b.start.x));

            if borders.len() == 2 {
                segments.push(BezColliderSegment {
                    left: borders[0],
                    right: borders[1],
                });
            }
        }

        BezColliderGroup { segments }
    }

    /// Finds the top-most position in the group to place an object with
    /// dimensions `dim`. Returns the origin point for the object.
    ///
    /// The point is selected such that:
    /// - An object with dimensions `dim` fits at that baseline anchor point
    ///   without colliding with any of the shapes in the group.
    /// - The whole object is placed below `top`
    ///   (that is `point.y - dim.height >= top`).
    pub fn place(&self, dim: Dim, _top: Length) -> Option<Point> {
        const TOLERANCE: f32 = 1e-2;

        for (f, first) in self.segments.iter().enumerate() {
            let mut top = first.top() + dim.height;
            let first_max_bot = first.bot() + dim.height;

            for (l, last) in self.segments.iter().enumerate().skip(f) {
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

                let segments = &self.segments[f ..= l];
                let found = search_bisect(dim, top, bot, segments, TOLERANCE);
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
}

impl BezColliderSegment {
    /// The top end of this segment.
    fn top(&self) -> Length {
        self.left.start.y
    }

    /// The bottom end of this segment.
    fn bot(&self) -> Length {
        self.left.end.y
    }

    /// Whether the left border is monotonously widening the segment.
    fn left_widening(&self) -> bool {
        self.left.start.x >= self.left.end.x
    }

    /// Whether the right border is monotonously widening the segment.
    fn right_widening(&self) -> bool {
        self.right.start.x <= self.right.end.x
    }
}

impl_approx_eq!(BezColliderGroup [segments]);
impl_approx_eq!(BezColliderSegment [left, right]);

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
    mut top: Length,
    mut bot: Length,
    segments: &[BezColliderSegment],
    tolerance: f32,
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
    let tightest_offset = |widening: bool| -> Length {
        if widening { -dim.height } else { dim.depth }
    };

    let left_first_offset  = tightest_offset(first.left_widening());
    let left_last_offset   = tightest_offset(last.left_widening());
    let right_first_offset = tightest_offset(first.right_widening());
    let right_last_offset  = tightest_offset(last.right_widening());

    let left_mid_x = mid
        .iter()
        .map(|seg| max(seg.left.start.x, seg.left.end.x))
        .max_by(value_no_nans)
        .unwrap_or(Length::NEG_INF);

    let right_mid_x = mid
        .iter()
        .map(|seg| min(seg.right.start.x, seg.right.end.x))
        .min_by(value_no_nans)
        .unwrap_or(Length::INF);

    // Left x, right x and width if the object's origin is placed at `y`.
    let lrxw_at_y = |y: Length| {
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
fn find_one_x(curve: Bez, y: Length) -> Length {
    const EPS: Length = Length::pt(1e-4);

    if y < curve.start.y + EPS {
        return curve.start.x;
    } else if y > curve.end.y - EPS {
        return curve.end.x;
    }

    match curve.solve_x_for_y(y).as_slice() {
        &[] => panic!("there should be at least one root"),
        &[x] => x,
        xs => panic!("curve is not monotone and has multiple roots: {:?}", xs),
    }
}

#[cfg(test)]
mod tests {
    use super::super::{BezShape, Rect, Vec2, pt};
    use super::*;

    #[allow(unused)]
    fn dim_rect(point: Point, dim: Dim) -> Rect {
        Rect::new(
            point - Vec2::with_y(dim.height),
            point + Vec2::new(dim.width, dim.depth),
        )
    }

    fn shape(path: &str) -> BezShape {
        BezShape::from_svg_path(path).unwrap()
    }

    fn left_right_collider(shape: &BezShape) -> BezColliderGroup {
        let curves: Vec<_> = shape.curves().collect();
        BezColliderGroup {
            segments: vec![
                BezColliderSegment { left: curves[0].rev(), right: curves[2] },
            ],
        }
    }

    #[test]
    fn test_place_into_trapez() {
        let shape = shape("M20 100L40 20H80L100 100H20Z");

        let dim = Dim::new(pt(50.0), pt(10.0), pt(5.0));
        let correct = Point::new(pt(35.0), pt(40.0) + dim.height);

        let found = left_right_collider(&shape).place(dim, pt(25.0));
        assert_approx_eq!(found, Some(correct));
    }

    #[test]
    fn test_place_into_silo() {
        let shape = shape("
            M20 100C20 100 28 32 40 20C52 8.00005 66 8.5 80 20C94 31.5 100 100
            100 100H20Z
        ");

        let dim = Dim::new(pt(70.0), pt(10.0), pt(20.0));
        let approx_correct = Point::new(pt(25.0), pt(66.0) + dim.height);

        let found = left_right_collider(&shape).place(dim, pt(0.0));
        assert_approx_eq!(found, Some(approx_correct), tolerance = 1.0);
    }

    #[test]
    fn test_place_into_tailplane() {
        let shape = shape("M38 100L16 20H52.5L113 100H38Z");

        let dim = Dim::new(pt(40.0), pt(10.0), pt(20.0));
        let approx_correct = Point::new(pt(31.0), pt(75.0) - dim.depth);

        let found = left_right_collider(&shape).place(dim, pt(0.0));
        assert_approx_eq!(found, Some(approx_correct), tolerance = 1.0);
    }

    fn hat_shape() -> BezShape {
        shape("M65.5 27.5H21.5L29 64.5L15.5 104.5H98L80 64.5L65.5 27.5Z")
    }

    fn hat_collider() -> BezColliderGroup {
        let shape = hat_shape();
        let curves: Vec<_> = shape.curves().collect();
        BezColliderGroup {
            segments: vec![
                BezColliderSegment { left: curves[1], right: curves[5].rev() },
                BezColliderSegment { left: curves[2], right: curves[4].rev() },
            ]
        }
    }

    #[test]
    fn test_place_into_top_of_hat() {
        let dim = Dim::new(pt(35.0), pt(15.0), pt(15.0));
        let approx_correct = Point::new(pt(28.0), pt(58.0) - dim.depth);

        let found = hat_collider().place(dim, pt(0.0));
        assert_approx_eq!(found, Some(approx_correct), tolerance = 1.0);
    }

    #[test]
    fn test_place_into_mid_of_hat() {
        let dim = Dim::new(pt(43.0), pt(15.0), pt(15.0));
        let approx_correct = Point::new(pt(29.0), pt(44.0) + dim.height);

        let found = hat_collider().place(dim, pt(0.0));
        assert_approx_eq!(found, Some(approx_correct), tolerance = 0.1);
    }

    #[test]
    fn test_place_into_bot_of_hat() {
        let dim = Dim::new(pt(65.0), pt(10.0), pt(2.0));
        let approx_correct = Point::new(pt(23.0), pt(83.0) + dim.height);

        let found = hat_collider().place(dim, pt(0.0));
        assert_approx_eq!(found, Some(approx_correct), tolerance = 1.0);
    }

    #[test]
    fn test_build_hat_collider() {
        let shape = hat_shape();
        let collider = BezColliderGroup::new(&shape);
        assert_approx_eq!(collider, hat_collider());
    }

    #[test]
    fn test_build_curvy_collider() {
        let shape = shape("
            M32.2171 19C32.2171 19 16.2907 22.3262 14.5088 30.3292C12.6925
            38.4862 14.5088 87.4283 40.8443 111.446C49.5993 119.431 85.2274
            117.402 104.867 101.476C112.132 95.5853 98.51 33.9545 98.51
            33.9545C98.51 33.9545 88.1128 31.0753 82.6178 33.9545C76.9571
            36.9206 73.5366 47.5495 73.5366 47.5495H50.3796L32.2171 19Z
        ");

        let _collider = BezColliderGroup::new(&shape);
    }
}
