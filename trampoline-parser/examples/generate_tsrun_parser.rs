//! Generate the TypeScript parser for tsrun
//!
//! Run with: cargo run --example generate_tsrun_parser

use trampoline_parser::grammars::typescript_grammar;

fn main() {
    let grammar = typescript_grammar();
    let compiled = grammar.build();
    let code = compiled.generate();

    // Write to stdout so it can be redirected
    println!("{}", code);

    // Also report some stats
    eprintln!("Generated {} lines", code.lines().count());
}
