// Build scripts are allowed to panic on error - a failed build script should stop the build
#![allow(clippy::unwrap_used, clippy::expect_used)]

use tsrun_grammar::typescript_grammar;

fn main() {
    let grammar = typescript_grammar();
    let compiled = grammar.build_optimized();
    let mut code = compiled.generate();

    // Append parse_program() wrapper method
    code.push_str(PARSE_PROGRAM_IMPL);

    // Write to OUT_DIR instead of src/ - better for generated code
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dest = std::path::Path::new(&out_dir).join("parser_generated.rs");
    std::fs::write(&dest, code).expect("Failed to write parser_generated.rs");

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
            // AST variants don't contain text
            ParseResult::Expr(_) | ParseResult::Stmt(_) | ParseResult::Ident(_)
            | ParseResult::Pat(_) | ParseResult::Prog(_)
            | ParseResult::ClassBody(_) | ParseResult::ClassMember(_)
            | ParseResult::SwitchCase(_) => JsString::from(""),
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
    pub fn parse_program(&mut self) -> Result<Program, ParseError> {
        let result = self.parse()?;

        // Extract Program from the parse result
        match result {
            ParseResult::Prog(program) => Ok(program),
            ParseResult::List(items) => {
                // If we got a list, try to convert items to statements
                let mut statements = Vec::new();
                for item in items {
                    match item {
                        ParseResult::Stmt(stmt) => statements.push(stmt),
                        ParseResult::Prog(prog) => return Ok(prog),
                        _ => {} // Skip non-statement items
                    }
                }
                Ok(Program {
                    body: Rc::from(statements),
                    source_type: SourceType::Module,
                })
            }
            _ => {
                // Fallback: return empty program
                Ok(Program {
                    body: Rc::from(vec![]),
                    source_type: SourceType::Module,
                })
            }
        }
    }
}
"#;
