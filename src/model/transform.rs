use std::ops::Add;

use super::StyleMap;

/// A composable representation of styling and transformation.
#[derive(Debug, PartialEq, Clone, Hash)]
pub struct Transform(pub StyleMap);

impl Add for Transform {
    type Output = Self;

    fn add(self, mut rhs: Self) -> Self::Output {
        rhs.0.apply_map(&self.0);
        rhs
    }
}
