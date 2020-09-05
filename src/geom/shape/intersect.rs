use super::*;
use arrayvec::{Array, ArrayVec};

/// Find the intersections of two curves recursively using bounding boxes.
///
/// The points are in no particular order. No guarantees are made about which
/// points are returned when the curves have coinciding segments.
///
/// The size of the array-vec can be defined by the caller to give a boost in
/// performance in situations were there is a known bound on the number of
/// intersections. This is because this function is recursive and quite a few of
/// those vecs will be allocated on the stack depending on the `accuracy`. To be
/// safe in a cubic bezier situation, use `9`. For monotone curves, use `3`. At
/// most as many intersection as the array-vec has capacity will be reported.
///
/// This function computes many bounding boxes of curves. Since this operation
/// is very fast for monotone curves, consider using the `Monotone` wrapper if
/// your curves are monotone.
pub fn find_intersections_bbox<C, A>(a: &C, b: &C, accuracy: f64) -> ArrayVec<A>
where
    C: ParamCurveExtrema,
    A: Array<Item = Point>,
{
    let mut result = ArrayVec::new();

    let ba = a.bounding_box();
    let bb = b.bounding_box();

    // When the bounding boxes don't overlap we have no intersection.
    if !ba.overlaps(&bb) {
        return result;
    }

    // When the bounding boxes do overlap, but one of the curves is smaller than
    // the accuracy, any point inside that curve is fine as our intersection, so
    // we just pick the center of its bounding box.
    if ba.width() < accuracy && ba.height() < accuracy {
        result.push(ba.center());
        return result;
    }

    if bb.width() < accuracy && bb.height() < accuracy {
        result.push(bb.center());
        return result;
    }

    // When we are not at the accuracy level, we continue by subdividing both
    // curves and intersecting each pair.
    let (a1, a2) = a.subdivide();
    let (b1, b2) = b.subdivide();

    let double = 2.0 * accuracy;
    let mut extend = |values: ArrayVec<A>| {
        for point in values {
            // We don't want to count intersections twice.
            if !result.is_full() && !result.iter().any(|p| p.approx_eq(&point, double)) {
                result.push(point);
            }
        }
    };

    extend(find_intersections_bbox(&a1, &b1, accuracy));
    extend(find_intersections_bbox(&a1, &b2, accuracy));
    extend(find_intersections_bbox(&a2, &b1, accuracy));
    extend(find_intersections_bbox(&a2, &b2, accuracy));

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::cmp::value_no_nans;

    fn seg(d: &str) -> PathSeg {
        BezPath::from_svg(d).unwrap().segments().next().unwrap()
    }

    #[test]
    fn test_intersect_monotone_two_intersections() {
        let a = Monotone(seg("M9 31C37.5 31 59 61 59 81"));
        let b = Monotone(seg("M21 20C21 40 42.5 70 71 70"));

        assert_approx_eq!(
            a.intersect::<[_; 3]>(&b, 0.01).to_vec(),
            vec![Point::new(24.0, 34.0), Point::new(56.0, 67.0)],
            tolerance = 0.5,
        );
    }

    #[test]
    fn test_intersect_monotone_three_intersections() {
        let a = Monotone(seg("M59 81C14 74.5 37.5 31 9 31"));
        let b = Monotone(seg("M17 31C17 81 50 53 50 81"));

        let mut vec = a.intersect::<[_; 3]>(&b, 0.01).to_vec();
        vec.sort_by(|a, b| value_no_nans(&a.y, &b.y));

        assert_approx_eq!(
            vec,
            vec![
                Point::new(17.0, 32.5),
                Point::new(31.5, 63.5),
                Point::new(50.0, 79.0),
            ],
            tolerance = 0.25,
        );
    }

    #[test]
    fn test_intersect_not_monotone_five_intersections() {
        let a = seg("M53 69C82 12 -2 -11 23 69");
        let b = seg("M31 63C-71 14 187 75 11 17");

        let mut vec = find_intersections_bbox::<_, [_; 5]>(&a, &b, 0.01).to_vec();
        vec.sort_by(|a, b| value_no_nans(&a.y, &b.y));

        assert_approx_eq!(
            vec,
            vec![
                Point::new(25.0, 21.5),
                Point::new(56.5, 33.0),
                Point::new(18.0, 42.0),
                Point::new(59.0, 44.0),
                Point::new(20.0, 57.5),
            ],
            tolerance = 0.5,
        );
    }

    #[test]
    fn test_intersect_curve_with_itself() {
        let a1 = seg("M53 69C82 12 -2 -11 23 69");
        let a2 = seg("M53 69C82 12 -2 -11 23 69");

        let vec = find_intersections_bbox::<_, [_; 10]>(&a1, &a2, 0.01).to_vec();
        assert_eq!(vec.len(), 10);
    }
}
