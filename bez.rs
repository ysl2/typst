use std::fmt;
use svgtypes::{PathParser, PathSegment};
use super::{ApproxEq, Length, Point};

/// A closed shape defined by a list of connected cubic bezier curves.
#[derive(Debug, Clone, PartialEq)]
pub struct BezShape {
    /// The list of start-, control- and endpoints as explained in
    /// `BezShape::new`.
    points: Vec<Point>,
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

    /// Iterator over the bezier curves defined by the point listing.
    pub fn curves<'a>(&'a self) -> impl Iterator<Item=Bez> + 'a {
        curves(&self.points)
    }
}

impl_approx_eq!(BezShape [points]);
impl_approx_eq!(Bez [start, c1, c2, end]);

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

/// An error that can occur when parsing a `BezShape` from a svg path.
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

    macro_rules! bezshape {
        ($(($x:expr, $y:expr)),* $(,)?) => {
            BezShape {
                points: vec![$(Point::new(pt($x as f32), pt($y as f32))),*],
            }
        };
    }

    #[test]
    fn test_parse_svg_path_with_line_commands() {
        assert_approx_eq!(
            BezShape::from_svg_path("M20 107.5L43 20.5H80L98.5 107.5H20Z").unwrap(),
            bezshape![
                (20, 107.5),
                (20, 107.5),
                (43, 20.5),
                (43, 20.5),
                (43, 20.5),
                (80, 20.5),
                (80, 20.5),
                (80, 20.5),
                (98.5, 107.5),
                (98.5, 107.5),
                (98.5, 107.5),
                (20, 107.5),
            ]
        );
    }

    #[test]
    fn test_parse_svg_path_consisting_of_only_one_curve() {
        let bez = BezShape::from_svg_path("M5 43C125 18 -25 -37 5 43Z").unwrap();
        assert_approx_eq!(bez, bezshape![(5, 43), (125, 18), (-25, -37)]);
        assert_approx_eq!(
            bez.curves().collect::<Vec<_>>(),
            vec![Bez {
                start: Point::new(pt(5.0), pt(43.0)),
                c1: Point::new(pt(125.0), pt(18.0)),
                c2: Point::new(pt(-25.0), pt(-37.0)),
                end: Point::new(pt(5.0), pt(43.0)),
            }],
        )
    }

    #[test]
    fn test_parse_svg_path_automatically_closes_path() {
        assert_approx_eq!(
            BezShape::from_svg_path("M1 15C10 -4 35 -4 45 15Z").unwrap(),
            bezshape![
                (1, 15),
                (10, -4),
                (35, -4),
                (45, 15),
                (45, 15),
                (1, 15),
            ]
        );
    }
}
