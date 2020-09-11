//! The standard library.

// mod align;
// mod boxed;
mod color;
mod font;
// mod page;
// mod spacing;

// pub use align::*;
// pub use boxed::*;
pub use color::*;
pub use font::*;
// pub use page::*;
// pub use spacing::*;

use crate::eval::Scope;
use crate::prelude::*;

macro_rules! std {
    ($($func:ident $([$name:expr])?),* $(,)?) => {
        /// Create a scope with all standard library functions.
        pub fn _std() -> Scope {
            let mut std = Scope::new();
            $({
                let name = std!(@name $func $([$name])?);
                std.insert(name, FuncValue::new($func));
            })*
            std
        }
    };

    (@name $func:ident) => { stringify!($func) };
    (@name $func:ident [$name:expr]) => { $name };
}

std! {
    // align,
    // boxed ["box"],
    font,
    // h,
    // page,
    // pagebreak,
    rgb,
    // v,
}
