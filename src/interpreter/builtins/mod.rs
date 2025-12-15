//! Built-in function implementations for JavaScript standard library

pub mod array;
pub mod error;
pub mod math;
pub mod number;
// TODO: Enable these after migration to new GC
// pub mod console;
// pub mod date;
// pub mod function;
// pub mod generator;
// pub mod global;
// pub mod json;
// pub mod map;
// pub mod object;
// pub mod promise;
// pub mod regexp;
// pub mod set;
// pub mod string;
// pub mod symbol;

// Re-export public functions from enabled modules
pub use array::*;
pub use error::*;
pub use math::*;
pub use number::*;
