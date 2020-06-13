use std::cmp::{PartialEq, Eq};
use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use super::{value_no_nans, Length};

/// A range (_start_ / _end_) along an axis.
#[derive(Copy, Clone, PartialEq)]
pub struct Range {
    pub start: Length,
    pub end: Length,
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
    pub const ZERO: Range = Range {
        start: Length::ZERO,
        end: Length::ZERO,
    };

    /// Create a new range from `start` and `end`.
    pub fn new(start: Length, end: Length) -> Range {
        Range { start, end }
    }

    /// The distance between start and end.
    pub fn size(self) -> Length {
        self.end - self.start
    }

    /// The mid-point of start and end.
    pub fn mid(self) -> Length {
        (self.start + self.end) / 2.0
    }

    /// Whether the range is finite (both start and end are finite).
    pub fn is_finite(self) -> bool {
        self.start.to_pt().is_finite() && self.end.to_pt().is_finite()
    }

    /// The region of `v` relative to the range.
    pub fn region(self, v: Length) -> Region {
        if v < self.start {
            Region::Before
        } else if v > self.end {
            Region::After
        } else {
            Region::Inside
        }
    }

    /// This range shrunk by an amount at start and end.
    pub fn shrunk(self, less_start: Length, less_end: Length) -> Range {
        Range {
            start: self.start + less_start,
            end: self.end - less_end,
        }
    }

    /// This range extended by an extra amount at start and end.
    pub fn extended(self, extra_start: Length, extra_end: Length) -> Range {
        Range {
            start: self.start - extra_start,
            end: self.end + extra_end,
        }
    }

    /// Simplifies a set of ranges into a run of non-overlapping intervals.
    pub fn simplify(mut ranges: Vec<Range>) -> Vec<Range> {
        ranges.sort_by(|a, b| value_no_nans(&a.start, &b.start));

        let mut out: Vec<Range> = vec![];

        for range in ranges {
            if let Some(prev) = out.last_mut() {
                if range.start <= prev.end {
                    prev.end.make_max(range.end);
                    continue;
                }
            }

            out.push(range);
        }

        out
    }

    /// Returns the inverse run of a run of non-overlapping intervals.
    pub fn inverse(ranges: &[Range]) -> Vec<Range> {
        let mut out = vec![];

        if ranges.is_empty() {
            out.push(Range::new(Length::NEG_INF, Length::INF));
        }

        if let Some(range) = ranges.first() {
            if range.start > Length::NEG_INF {
                out.push(Range::new(Length::NEG_INF, range.start));
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
            if range.end < Length::INF {
                out.push(Range::new(range.end, Length::INF));
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
            let mut start = Length::NEG_INF;
            let mut end = Length::INF;
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

    /// Shrinks all intervals by the given values and removes disappearing
    /// intervals.
    pub fn shrink(mut ranges: Vec<Range>, start: Length, end: Length) -> Vec<Range> {
        for range in &mut ranges {
            range.start += start;
            range.end -= end;
        }

        ranges.retain(|r| r.size() > Length::ZERO);
        ranges
    }
}

impl_approx_eq!(Range [start, end]);

impl Debug for Range {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "[{} .. {}]", self.start, self.end)
    }
}

/// A hashable `Range` which hashes to the same value for very close ranges (at
/// least most of the time, if you're unlucky the ranges _just_ fall into
/// different slots).
#[derive(Debug, Copy, Clone)]
pub struct RangeKey(pub Range);

impl Hash for RangeKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        key(self.0).hash(state);
    }
}

impl PartialEq for RangeKey {
    fn eq(&self, other: &Self) -> bool {
        key(self.0) == key(other.0)
    }
}

impl Eq for RangeKey {}

fn key(range: Range) -> (i32, i32) {
    (slot(range.start), slot(range.end))
}

fn slot(x: Length) -> i32 {
    (x.to_pt() * 1000.0).round() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    const INF: f32 = f32::INFINITY;

    macro_rules! ranges {
        ($(($a:expr, $b:expr)),* $(,)?) => {
            vec![$(Range {
                start: Length::pt($a as f32),
                end: Length::pt($b as f32)
            }),*]
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
            Range::intersect(&[
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
            Range::inverse(&ranges![(-3, 5), (8, 11), (11, 12)]),
            ranges![(-INF, -3), (5, 8), (12, INF)]
        );
    }

    #[test]
    fn test_inverse_of_empty_is_everything() {
        assert_eq!(Range::inverse(&[]), ranges![(-INF, INF)]);
    }

    #[test]
    fn test_inverse_of_infinite_in_one_dir_is_infinite_in_other_dir() {
        assert_eq!(Range::inverse(&ranges![(3, INF)]), ranges![(-INF, 3)]);
    }
}
