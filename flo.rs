use super::{Bez, Rect, Point, Length};

pub use flo_curves::{BezierCurve, BoundingBox, Coordinate};
pub use flo_curves::{Coord2 as FloCoord};

pub type FloCurve = flo_curves::bezier::Curve<FloCoord>;
pub type FloBounds = flo_curves::Bounds<FloCoord>;

/// Transform a point into flo coordinate.
pub fn flo_coord(point: Point) -> FloCoord {
    FloCoord(point.x.to_pt() as f64, point.y.to_pt() as f64)
}

/// Transform a curve struct into a flo curve.
pub fn flo_curve(curve: Bez) -> FloCurve {
    FloCurve {
        start_point: flo_coord(curve.start),
        control_points: (flo_coord(curve.c1), flo_coord(curve.c2)),
        end_point: flo_coord(curve.end),
    }
}

/// Transform a flo coordinate into a point.
pub fn unflo_point<P: Coordinate>(coord: P) -> Point {
    Point::new(Length::pt(coord.get(0) as f32), Length::pt(coord.get(1) as f32))
}

/// Transform flo bounds into a rect.
pub fn unflo_rect<B: BoundingBox>(bounds: B) -> Rect {
    Rect::new(unflo_point(bounds.min()), unflo_point(bounds.max()))
}

pub fn unflo_bez<C: BezierCurve>(curve: C) -> Bez {
    let start = curve.start_point();
    let (c1, c2) = curve.control_points();
    let end = curve.end_point();

    Bez {
        start: unflo_point(start),
        c1: unflo_point(c1),
        c2: unflo_point(c2),
        end: unflo_point(end),
    }
}
