//! Collisions and placement of objects.

use super::{ApproxEq, Bez, Dim, Length, Point, Vec2};

#[derive(Debug, Clone)]
pub struct BezColliderGroup {
    segments: Vec<BezColliderSegment>,
}

#[derive(Debug, Clone)]
struct BezColliderSegment {
    /// The top end of the segment.
    top: Length,
    /// The bottom end of the segment.
    bot: Length,
    /// The left border of the segment (start point is at the top, end point at
    /// the bottom).
    left: Bez,
    /// The right border of the segment (start point is at the top, end point at
    /// the bottom).
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
    pub fn place(&self, dim: Dim, _top: Length) -> Point {
        assert_eq!(self.segments.len(), 1);
        let seg = &self.segments[0];
        let top = seg.search_bisect(dim.width, 1e-2).expect("oops");
        top + Vec2::with_y(dim.height)
    }
}

impl BezColliderSegment {
    /// Search for a position in this segment with the given `width`.
    ///
    /// The returned point lies on the left border at a position where the
    /// distance to the right border is `width`.
    fn search_bisect(&self, width: Length, tolerance: f32) -> Option<Point> {
        const MAX_ITERS: usize = 30;

        let top_width = self.right.start.x - self.left.start.x;
        let bot_width = self.right.end.x - self.left.end.x;
        let (mut min, mut min_width, mut max, mut max_width) =
            if self.top < self.bot {
                (self.top, top_width, self.bot, bot_width)
            } else {
                (self.bot, bot_width, self.top, top_width)
            };

        // Since the border curves are monotone, all widths are in the interval
        // between top and bottom widths and as such if the width is not in the
        // interval, there is no match.
        if width < min_width || width > max_width {
            return None;
        }

        let mut iter = 1;
        loop {
            let full_delta = max_width - min_width;
            let wanted_delta = width - min_width;
            let ratio = wanted_delta / full_delta;

            assert!(0.0 <= ratio && ratio <= 1.0);

            let y = min + ratio * (max - min);
            let left_x  = find_one_x(self.left, y);
            let right_x = find_one_x(self.right, y);
            let y_width = right_x - left_x;

            if y_width.approx_eq(&width, tolerance) {
                println!("info: convereged in {}. iteration", iter);
                return Some(Point::new(left_x, y));
            }

            if width < y_width {
                max = y;
                max_width = y_width;
            } else {
                min = y;
                min_width = y_width;
            }

            if iter > MAX_ITERS {
                println!("warning: newton search did not converge");
                return None;
            }

            iter += 1;
        }
    }
}

/// Tries to find the only `x` position at which the curve has the given `y`
/// value and panics if there are no or multiple such positions.
fn find_one_x(curve: Bez, y: Length) -> Length {
    match curve.x_for_y(y).as_slice() {
        &[] => panic!("there should be at least one root"),
        &[x] => x,
        xs => panic!("curve is not monotone and has multiple roots: {:?}", xs),
    }
}

#[cfg(test)]
mod tests {
    use super::super::{BezShape, Rect, pt};
    use super::*;

    #[allow(unused)]
    fn dim_rect(point: Point, dim: Dim) -> Rect {
        Rect::new(
            point - Vec2::with_y(dim.height),
            point + Vec2::new(dim.width, dim.depth),
        )
    }

    #[test]
    fn test_layout_into_trapez() {
        let shape = BezShape::from_svg_path("M20 100L40 20H80L100 100H20Z").unwrap();

        let curves: Vec<_> = shape.curves().collect();
        let group = BezColliderGroup {
            segments: vec![BezColliderSegment {
                top: pt(20.0),
                bot: pt(100.0),
                left: curves[0].rev(),
                right: curves[2],
            }],
        };

        let dim = Dim::new(pt(50.0), pt(10.0), pt(5.0));
        let found = group.place(dim, pt(25.0));
        let correct = Point::new(pt(35.0), pt(40.0) + dim.height);
        assert_approx_eq!(found, correct);
    }

    #[test]
    fn test_layout_into_silo() {
        let shape = BezShape::from_svg_path("
            M20 100C20 100 28 32 40 20C52 8.00005 66 8.5 80 20C94 31.5 100 100
            100 100H20Z
        ").unwrap();

        let curves: Vec<_> = shape.curves().collect();
        let group = BezColliderGroup {
            segments: vec![BezColliderSegment {
                top: pt(20.0),
                bot: pt(100.0),
                left: curves[0].rev(),
                right: curves[2],
            }],
        };

        let dim = Dim::new(pt(70.0), pt(10.0), pt(20.0));
        let found = group.place(dim, pt(0.0));
        let approx_correct = Point::new(pt(25.0), pt(66.0) + dim.height);
        assert_approx_eq!(found, approx_correct, tolerance = 1.0);
    }
}
