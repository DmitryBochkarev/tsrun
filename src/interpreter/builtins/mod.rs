//! Built-in function implementations for JavaScript standard library

pub mod array;
pub mod console;
pub mod error;
pub mod function;
pub mod json;
pub mod math;
pub mod number;
pub mod object;
pub mod regexp;
pub mod string;
// TODO: Enable these after migration to new GC
// pub mod date;
// pub mod generator;
// pub mod global;
// pub mod map;
// pub mod promise;
// pub mod set;
// pub mod symbol;

// Re-export public functions from enabled modules
pub use array::*;
pub use console::*;
pub use error::*;
pub use function::*;
pub use json::*;
pub use math::*;
pub use number::*;
pub use object::*;
pub use regexp::*;
pub use string::*;
