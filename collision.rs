//! Collisions and placement of objects.

#![allow(unused)]

use super::{BezShape, Bez, Dim, Length, Point, Vec2, pt};

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
    /// The maximum x-value of the left border.
    left_max: Length,
    /// The right border of the segment (start point is at the top, end point at
    /// the bottom).
    right: Bez,
    /// The minimum x-value of the right border.
    right_min: Length,
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
    pub fn place(&self, dim: Dim, top: Length) -> Point {
        assert_eq!(self.segments.len(), 1);
        let seg = &self.segments[0];
        let h = dim.height + dim.depth;

        let top = seg.search_newton(dim.width).expect("oops");
        top + Vec2::with_y(dim.height)
    }
}

impl BezColliderSegment {
    fn search_newton(&self, width: Length) -> Option<Point> {
        let left_top_x  = self.left.start.x;
        let left_bot_x  = self.left.end.x;
        let right_top_x = self.right.start.x;
        let right_bot_x = self.right.end.x;

        let top_width = right_top_x - left_top_x;
        let bot_width = right_bot_x - left_bot_x;

        // Since the border curves are monotone, the width of this segment is
        // never the required width.
        if (width < top_width && width < bot_width) ||
           (width > top_width && width > bot_width) {
            return None;
        }

        let total_delta = bot_width - top_width;
        let wanted_delta = width - top_width;
        assert!(wanted_delta <= total_delta);

        // TODO: Actual search and not just linear interpolation.
        let ratio = wanted_delta / total_delta;
        let y = self.top + (self.bot - self.top) * ratio;
        let xs = self.left.x_for_y(y);
        assert_eq!(xs.len(), 1, "curve is not monotone, xs = {:?}", xs);
        let x = xs[0];

        Some(Point::new(x, y))
    }
}

#[cfg(test)]
mod tests {
    use super::super::pt;
    use super::*;

    #[test]
    fn test_layout_into_trapez() {
        let shape = BezShape::from_svg_path("M20 100L40 20H80L100 100H20Z").unwrap();
        let curves: Vec<_> = shape.curves().collect();

        let group = BezColliderGroup {
            segments: vec![BezColliderSegment {
                top: pt(20.0),
                bot: pt(100.0),
                left: curves[0].rev(),
                left_max: pt(20.0),
                right: curves[2],
                right_min: pt(80.0),
            }],
        };

        let dim = Dim::new(pt(50.0), pt(10.0), pt(5.0));
        let found = group.place(dim, pt(25.0));
        let correct = Point::new(pt(35.0), pt(40.0) + dim.height);
        assert_approx_eq!(found, correct);
    }
}
