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
    write_parser(out_path, "sequence_parser", &grammars::sequence().generate());
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

    // Tell Cargo to rerun if trampoline-parser or grammars change
    println!("cargo:rerun-if-changed=../trampoline-parser/src");
    println!("cargo:rerun-if-changed=grammars");
}

fn write_parser(out_path: &Path, name: &str, code: &str) {
    let file_path = out_path.join(format!("{}.rs", name));
    fs::write(&file_path, code).unwrap_or_else(|_| panic!("Failed to write {}", name));
}
