//! Collisions and placement of objects.

use super::{ApproxEq, Bez, Dim, VDim, Length, Point};

#[derive(Debug, Clone)]
pub struct BezColliderGroup {
    segments: Vec<BezColliderSegment>,
}

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
    /// Finds the top-most point in the shape to place an object with dimensions
    /// `dim`.
    ///
    /// Specifically, the point is selected such that:
    /// - An object with dimensions `dim` fits at that baseline anchor point
    ///   without colliding with the shape.
    /// - The whole object is placed below `top` (that is `point.y - dim.height
    ///   >= top`).
    pub fn place(&self, dim: Dim, _top: Length) -> Option<Point> {
        self.segments[0].search_bisect(dim, 1e-2)
    }
}

impl BezColliderSegment {
    /// Search for a position in this segment to fit the given dimensions.
    ///
    /// The returned point lies on the left border at a position where the
    /// distance to the right border is `width`.
    fn search_bisect(&self, dim: Dim, tolerance: f32) -> Option<Point> {
        const MAX_ITERS: usize = 20;

        let Dim { width, height, depth } = dim;

        // The real top and bottom end of the search interval for the object's
        // origin point is inset by the height and depth of the object - lower
        // or higher would make the object stick out of the segment.
        let mut top = self.top() + height;
        let mut bot = self.bot() - depth;

        // If the object is higher than the segment, it cannot fit.
        if top > bot {
            println!("info: object is too high");
            return None;
        }

        // Which corner (top or bottom) of the object we want to check against
        // each border depending on whether the border curve in question is
        // widening or shrinking the segment with growing y-value.
        let offsets = (
            tighter_offset(self.left.start.x >= self.left.end.x, dim.vdim()),
            tighter_offset(self.right.start.x <= self.right.end.x, dim.vdim()),
        );

        // Determine the widths at the boundaries.
        let (top_left_x, top_right_x) = self.borders_x_for_y(top, offsets);
        let (bot_left_x, bot_right_x) = self.borders_x_for_y(bot, offsets);
        let mut top_width = top_right_x - top_left_x;
        let mut bot_width = bot_right_x - bot_left_x;

        // If it already fits at the top, we're good.
        if width <= top_width {
            println!("info: fits at the top");
            return Some(Point::new(top_left_x, top));
        }

        // If it does not fit at the top and also not at the bottom, it won't fit
        // at all, since the width function is monotonous.
        if width > bot_width {
            println!("info: object is too wide");
            return None;
        }

        let mut iter = 1;
        loop {
            // Determine the next `y` value by linear interpolation between the
            // min and max bounds.
            let ratio = (width - top_width) / (bot_width - top_width);
            let y = top + ratio * (bot - top);

            let (left_x, right_x) = self.borders_x_for_y(y, offsets);
            let y_width = right_x - left_x;

            // Check whether we converged to a good spot.
            if y_width.approx_eq(&width, tolerance) {
                println!("info: converged in {}. iteration", iter);
                return Some(Point::new(left_x, y));
            }

            // Adjust the bounds by replacing the bad bound with the better
            // estimate.
            if width < y_width {
                bot = y;
                bot_width = y_width;
            } else {
                top = y;
                top_width = y_width;
            }

            if iter > MAX_ITERS {
                println!("warning: bisection search did not converge");
                return None;
            }

            iter += 1;
        }
    }

    /// The `x` values at y-values `y + offsets.0` for the left and
    /// `y + offsets.1` for the right border.
    fn borders_x_for_y(
        &self,
        y: Length,
        offsets: (Length, Length),
    ) -> (Length, Length) {
        let left_x  = find_one_x(self.left, y + offsets.0);
        let right_x = find_one_x(self.right, y + offsets.1);
        (left_x, right_x)
    }

    /// The top end of this segment.
    fn top(&self) -> Length {
        self.left.start.y
    }

    /// The bottom end of this segment.
    fn bot(&self) -> Length {
        self.left.end.y
    }
}

/// The offset from the origin point to the corner (top or bottom) at which the
/// curve is tighter. The bool `widening` should be true when the curve is
/// widening the segment from with growing y-value.
fn tighter_offset(widening: bool, vdim: VDim) -> Length {
    if widening { -vdim.height } else { vdim.depth }
}

/// Tries to find the only `x` position at which the curve has the given `y`
/// value.
///
/// This will panic if there are no or multiple such positions except if `y` is
/// equal to the start- or end-points y-coordinate.
fn find_one_x(curve: Bez, y: Length) -> Length {
    const EPS: f32 = 1e-4;

    // No need to compute roots for start and end point.
    if y.approx_eq(&curve.start.y, EPS) {
        return curve.start.x;
    } else if y.approx_eq(&curve.end.y, EPS) {
        return curve.end.x;
    }

    match curve.x_for_y(y).as_slice() {
        &[] => panic!("there should be at least one root"),
        &[x] => x,
        xs => panic!("curve is not monotone and has multiple roots: {:?}", xs),
    }
}

#[cfg(test)]
mod tests {
    use super::super::{BezShape, Rect, Vec2, pt};
    use super::*;

    fn shape(path: &str) -> BezShape {
        BezShape::from_svg_path(path).unwrap()
    }

    fn collider_curves_0_2(shape: &BezShape) -> BezColliderGroup {
        let curves: Vec<_> = shape.curves().collect();
        BezColliderGroup {
            segments: vec![
                BezColliderSegment { left: curves[0].rev(), right: curves[2] },
            ],
        }
    }

    #[allow(unused)]
    fn dim_rect(point: Point, dim: Dim) -> Rect {
        Rect::new(
            point - Vec2::with_y(dim.height),
            point + Vec2::new(dim.width, dim.depth),
        )
    }

    #[test]
    fn test_place_into_trapez() {
        let shape = shape("M20 100L40 20H80L100 100H20Z");
        let collider = collider_curves_0_2(&shape);

        let dim = Dim::new(pt(50.0), pt(10.0), pt(5.0));
        let correct = Point::new(pt(35.0), pt(40.0) + dim.height);
        let found = collider.place(dim, pt(25.0));
        assert_approx_eq!(found, Some(correct));
    }

    #[test]
    fn test_place_into_silo() {
        let shape = shape("
            M20 100C20 100 28 32 40 20C52 8.00005 66 8.5 80 20C94 31.5 100 100
            100 100H20Z
        ");
        let collider = collider_curves_0_2(&shape);

        let dim = Dim::new(pt(70.0), pt(10.0), pt(20.0));
        let approx_correct = Point::new(pt(25.0), pt(66.0) + dim.height);
        let found = collider.place(dim, pt(0.0));
        assert_approx_eq!(found, Some(approx_correct), tolerance = 1.0);
    }

    #[test]
    fn test_place_into_tailplane() {
        let shape = shape("M38 100L16 20H52.5L113 100H38Z");
        let collider = collider_curves_0_2(&shape);

        let dim = Dim::new(pt(40.0), pt(10.0), pt(20.0));
        let approx_correct = Point::new(pt(31.0), pt(75.0) - dim.depth);
        let found = collider.place(dim, pt(0.0));
        assert_approx_eq!(found, Some(approx_correct), tolerance = 1.0);
    }

    #[test]
    fn test_place_into_hat() {
        let shape = shape("
            M65.5 27.5H21.5L29 64.5L15.5 104.5H98L80 64.5L65.5 27.5Z
        ");

        let curves: Vec<_> = shape.curves().collect();
        let collider = BezColliderGroup {
            segments: vec![
                BezColliderSegment { left: curves[1], right: curves[5].rev() },
                BezColliderSegment { left: curves[2], right: curves[4].rev() },
            ]
        };

        let dim1 = Dim::new(pt(35.0), pt(15.0), pt(14.0));
        let approx_correct = Point::new(pt(28.0), pt(57.0) - dim1.depth);
        let found = collider.place(dim1, pt(0.0));
        assert_approx_eq!(found, Some(approx_correct), tolerance = 1.0);
    }
}
