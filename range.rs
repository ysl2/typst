use std::fmt;
use super::value_no_nans;

/// A range (_start_ / _end_) along an axis.
#[derive(Copy, Clone, PartialEq)]
pub struct Range {
    pub start: f64,
    pub end: f64,
}

/// The region a coordinate lies in relative to a range.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Region {
    Before,
    Inside,
    After,
}

impl Range {
    /// The range going from zero to zero.
    pub const ZERO: Range = Range { start: 0.0, end: 0.0 };

    /// Create a new range from `start` and `end`.
    pub fn new(start: f64, end: f64) -> Range {
        Range { start, end }
    }

    /// The distance between start and end.
    pub fn size(self) -> f64 {
        self.end - self.start
    }

    /// The mid-point of start and end.
    pub fn mid(self) -> f64 {
        (self.start + self.end) / 2.0
    }

    /// Whether the range is finite (both start and end are finite).
    pub fn is_finite(self) -> bool {
        self.start.is_finite() && self.end.is_finite()
    }

    /// The region of `v` relative to the range.
    pub fn region(self, v: f64) -> Region {
        if v < self.start {
            Region::Before
        } else if v > self.end {
            Region::After
        } else {
            Region::Inside
        }
    }

    /// This range shrunk by an amount at start and end.
    pub fn shrunk(self, start: f64, end: f64) -> Range {
        Range {
            start: self.start + start,
            end: self.end - end,
        }
    }

    /// This range extended by an extra amount at start and end.
    pub fn extended(self, start: f64, end: f64) -> Range {
        Range {
            start: self.start - start,
            end: self.end + end,
        }
    }

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
    pub fn shrink_run(mut ranges: Vec<Range>, start: f64, end: f64) -> Vec<Range> {
        for range in &mut ranges {
            range.start += start;
            range.end -= end;
        }

        ranges.retain(|r| r.size() > 0.0);
        ranges
    }

    /// Returns the inverse run of a run of non-overlapping intervals.
    pub fn inverse_run(ranges: &[Range]) -> Vec<Range> {
        let mut out = vec![];

        if ranges.is_empty() {
            out.push(Range::new(f64::NEG_INFINITY, f64::INFINITY));
        }

        if let Some(range) = ranges.first() {
            if range.start > f64::NEG_INFINITY {
                out.push(Range::new(f64::NEG_INFINITY, range.start));
            }
        }

        for window in ranges.windows(2) {
            let first = window[0];
            let second = window[1];

            if first.end < second.start {
                out.push(Range::new(first.end, second.start));
            }
        }

        if let Some(range) = ranges.last() {
            if range.end < f64::INFINITY {
                out.push(Range::new(range.end, f64::INFINITY));
            }
        }

        out
    }

    /// Returns the intersections of multiple runs of non-overlapping intervals.
    pub fn intersect_runs(ranges: &[Vec<Range>]) -> Vec<Range> {
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
}

impl_approx_eq!(Range [start, end]);

impl fmt::Debug for Range {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[{} .. {}]", self.start, self.end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INF: f64 = f64::INFINITY;

    macro_rules! ranges {
        ($(($start:expr, $end:expr)),* $(,)?) => {
            vec![$(Range { start: $start as f64, end: $end as f64 }),*]
        };
    }

    #[test]
    fn test_simplification_joins_ranges() {
        assert_eq!(
            Range::simplify(ranges![(11, 12), (-4, 3), (10, 15), (6, 7), (2, 5)]),
            ranges![(-4, 5), (6, 7), (10, 15)],
        );
    }

    #[test]
    fn test_intersection_from_three_vecs() {
        assert_eq!(
            Range::intersect_runs(&[
                ranges![(1, 4), (5, 9), (9, 12)],
                ranges![(-INF, 3), (6, 13)],
                ranges![(1, 2), (2, 3), (4, 11)],
            ]),
            ranges![(1, 2), (2, 3), (6, 9), (9, 11)],
        );
    }

    #[test]
    fn test_inverse_with_finite_intervals() {
        assert_eq!(
            Range::inverse_run(&ranges![(-3, 5), (8, 11), (11, 12)]),
            ranges![(-INF, -3), (5, 8), (12, INF)]
        );
    }

    #[test]
    fn test_inverse_of_empty_is_everything() {
        assert_eq!(Range::inverse_run(&[]), ranges![(-INF, INF)]);
    }

    #[test]
    fn test_inverse_of_infinite_in_one_dir_is_infinite_in_other_dir() {
        assert_eq!(Range::inverse_run(&ranges![(3, INF)]), ranges![(-INF, 3)]);
    }
}
