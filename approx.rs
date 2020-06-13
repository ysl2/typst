/// Trait for approximate floating point comparisons.
pub trait ApproxEq {
    fn approx_eq(&self, other: &Self, tolerance: f32) -> bool;
}

impl ApproxEq for f32 {
    fn approx_eq(&self, other: &Self, tolerance: f32) -> bool {
        (self - other).abs() < tolerance
    }
}

impl<T> ApproxEq for Vec<T> where T: ApproxEq {
    fn approx_eq(&self, other: &Self, tolerance: f32) -> bool {
        self.len() == other.len() &&
        self.iter().zip(other)
            .all(|(x, y)| x.approx_eq(y, tolerance))
    }
}

impl<T> ApproxEq for [T] where T: ApproxEq {
    fn approx_eq(&self, other: &Self, tolerance: f32) -> bool {
        self.len() == other.len() &&
        self.iter().zip(other)
            .all(|(x, y)| x.approx_eq(y, tolerance))
    }
}

impl<T> ApproxEq for Option<T> where T: ApproxEq {
    fn approx_eq(&self, other: &Self, tolerance: f32) -> bool {
        match (self, other) {
            (Some(x), Some(y)) => x.approx_eq(y, tolerance),
            (None, None) => true,
            _ => false,
        }
    }
}

/// Implements the `ApproxEq` trait for a struct by invoking
/// `approx_eq` on each of the listed fields.
macro_rules! impl_approx_eq {
    ($type:ty [$($field:ident),*]) => {
        impl $crate::geom::ApproxEq for $type {
            fn approx_eq(&self, other: &Self, tolerance: f32) -> bool {
                $($crate::geom::ApproxEq::approx_eq(
                    &self.$field, &other.$field, tolerance
                ))&&*
            }
        }
    };
}

/// Ensures that two values are approximately equal by the `ApproxEq` trait.
///
/// The default tolerance is `1e-5`, but it can be changed through a keyword
/// argument.
///
/// # Examples
/// These comparisons work out fine:
/// ```
/// # use layr::assert_approx_eq;
/// assert_approx_eq!(1.0, 1.00000001);
/// assert_approx_eq!(1.0, 1.2, tolerance = 0.3);
/// ```
///
/// Whereas this one will panic:
/// ```should_panic
/// # use layr::assert_approx_eq;
/// # let boom = "";
/// assert_approx_eq!(1.0, 1.2, "a problem has been detected: {}", boom);
/// ```
#[macro_export]
macro_rules! assert_approx_eq {
    ($left:expr, $right:expr, tolerance = $t:expr $(,)?) => {{
        let (left, right) = (&$left, &$right);
        if !$crate::geom::ApproxEq::approx_eq(left, right, $t) {
            panic!(
                "approximate assertion failed:\n  left: `{:?}`,\n right: `{:?}`",
                left, right,
            );
        }
    }};

    ($left:expr, $right:expr $(,)?) => {
        assert_approx_eq!($left, $right, tolerance = 1e-5);
    };

    ($left:expr, $right:expr, tolerance = $t:expr, $($arg:tt)+) => {{
        let (left, right) = (&$left, &$right);
        if !$crate::geom::ApproxEq::approx_eq(left, right, $t) {
            panic!(
                "approximate assertion failed:\n  left: `{:?}`,\n right: `{:?}`: {}",
                left, right,
                format_args!($($arg)+),
            );
        }
    }};

    ($left:expr, $right:expr, $($arg:tt)+) => {
        assert_approx_eq!($left, $right, tolerance = 1e-5, $($arg)+);
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_macro_works_basic_when_approx_equal() {
        assert_approx_eq!(1.2, 1.2000000001);
        assert_approx_eq!(1.2, 1.2000000001,);
    }

    #[test]
    #[should_panic(expected = "approximate assertion failed:\n  left: `1.2`,\n right: `1.3`")]
    fn test_macro_works_basic_when_not_approx_equal() {
        assert_approx_eq!(1.2, 1.3);
    }

    #[test]
    fn test_macro_works_with_tolerance_when_approx_equal() {
        assert_approx_eq!(1.5, 2.0, tolerance = 0.7);
        assert_approx_eq!(1.5, 2.0, tolerance = 0.7,);
    }

    #[test]
    #[should_panic(expected = "approximate assertion failed:\n  left: `1.5`,\n right: `2.5`")]
    fn test_macro_works_with_tolerance_when_not_approx_equal() {
        assert_approx_eq!(1.5, 2.5, tolerance = 0.7);
    }

    #[test]
    #[should_panic(expected = "approximate assertion failed:\n  left: `1.5`,\n right: `2.0`: this is okay")]
    fn test_macro_works_with_message() {
        assert_approx_eq!(1.5, 2.0, "{} is okay", "this");
    }

    #[test]
    #[should_panic(expected = "approximate assertion failed:\n  left: `1.5`,\n right: `2.0`: this is okay")]
    fn test_macro_works_with_message_and_tolerance() {
        assert_approx_eq!(1.5, 2.0, tolerance = 0.3, "{} is okay", "this");
    }
}
