use std::env;
use std::fs;
use std::path::Path;

mod grammars;

fn main() {
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let out_path = Path::new(&out_dir);

    // Generate all test parsers
    write_parser(out_path, "literal_parser", &grammars::literal().generate());
    write_parser(out_path, "digit_parser", &grammars::digit().generate());
    write_parser(out_path, "number_parser", &grammars::number().generate());
    write_parser(
        out_path,
        "sequence_parser",
        &grammars::sequence().generate(),
    );
    write_parser(out_path, "choice_parser", &grammars::choice().generate());
    write_parser(
        out_path,
        "zero_or_more_parser",
        &grammars::zero_or_more().generate(),
    );
    write_parser(
        out_path,
        "one_or_more_parser",
        &grammars::one_or_more().generate(),
    );
    write_parser(
        out_path,
        "not_followed_parser",
        &grammars::not_followed().generate(),
    );
    write_parser(
        out_path,
        "followed_by_parser",
        &grammars::followed_by().generate(),
    );
    write_parser(out_path, "list_parser", &grammars::list().generate());
    write_parser(
        out_path,
        "list_trailing_parser",
        &grammars::list_trailing().generate(),
    );
    write_parser(
        out_path,
        "arithmetic_parser",
        &grammars::arithmetic().generate(),
    );
    write_parser(out_path, "nested_parser", &grammars::nested().generate());
    write_parser(out_path, "json_parser", &grammars::json().generate());
    write_parser(
        out_path,
        "right_assoc_parser",
        &grammars::right_assoc().generate(),
    );
    write_parser(
        out_path,
        "hex_parser",
        &grammars::char_classes_hex().generate(),
    );
    write_parser(
        out_path,
        "alphanum_parser",
        &grammars::char_classes_alphanum().generate(),
    );
    write_parser(
        out_path,
        "ident_parser",
        &grammars::char_classes_ident().generate(),
    );
    write_parser(
        out_path,
        "lowercase_parser",
        &grammars::char_classes_lowercase().generate(),
    );
    write_parser(
        out_path,
        "uppercase_parser",
        &grammars::char_classes_uppercase().generate(),
    );
    write_parser(
        out_path,
        "custom_range_parser",
        &grammars::char_classes_custom_range().generate(),
    );
    write_parser(
        out_path,
        "optional_parser",
        &grammars::optional_test().generate(),
    );
    write_parser(out_path, "skip_parser", &grammars::skip_test().generate());
    write_parser(
        out_path,
        "keywords_parser",
        &grammars::keywords().generate(),
    );
    write_parser(out_path, "postfix_parser", &grammars::postfix().generate());
    write_parser(out_path, "lua_parser", &grammars::lua().generate());
    write_parser(
        out_path,
        "lua_expr_parser",
        &grammars::lua_expr().generate(),
    );
    write_parser(out_path, "scheme_parser", &grammars::scheme().generate());
    write_parser(
        out_path,
        "backtracking_bad_parser",
        &grammars::backtracking_bad().generate(),
    );
    write_parser(
        out_path,
        "backtracking_good_parser",
        &grammars::backtracking_good().generate(),
    );
    write_parser(
        out_path,
        "generic_call_bad_parser",
        &grammars::generic_call_bad().generate(),
    );
    write_parser(
        out_path,
        "generic_call_good_parser",
        &grammars::generic_call_good().generate(),
    );
    write_parser(
        out_path,
        "generic_call_memoized_parser",
        &grammars::generic_call_memoized().generate(),
    );
    write_parser(
        out_path,
        "generic_call_auto_memoized_parser",
        &grammars::generic_call_auto_memoized().generate(),
    );
    write_parser(
        out_path,
        "sparse_array_parser",
        &grammars::sparse_array().generate(),
    );
    write_parser(
        out_path,
        "pratt_in_list_parser",
        &grammars::pratt_in_list().generate(),
    );
    write_parser(
        out_path,
        "pratt_in_list_postfix_parser",
        &grammars::pratt_in_list_postfix().generate(),
    );
    write_parser(
        out_path,
        "pratt_in_list_ts_parser",
        &grammars::pratt_in_list_ts().generate(),
    );
    write_parser(
        out_path,
        "nested_postfix_parser",
        &grammars::nested_postfix().generate(),
    );

    // Tell Cargo to rerun if trampoline-parser or grammars change
    println!("cargo:rerun-if-changed=../trampoline-parser/src");
    println!("cargo:rerun-if-changed=grammars");
}

fn write_parser(out_path: &Path, name: &str, code: &str) {
    let file_path = out_path.join(format!("{}.rs", name));
    fs::write(&file_path, code).unwrap_or_else(|_| panic!("Failed to write {}", name));
}
