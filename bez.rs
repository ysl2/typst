use std::fmt;
use arrayvec::ArrayVec;
use svgtypes::{PathParser, PathSegment};
use super::{ApproxEq, Length, Point, Rect};

/// A closed shape defined by a list of connected cubic bezier curves.
#[derive(Debug, Clone, PartialEq)]
pub struct BezShape {
    /// The list of start-, control- and endpoints as explained in
    /// `BezShape::new`.
    points: Vec<Point>,
}

impl BezShape {
    /// Create a new shape from a list of points in the format `[s1, c1, c1, e1,
    /// c2, c2, e2, ...]` where:
    /// - `s_n` is the starting point of the n-th cubic bezier,
    /// - `c_n` are the control points of the n-th curve,
    /// - `e_n` (or equivalently `s_(n+1)`) is the shared end point of the n-th
    ///   and start point of the (n+1)-th curve.
    ///
    /// The shape is always closed and as such the last curve is defined by the
    /// last three and the first point in the listing. As a consequence the
    /// number of points is always divisble by three.
    ///
    /// When the shape consists of only one curve, this curve must have the same
    /// start and end point for the shape to be closed. Then, the point vector
    /// shall contain only three points.
    pub fn new(points: Vec<Point>) -> BezShape {
        assert!(
            points.len() >= 3 && points.len() % 3 == 0,
            "invalid shape definition",
        );

        BezShape { points }
    }

    /// Parses an svg path into a bezier shape.
    ///
    /// The coordinates are interpreted as having the unit `pt`. The path may
    /// only contain `moveto`, the different `lineto`, `curveto` and `closepath`
    /// commands, other commands are not supported. All coordinates have to be
    /// absolute.
    pub fn from_svg_path(d: &str) -> Result<BezShape, ParseSvgError> {
        let mut open = false;
        let mut done = false;
        let mut points = vec![];

        let coord = |v: f64| Length::pt(v as f32);
        let point = |x: f64, y: f64| Point::new(coord(x), coord(y));

        for segment in PathParser::from(d) {
            if done {
                return Err(ParseSvgError::Multiple);
            }

            match segment.map_err(|_| ParseSvgError::Invalid)? {
                PathSegment::MoveTo { abs: true, x, y } if !open => {
                    points.push(point(x, y));
                    open = true;
                }

                PathSegment::LineTo { abs: true, x, y } if open => {
                    let start = *points.last().unwrap();
                    let end = point(x, y);
                    points.push(start);
                    points.push(end);
                    points.push(end);
                }

                PathSegment::HorizontalLineTo { abs: true, x } if open => {
                    let start = *points.last().unwrap();
                    let end = Point::new(coord(x), start.y);
                    points.push(start);
                    points.push(end);
                    points.push(end);
                }

                PathSegment::VerticalLineTo { abs: true, y } if open => {
                    let start = *points.last().unwrap();
                    let end = Point::new(start.x, coord(y));
                    points.push(start);
                    points.push(end);
                    points.push(end);
                }

                PathSegment::CurveTo { abs: true, x1, y1, x2, y2, x, y } if open => {
                    points.push(point(x1, y1));
                    points.push(point(x2, y2));
                    points.push(point(x, y));
                }

                PathSegment::ClosePath { .. } if open => {
                    let start = points[0];
                    let end = *points.last().unwrap();

                    // If start and end point are already equal, we remove the
                    // end point. Otherwise, we create a last segment between
                    // end and start to close the shape.
                    if start.approx_eq(&end, 1e-5) {
                        points.pop();
                    } else {
                        points.push(end);
                        points.push(start);
                    }

                    open = false;
                    done = true;
                }

                _ if !open => return Err(ParseSvgError::Unopened),
                _ => return Err(ParseSvgError::UnsupportedCommand)
            }
        }

        if open || points.is_empty() {
            return Err(ParseSvgError::Unclosed);
        }

        Ok(BezShape::new(points))
    }

    /// The axis-aligned bounding box of the shape.
    pub fn bounds(&self) -> Rect {
        let mut bounds = Rect::new(self.points[0], self.points[0]);
        for bez in self.curves() {
            bounds = bounds.union(bez.bounds());
        }
        bounds
    }

    /// Translate the shape along the `x` and `y` axes.
    pub fn translate(&mut self, x: Length, y: Length) {
        for point in &mut self.points {
            point.x += x;
            point.y += y;
        }
    }

    /// Scale the shape along the `x` and `y` axes.
    pub fn scale(&mut self, x: f32, y: f32) {
        for point in &mut self.points {
            point.x *= x;
            point.y *= y;
        }
    }

    /// Iterator over the bezier curves defined by the point listing.
    pub fn curves<'a>(&'a self) -> impl Iterator<Item=Bez> + 'a {
        curves(&self.points)
    }
}

impl_approx_eq!(BezShape [points]);

/// Iterator over the curves defined by a point list.
fn curves<'a>(points: &'a [Point]) -> impl Iterator<Item=Bez> + 'a {
    let mut i = 0;
    std::iter::from_fn(move || {
        if i >= points.len() {
            None
        } else {
            i += 3;
            Some(Bez {
                start: points[i - 3],
                c1: points[i - 2],
                c2: points[i - 1],
                end: points[i % points.len()],
            })
        }
    })
}

/// A cubic bezier curve.
///
/// Such a curve consists of a start point, two control points and an end point.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Bez {
    pub start: Point,
    pub c1: Point,
    pub c2: Point,
    pub end: Point,
}

impl Bez {
    /// Evaluate the curve for parameter value `t`.
    pub fn point_for_t(self, t: f32) -> Point {
        let omt = 1.0 - t;
        let dir = omt * omt * omt * self.start.to_vec2()
             + 3.0 * omt * omt * t * self.c1.to_vec2()
             + 3.0 * omt * t * t * self.c2.to_vec2()
             + t * t * t * self.end.to_vec2();
        dir.to_point()
    }

    /// Solve a cubic equation to find the `y` values corresponding to an `x`.
    pub fn solve_y_for_x(self, x: Length) -> ArrayVec<[Length; 3]> {
        self.solve_t_for_x(x)
            .into_iter()
            .map(|t| self.point_for_t(t).y)
            .collect()
    }

    /// Solve a cubic equation to find the `x` values corresponding to a `y`.
    pub fn solve_x_for_y(self, y: Length) -> ArrayVec<[Length; 3]> {
        self.solve_t_for_y(y)
            .into_iter()
            .map(|t| self.point_for_t(t).x)
            .collect()
    }

    /// Find all `t` values where the curve has the given `x` value.
    pub fn solve_t_for_x(self, x: Length) -> ArrayVec<[f32; 3]> {
        solve_t_for_v(self.start.x, self.c1.x, self.c2.x, self.end.x, x)
    }

    /// Find all `t` values where the curve has the given `y` value.
    pub fn solve_t_for_y(self, y: Length) -> ArrayVec<[f32; 3]> {
        solve_t_for_v(self.start.y, self.c1.y, self.c2.y, self.end.y, y)
    }

    /// The axis-aligned bounding box of the curve.
    pub fn bounds(self) -> Rect {
        use super::flo::*;
        unflo_rect(flo_curve(self).bounding_box::<FloBounds>())
    }

    /// The reverse curve (`start` & `end` and `c1` & `c2` swapped).
    pub fn rev(self) -> Self {
        Self {
            start: self.end,
            c1: self.c2,
            c2: self.c1,
            end: self.start,
        }
    }
}

/// Find all `t` values where the curve has the given `v` value in the dimension
/// for which the control values are given.
fn solve_t_for_v(
    start: Length,
    c1: Length,
    c2: Length,
    end: Length,
    v: Length,
) -> ArrayVec<[f32; 3]> {
    let a = (-start + 3.0 * c1 - 3.0 * c2 + end).to_pt();
    let b = (3.0 * start - 6.0 * c1 + 3.0 * c2).to_pt();
    let c = (-3.0 * start + 3.0 * c1).to_pt();
    let d = (start - v).to_pt();

    super::roots::solve_cubic(a, b, c, d)
        .into_iter()
        .filter(|&t| 0.0 <= t && t <= 1.0)
        .collect()
}

impl_approx_eq!(Bez [start, c1, c2, end]);

/// An error that can occur when parsing a `BezShape` from an svg path.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ParseSvgError {
    /// The path is invalid.
    Invalid,
    /// The path contains an unsupported command.
    UnsupportedCommand,
    /// The path did not start with a `moveto`.
    Unopened,
    /// The path is not closed.
    Unclosed,
    /// The svg path consists of multiple paths, but a bezier shape can only
    /// hold one path.
    Multiple,
}

impl fmt::Display for ParseSvgError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Invalid => f.pad("invalid"),
            Self::UnsupportedCommand => f.pad("unsupported command"),
            Self::Unopened => f.pad("unopened"),
            Self::Unclosed => f.pad("unclosed"),
            Self::Multiple => f.pad("multiple"),
        }
    }
}

impl std::error::Error for ParseSvgError {}

#[cfg(test)]
mod tests {
    use super::super::pt;
    use super::*;

    macro_rules! points {
        ($(($x:expr, $y:expr)),* $(,)?) => {
            vec![$(Point::new(pt($x as f32), pt($y as f32))),*]
        };
    }

    macro_rules! bez {
        ($($tts:tt)*) => {{
            let points = points![$($tts)*];
            Bez {
                start: points[0],
                c1: points[1],
                c2: points[2],
                end: points[3],
            }
        }};
    }

    #[test]
    fn test_parse_svg_path_with_line_commands() {
        let shape = BezShape::from_svg_path("M20 107.5L43 20.5H80L98.5 107.5H20Z").unwrap();
        assert_approx_eq!(
            shape.points,
            points![
                (20, 107.5), (20, 107.5), (43, 20.5), (43, 20.5), (43, 20.5),
                (80, 20.5), (80, 20.5), (80, 20.5), (98.5, 107.5),
                (98.5, 107.5), (98.5, 107.5), (20, 107.5),
            ],
        );
    }

    #[test]
    fn test_parse_svg_path_consisting_of_only_one_curve() {
        let shape = BezShape::from_svg_path("M5 43C125 18 -25 -37 5 43Z").unwrap();
        assert_approx_eq!(shape.points, points![(5, 43), (125, 18), (-25, -37)]);
        assert_approx_eq!(
            shape.curves().collect::<Vec<_>>(),
            vec![bez![(5, 43), (125, 18), (-25, -37), (5, 43)]],
        )
    }

    #[test]
    fn test_parse_svg_path_automatically_closes_path() {
        let shape = BezShape::from_svg_path("M1 15C10 -4 35 -4 45 15Z").unwrap();
        assert_approx_eq!(
            shape.points,
            points![(1, 15), (10, -4), (35, -4), (45, 15), (45, 15), (1, 15)],
        );
    }

    fn simple_curve() -> Bez {
        bez![(0, 0), (35, 0), (80, 35), (80, 70)]
    }

    #[test]
    fn test_bez_point_for_t() {
        let bez = simple_curve();

        assert_approx_eq!(bez.point_for_t(0.0), bez.start);
        assert_approx_eq!(bez.point_for_t(1.0), bez.end);

        let point = Point::new(pt(32.7), pt(8.5));
        assert_approx_eq!(bez.point_for_t(0.3), point, tolerance=0.1);
    }

    #[test]
    fn test_bez_solve_for_coordinate_for_different_sampled_points() {
        let eps = 1e-3;
        let bez = simple_curve();

        let ts = [0.0, 0.01, 0.2, 0.5, 0.7, 0.99, 1.0];
        for &t in &ts {
            let Point { x, y } = bez.point_for_t(t);

            assert_approx_eq!(bez.solve_t_for_x(x).to_vec(), vec![t], tolerance=eps);
            assert_approx_eq!(bez.solve_t_for_y(y).to_vec(), vec![t], tolerance=eps);
            assert_approx_eq!(bez.solve_y_for_x(x).to_vec(), vec![y], tolerance=eps);
            assert_approx_eq!(bez.solve_x_for_y(y).to_vec(), vec![x], tolerance=eps);
        }
    }

    #[test]
    fn test_bez_solve_for_coordinate_out_of_bounds() {
        let bez = simple_curve();

        assert!(bez.solve_x_for_y(pt(-10.0)).is_empty());
        assert!(bez.solve_x_for_y(pt(100.0)).is_empty());
        assert!(bez.solve_y_for_x(pt(-20.0)).is_empty());
        assert!(bez.solve_y_for_x(pt(100.0)).is_empty());
    }
}
