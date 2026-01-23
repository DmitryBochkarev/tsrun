// Build scripts are allowed to panic on error - a failed build script should stop the build
#![allow(clippy::unwrap_used, clippy::expect_used)]

use tsrun_grammar::typescript_grammar;

fn main() {
    let grammar = typescript_grammar();
    let compiled = grammar.build();
    let mut code = compiled.generate();

    // Append parse_program() wrapper method
    code.push_str(PARSE_PROGRAM_IMPL);

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let dest = std::path::Path::new(&manifest_dir).join("src/parser.rs");
    std::fs::write(&dest, code).expect("Failed to write parser.rs");

    println!("cargo:rerun-if-changed=grammar/src/lib.rs");
    println!("cargo:rerun-if-changed=trampoline-parser/src/codegen.rs");
    println!("cargo:rerun-if-changed=build.rs");
}

const PARSE_PROGRAM_IMPL: &str = r#"

impl ParseResult {
    /// Get a combined span for the result
    pub fn combined_span(&self) -> Span {
        self.span()
    }

    /// Convert result to text (for captured tokens)
    pub fn into_text(self) -> JsString {
        match self {
            ParseResult::Text(s, _) => s,
            ParseResult::List(items) => {
                // Combine text from list items
                let mut result = String::new();
                for item in items {
                    if let ParseResult::Text(s, _) = item {
                        result.push_str(s.as_ref());
                    }
                }
                JsString::from(result)
            }
            ParseResult::None => JsString::from(""),
        }
    }

    /// Convert result to a list
    pub fn into_list(self) -> Vec<ParseResult> {
        match self {
            ParseResult::List(items) => items,
            other => vec![other],
        }
    }
}

impl<'a> Parser<'a> {
    /// Parse a complete program, returning the AST
    /// NOTE: AST mapping is not yet implemented in the grammar.
    /// This currently returns a placeholder empty program.
    pub fn parse_program(&mut self) -> Result<Program, ParseError> {
        let _result = self.parse()?;

        // TODO: Once grammar rules have .ast() mappings, convert result to actual AST
        // For now, return empty program to allow build to succeed
        Ok(Program {
            body: Rc::from(vec![]),
            source_type: SourceType::Module,
        })
    }
}
"#;
