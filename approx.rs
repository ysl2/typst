/// An epsilon for approximate floating point calculations.
pub const EPS: f32 = 1.0e-5;

/// Trait for approximate floating point comparisons.
pub trait ApproxEq {
    fn approx_eq(&self, other: &Self) -> bool;
}

impl ApproxEq for f32 {
    fn approx_eq(&self, other: &Self) -> bool {
        (self - other).abs() < EPS
    }
}

impl<T> ApproxEq for Vec<T> where T: ApproxEq {
    fn approx_eq(&self, other: &Self) -> bool {
        self.len() == other.len() &&
        self.iter().zip(other)
            .all(|(x, y)| x.approx_eq(y))
    }
}

impl<T> ApproxEq for [T] where T: ApproxEq {
    fn approx_eq(&self, other: &Self) -> bool {
        self.len() == other.len() &&
        self.iter().zip(other)
            .all(|(x, y)| x.approx_eq(y))
    }
}

impl<T> ApproxEq for Option<T> where T: ApproxEq {
    fn approx_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Some(x), Some(y)) => x.approx_eq(y),
            (None, None) => true,
            _ => false,
        }
    }
}

/// Implements the `ApproxEq` trait for a struct by invoking
/// `approx_eq` on each of the listed fields.
///
/// # Example
/// ```
/// impl_approx_eq!(Point [x, y]);
/// ```
macro_rules! impl_approx_eq {
    ($type:ty [$($field:ident),*]) => {
        impl $crate::geom::ApproxEq for $type {
            fn approx_eq(&self, other: &Self) -> bool {
                $($crate::geom::ApproxEq::approx_eq(
                    &self.$field, &other.$field
                ))&&*
            }
        }
    };
}

/// Ensures that two values are approximately equal per the `ApproxEq` trait.
#[macro_export]
macro_rules! assert_approx_eq {
    ($left:expr, $right:expr $(,)?) => ({
        let (left, right) = ($left, $right);
        if !$crate::geom::ApproxEq::approx_eq(&left, &right) {
            panic!(
                "approximate assertion failed: `(left !~= right)`\n  left: `{:?}`, \n right: `{:?}`",
                left, right,
            );
        }
    });

    ($left:expr, $right:expr, $($arg:tt)+) => ({
        let (left, right) = ($left, $right);
        if !$crate::geom::ApproxEq::approx_eq(&left, &right) {
            panic!(
                "approximate assertion failed: `(left !~= right)`\n  left: `{:?}`,\n right: `{:?}`: {}",
                left, right,
                format_args!($($arg)+),
            );
        }
    });
}
