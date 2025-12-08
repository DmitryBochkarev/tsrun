//! Error types for the TypeScript interpreter

use std::path::PathBuf;
use thiserror::Error;

/// Source location information for error messages
#[derive(Debug, Clone, PartialEq)]
pub struct SourceLocation {
    pub file: Option<PathBuf>,
    pub line: u32,
    pub column: u32,
    pub length: u32,
}

impl std::fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(file) = &self.file {
            write!(f, "{}:{}:{}", file.display(), self.line, self.column)
        } else {
            write!(f, "{}:{}", self.line, self.column)
        }
    }
}

/// Stack frame for error traces
#[derive(Debug, Clone)]
pub struct StackFrame {
    pub function_name: Option<String>,
    pub file: Option<PathBuf>,
    pub line: u32,
    pub column: u32,
}

impl std::fmt::Display for StackFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = self.function_name.as_deref().unwrap_or("<anonymous>");
        if let Some(file) = &self.file {
            write!(f, "    at {} ({}:{}:{})", name, file.display(), self.line, self.column)
        } else {
            write!(f, "    at {} (<eval>:{}:{})", name, self.line, self.column)
        }
    }
}

/// Main error type for the interpreter
#[derive(Debug, Error)]
pub enum JsError {
    #[error("SyntaxError: {message} at {location}")]
    SyntaxError {
        message: String,
        location: SourceLocation,
    },

    #[error("TypeError: {message}")]
    TypeError { message: String },

    #[error("ReferenceError: {name} is not defined")]
    ReferenceError { name: String },

    #[error("RangeError: {message}")]
    RangeError { message: String },

    #[error("{kind}: {message}\n{}", format_stack(stack))]
    RuntimeError {
        kind: String,
        message: String,
        stack: Vec<StackFrame>,
    },

    #[error("ModuleError: {message}")]
    ModuleError { message: String },

    #[error("Internal error: {0}")]
    Internal(String),

    /// Marker error indicating a value was thrown (actual value stored in interpreter)
    #[error("Thrown")]
    Thrown,
}

fn format_stack(stack: &[StackFrame]) -> String {
    stack.iter().map(|f| f.to_string()).collect::<Vec<_>>().join("\n")
}

impl JsError {
    pub fn syntax_error(message: impl Into<String>, line: u32, column: u32) -> Self {
        JsError::SyntaxError {
            message: message.into(),
            location: SourceLocation {
                file: None,
                line,
                column,
                length: 1,
            },
        }
    }

    /// Create a syntax error without location info (for internal use during parsing)
    pub fn syntax_error_simple(message: impl Into<String>) -> Self {
        JsError::SyntaxError {
            message: message.into(),
            location: SourceLocation {
                file: None,
                line: 0,
                column: 0,
                length: 0,
            },
        }
    }

    pub fn type_error(message: impl Into<String>) -> Self {
        JsError::TypeError {
            message: message.into(),
        }
    }

    pub fn reference_error(name: impl Into<String>) -> Self {
        JsError::ReferenceError { name: name.into() }
    }

    pub fn reference_error_with_message(name: impl Into<String>, message: impl Into<String>) -> Self {
        JsError::ReferenceError {
            name: format!("'{}': {}", name.into(), message.into())
        }
    }

    pub fn range_error(message: impl Into<String>) -> Self {
        JsError::RangeError {
            message: message.into(),
        }
    }

    pub fn module_error(message: impl Into<String>) -> Self {
        JsError::ModuleError {
            message: message.into(),
        }
    }
}
