//! Built-in function implementations for JavaScript standard library

pub mod array;
pub mod console;
pub mod date;
pub mod error;
pub mod function;
pub mod global;
pub mod internal;
pub mod json;
pub mod map;
pub mod math;
pub mod number;
pub mod object;
pub mod regexp;
pub mod set;
pub mod string;
pub mod symbol;
// TODO: Enable generator after migration to new GC
// pub mod generator;
// pub mod promise; // Old promise, kept for reference
pub mod promise_new; // New promise using new GC

// Re-export public functions from enabled modules
pub use array::*;
pub use console::*;
pub use date::*;
pub use error::*;
pub use function::*;
pub use global::*;
pub use internal::*;
pub use json::*;
pub use map::*;
pub use math::*;
pub use number::*;
pub use object::*;
pub use regexp::*;
pub use set::*;
pub use string::*;
pub use symbol::*;
