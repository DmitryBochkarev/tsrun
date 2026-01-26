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

#[allow(dead_code, unused_variables, clippy::all)]
pub mod right_assoc_parser {
    include!(concat!(env!("OUT_DIR"), "/right_assoc_parser.rs"));
}

#[allow(dead_code, unused_variables, clippy::all)]
pub mod hex_parser {
    include!(concat!(env!("OUT_DIR"), "/hex_parser.rs"));
}

#[allow(dead_code, unused_variables, clippy::all)]
pub mod alphanum_parser {
    include!(concat!(env!("OUT_DIR"), "/alphanum_parser.rs"));
}

#[allow(dead_code, unused_variables, clippy::all)]
pub mod ident_parser {
    include!(concat!(env!("OUT_DIR"), "/ident_parser.rs"));
}

#[allow(dead_code, unused_variables, clippy::all)]
pub mod lowercase_parser {
    include!(concat!(env!("OUT_DIR"), "/lowercase_parser.rs"));
}

#[allow(dead_code, unused_variables, clippy::all)]
pub mod uppercase_parser {
    include!(concat!(env!("OUT_DIR"), "/uppercase_parser.rs"));
}

#[allow(dead_code, unused_variables, clippy::all)]
pub mod custom_range_parser {
    include!(concat!(env!("OUT_DIR"), "/custom_range_parser.rs"));
}

#[allow(dead_code, unused_variables, clippy::all)]
pub mod optional_parser {
    include!(concat!(env!("OUT_DIR"), "/optional_parser.rs"));
}

#[allow(dead_code, unused_variables, clippy::all)]
pub mod skip_parser {
    include!(concat!(env!("OUT_DIR"), "/skip_parser.rs"));
}

#[allow(dead_code, unused_variables, clippy::all)]
pub mod keywords_parser {
    include!(concat!(env!("OUT_DIR"), "/keywords_parser.rs"));
}

#[allow(dead_code, unused_variables, clippy::all)]
pub mod postfix_parser {
    include!(concat!(env!("OUT_DIR"), "/postfix_parser.rs"));
}

#[allow(dead_code, unused_variables, clippy::all)]
pub mod lua_parser {
    include!(concat!(env!("OUT_DIR"), "/lua_parser.rs"));
}

#[allow(dead_code, unused_variables, clippy::all)]
pub mod lua_expr_parser {
    include!(concat!(env!("OUT_DIR"), "/lua_expr_parser.rs"));
}

#[allow(dead_code, unused_variables, clippy::all)]
pub mod scheme_parser {
    include!(concat!(env!("OUT_DIR"), "/scheme_parser.rs"));
}

#[allow(dead_code, unused_variables, clippy::all)]
pub mod backtracking_bad_parser {
    include!(concat!(env!("OUT_DIR"), "/backtracking_bad_parser.rs"));
}

#[allow(dead_code, unused_variables, clippy::all)]
pub mod backtracking_good_parser {
    include!(concat!(env!("OUT_DIR"), "/backtracking_good_parser.rs"));
}

#[allow(dead_code, unused_variables, clippy::all)]
pub mod generic_call_bad_parser {
    include!(concat!(env!("OUT_DIR"), "/generic_call_bad_parser.rs"));
}

#[allow(dead_code, unused_variables, clippy::all)]
pub mod generic_call_good_parser {
    include!(concat!(env!("OUT_DIR"), "/generic_call_good_parser.rs"));
}

#[allow(dead_code, unused_variables, clippy::all)]
pub mod generic_call_memoized_parser {
    include!(concat!(env!("OUT_DIR"), "/generic_call_memoized_parser.rs"));
}

#[allow(dead_code, unused_variables, clippy::all)]
pub mod generic_call_auto_memoized_parser {
    include!(concat!(
        env!("OUT_DIR"),
        "/generic_call_auto_memoized_parser.rs"
    ));
}

#[allow(dead_code, unused_variables, clippy::all)]
pub mod sparse_array_parser {
    include!(concat!(env!("OUT_DIR"), "/sparse_array_parser.rs"));
}

#[allow(dead_code, unused_variables, clippy::all)]
pub mod pratt_in_list_parser {
    include!(concat!(env!("OUT_DIR"), "/pratt_in_list_parser.rs"));
}

#[allow(dead_code, unused_variables, clippy::all)]
pub mod pratt_in_list_postfix_parser {
    include!(concat!(env!("OUT_DIR"), "/pratt_in_list_postfix_parser.rs"));
}

#[allow(dead_code, unused_variables, clippy::all)]
pub mod pratt_in_list_ts_parser {
    include!(concat!(env!("OUT_DIR"), "/pratt_in_list_ts_parser.rs"));
}

#[allow(dead_code, unused_variables, clippy::all)]
pub mod nested_postfix_parser {
    include!(concat!(env!("OUT_DIR"), "/nested_postfix_parser.rs"));
}
