//! Generated test parsers for trampoline-parser integration tests.
//!
//! Each parser module is generated at build time by build.rs.

#[allow(dead_code, unused_variables, clippy::all)]
pub mod literal_parser {
    include!(concat!(env!("OUT_DIR"), "/literal_parser.rs"));
}

#[allow(dead_code, unused_variables, clippy::all)]
pub mod digit_parser {
    include!(concat!(env!("OUT_DIR"), "/digit_parser.rs"));
}

#[allow(dead_code, unused_variables, clippy::all)]
pub mod number_parser {
    include!(concat!(env!("OUT_DIR"), "/number_parser.rs"));
}

#[allow(dead_code, unused_variables, clippy::all)]
pub mod sequence_parser {
    include!(concat!(env!("OUT_DIR"), "/sequence_parser.rs"));
}

#[allow(dead_code, unused_variables, clippy::all)]
pub mod choice_parser {
    include!(concat!(env!("OUT_DIR"), "/choice_parser.rs"));
}

#[allow(dead_code, unused_variables, clippy::all)]
pub mod zero_or_more_parser {
    include!(concat!(env!("OUT_DIR"), "/zero_or_more_parser.rs"));
}

#[allow(dead_code, unused_variables, clippy::all)]
pub mod one_or_more_parser {
    include!(concat!(env!("OUT_DIR"), "/one_or_more_parser.rs"));
}

#[allow(dead_code, unused_variables, clippy::all)]
pub mod not_followed_parser {
    include!(concat!(env!("OUT_DIR"), "/not_followed_parser.rs"));
}

#[allow(dead_code, unused_variables, clippy::all)]
pub mod followed_by_parser {
    include!(concat!(env!("OUT_DIR"), "/followed_by_parser.rs"));
}

#[allow(dead_code, unused_variables, clippy::all)]
pub mod list_parser {
    include!(concat!(env!("OUT_DIR"), "/list_parser.rs"));
}

#[allow(dead_code, unused_variables, clippy::all)]
pub mod list_trailing_parser {
    include!(concat!(env!("OUT_DIR"), "/list_trailing_parser.rs"));
}

#[allow(dead_code, unused_variables, clippy::all)]
pub mod arithmetic_parser {
    include!(concat!(env!("OUT_DIR"), "/arithmetic_parser.rs"));
}

#[allow(dead_code, unused_variables, clippy::all)]
pub mod nested_parser {
    include!(concat!(env!("OUT_DIR"), "/nested_parser.rs"));
}

#[allow(dead_code, unused_variables, clippy::all)]
pub mod json_parser {
    include!(concat!(env!("OUT_DIR"), "/json_parser.rs"));
}
