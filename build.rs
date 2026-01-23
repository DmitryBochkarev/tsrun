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

impl<'a> Parser<'a> {
    /// Parse a complete program, returning the AST
    pub fn parse_program(&mut self) -> Result<Program, ParseError> {
        let result = self.parse()?;

        // Convert ParseResult to Program AST
        match result {
            ParseResult::List(items) => {
                // Program rule returns a list of statements - need to convert
                let statements = convert_statements(items)?;
                Ok(Program {
                    body: Rc::from(statements),
                    source_type: SourceType::Module,
                })
            }
            other => Err(ParseError {
                span: other.combined_span(),
                expected: vec![],
                found: format!("Expected Program, got {:?}", std::mem::discriminant(&other)),
            }),
        }
    }
}

/// Convert a list of ParseResults to Statements
fn convert_statements(items: Vec<ParseResult>) -> Result<Vec<Statement>, ParseError> {
    let mut statements = Vec::new();
    for item in items {
        let stmt = convert_to_statement(item)?;
        statements.push(stmt);
    }
    Ok(statements)
}

/// Convert a ParseResult to a Statement
fn convert_to_statement(result: ParseResult) -> Result<Statement, ParseError> {
    match result {
        ParseResult::Stmt(stmt) => Ok(stmt),
        ParseResult::Expr(expr) => Ok(Statement::Expression(ExpressionStatement {
            expression: Rc::new(expr),
            span: Span::default(),
        })),
        // Unwrap rule variants that contain the actual AST
        ParseResult::VariableDeclaration(inner) => convert_to_statement(*inner),
        ParseResult::Statement(inner) => convert_to_statement(*inner),
        ParseResult::ExpressionStatement(inner) => convert_to_statement(*inner),
        ParseResult::IfStatement(inner) => convert_to_statement(*inner),
        ParseResult::ForStatement(inner) => convert_to_statement(*inner),
        ParseResult::ForInStatement(inner) => convert_to_statement(*inner),
        ParseResult::ForOfStatement(inner) => convert_to_statement(*inner),
        ParseResult::WhileStatement(inner) => convert_to_statement(*inner),
        ParseResult::DoWhileStatement(inner) => convert_to_statement(*inner),
        ParseResult::SwitchStatement(inner) => convert_to_statement(*inner),
        ParseResult::TryStatement(inner) => convert_to_statement(*inner),
        ParseResult::BlockStatement(inner) => convert_to_statement(*inner),
        ParseResult::ReturnStatement(inner) => convert_to_statement(*inner),
        ParseResult::BreakStatement(inner) => convert_to_statement(*inner),
        ParseResult::ContinueStatement(inner) => convert_to_statement(*inner),
        ParseResult::ThrowStatement(inner) => convert_to_statement(*inner),
        ParseResult::DebuggerStatement(inner) => convert_to_statement(*inner),
        ParseResult::LabeledStatement(inner) => convert_to_statement(*inner),
        ParseResult::EmptyStatement(inner) => convert_to_statement(*inner),
        ParseResult::FunctionDeclaration(inner) => convert_to_statement(*inner),
        ParseResult::ClassDeclaration(inner) => convert_to_statement(*inner),
        ParseResult::ImportDeclaration(inner) => convert_to_statement(*inner),
        ParseResult::ExportDeclaration(inner) => convert_to_statement(*inner),
        ParseResult::TypeAliasDeclaration(inner) => convert_to_statement(*inner),
        ParseResult::InterfaceDeclaration(inner) => convert_to_statement(*inner),
        ParseResult::EnumDeclaration(inner) => convert_to_statement(*inner),
        ParseResult::NamespaceDeclaration(inner) => convert_to_statement(*inner),
        // For now, return a placeholder for non-AST results
        // This will be removed once all rules have .ast() closures
        other => Err(ParseError {
            span: other.combined_span(),
            expected: vec![],
            found: format!("Statement conversion not yet implemented for {:?}", std::mem::discriminant(&other)),
        }),
    }
}
"#;
