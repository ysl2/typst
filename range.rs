//! Floating-point ranges.

use std::cmp::Ordering;
use super::value_no_nans;

/// A float range.
pub type Range = std::ops::Range<f64>;

/// Simplifies a set of ranges into a run of non-overlapping intervals.
pub fn simplify(mut ranges: Vec<Range>) -> Vec<Range> {
    ranges.sort_by(|a, b| value_no_nans(&a.start, &b.start));

    let mut out: Vec<Range> = vec![];

    for range in ranges {
        if let Some(prev) = out.last_mut() {
            if range.start <= prev.end {
                prev.end = prev.end.max(range.end);
                continue;
            }
        }

        out.push(range);
    }

    out
}

/// Shrinks all intervals by the given values and removes disappearing
/// intervals.
pub fn shrink(mut ranges: Vec<Range>, start: f64, end: f64) -> Vec<Range> {
    for range in &mut ranges {
        range.start += start;
        range.end -= end;
    }

    ranges.retain(|r| r.end > r.start);
    ranges
}

/// Returns the inverse run of a run of non-overlapping intervals.
pub fn inverse(ranges: &[Range]) -> Vec<Range> {
    let mut out = vec![];

    if ranges.is_empty() {
        out.push(f64::NEG_INFINITY .. f64::INFINITY);
    }

    if let Some(range) = ranges.first() {
        if range.start > f64::NEG_INFINITY {
            out.push(f64::NEG_INFINITY .. range.start);
        }
    }

    for window in ranges.windows(2) {
        let first = window[0].clone();
        let second = window[1].clone();

        if first.end < second.start {
            out.push(first.end .. second.start);
        }
    }

    if let Some(range) = ranges.last() {
        if range.end < f64::INFINITY {
            out.push(range.end .. f64::INFINITY);
        }
    }

    out
}

/// Returns the intersections of multiple runs of non-overlapping intervals.
pub fn intersect(ranges: &[Vec<Range>]) -> Vec<Range> {
    match ranges.len() {
        0 => return vec![],
        1 => return ranges[0].clone(),
        _ => {}
    }

    let mut out = vec![];
    let mut index = vec![0; ranges.len()];

    'main: loop {
        let mut start = f64::NEG_INFINITY;
        let mut end = f64::INFINITY;
        let mut min_end = 0;

        for (i, (r, &ri)) in ranges.iter().zip(&index).enumerate() {
            if ri >= r.len() {
                break 'main;
            }

            if r[ri].start > start {
                start = r[ri].start;
            }

            if r[ri].end < end {
                end = r[ri].end;
                min_end = i;
            }
        }

        if start < end {
            out.push(Range { start, end });
        }

        index[min_end] += 1;
    }

    out
}

/// An comparison function which returns equal when a value falls into a range
/// and less or greater when it is before or after the range.
pub fn value_relative_to_range(range: Range, v: f64) -> Ordering {
    if range.start > v {
        Ordering::Greater
    } else if range.end <= v {
        Ordering::Less
    } else {
        Ordering::Equal
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simplification_joins_ranges() {
        assert_eq!(
            simplify(vec![11.0..12.0, -4.0..3.0, 10.0..15.0, 6.0..7.0, 2.0..5.0]),
            vec![-4.0..5.0, 6.0..7.0, 10.0..15.0],
        );
    }

    #[test]
    fn test_intersection_from_three_vecs() {
        assert_eq!(
             intersect(&[
                vec![1.0..4.0, 5.0..9.0, 9.0..12.0],
                vec![f64::NEG_INFINITY..3.0, 6.0..13.0],
                vec![1.0..2.0, 2.0..3.0, 4.0..11.0],
            ]),
            vec![1.0..2.0, 2.0..3.0, 6.0..9.0, 9.0..11.0],
        );
    }

    #[test]
    fn test_inverse_of_empty_is_everything() {
        assert_eq!(inverse(&[]), vec![f64::NEG_INFINITY..f64::INFINITY]);
    }

    #[test]
    fn test_inverse_of_infinite_in_one_dir_is_infinite_in_other_dir() {
        assert_eq!(
            inverse(&vec![3.0..f64::INFINITY]),
            vec![f64::NEG_INFINITY..3.0],
        );
    }

    #[test]
    fn test_inverse_with_finite_intervals() {
        assert_eq!(
            inverse(&vec![-3.0..5.0, 8.0..11.0, 11.0..12.0]),
            vec![f64::NEG_INFINITY..-3.0, 5.0..8.0, 12.0..f64::INFINITY]
        );
    }
}
