//! Grammar definitions for test parsers.
//!
//! Each module defines a grammar that gets compiled at build time.

mod arithmetic;
mod choice;
mod digit;
mod followed_by;
mod json;
mod list;
mod list_trailing;
mod literal;
mod nested;
mod not_followed;
mod number;
mod one_or_more;
mod sequence;
mod zero_or_more;

pub use arithmetic::grammar as arithmetic;
pub use choice::grammar as choice;
pub use digit::grammar as digit;
pub use followed_by::grammar as followed_by;
pub use json::grammar as json;
pub use list::grammar as list;
pub use list_trailing::grammar as list_trailing;
pub use literal::grammar as literal;
pub use nested::grammar as nested;
pub use not_followed::grammar as not_followed;
pub use number::grammar as number;
pub use one_or_more::grammar as one_or_more;
pub use sequence::grammar as sequence;
pub use zero_or_more::grammar as zero_or_more;
