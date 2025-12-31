//! Error types for the TypeScript interpreter

use crate::value::Guarded;
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
            write!(
                f,
                "    at {} ({}:{}:{})",
                name,
                file.display(),
                self.line,
                self.column
            )
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

    #[error("TypeError: {message}{}", format_location(.location))]
    TypeError {
        message: String,
        location: Option<SourceLocation>,
    },

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

    /// Error thrown with a JsValue (used for Promise rejection handling)
    #[error("ThrownValue")]
    ThrownValue { guarded: Guarded },

    /// Internal marker for generator yield (not a real error)
    #[error("GeneratorYield")]
    GeneratorYield { guarded: Guarded },

    /// Internal marker for optional chain short-circuit (not a real error)
    /// When a?.b evaluates with a being null/undefined, we need to short-circuit
    /// the entire optional chain (a?.b.c.d should all return undefined)
    #[error("OptionalChainShortCircuit")]
    OptionalChainShortCircuit,
}

fn format_stack(stack: &[StackFrame]) -> String {
    stack
        .iter()
        .map(|f| f.to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_location(location: &Option<SourceLocation>) -> String {
    match location {
        Some(loc) => format!(" at {}", loc),
        None => String::new(),
    }
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
            location: None,
        }
    }

    pub fn type_error_at(message: impl Into<String>, line: u32, column: u32) -> Self {
        JsError::TypeError {
            message: message.into(),
            location: Some(SourceLocation {
                file: None,
                line,
                column,
                length: 1,
            }),
        }
    }

    pub fn reference_error(name: impl Into<String>) -> Self {
        JsError::ReferenceError { name: name.into() }
    }

    pub fn reference_error_with_message(
        name: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        JsError::ReferenceError {
            name: format!("'{}': {}", name.into(), message.into()),
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

    /// Create an internal error for unexpected interpreter states
    /// These should never happen in correctly-written code
    pub fn internal_error(message: impl Into<String>) -> Self {
        JsError::TypeError {
            message: format!("Internal error: {}", message.into()),
            location: None,
        }
    }

    /// Create an error that wraps a thrown JsValue with its guard
    pub fn thrown(guarded: Guarded) -> Self {
        JsError::ThrownValue { guarded }
    }

    /// Extract the JsValue from this error (for Promise rejection handling)
    pub fn to_value(&self) -> crate::value::JsValue {
        match self {
            JsError::ThrownValue { guarded } => guarded.value.clone(),
            JsError::GeneratorYield { guarded } => guarded.value.clone(),
            JsError::TypeError { message, .. } => crate::value::JsValue::String(
                crate::value::JsString::from(format!("TypeError: {}", message)),
            ),
            JsError::ReferenceError { name } => crate::value::JsValue::String(
                crate::value::JsString::from(format!("ReferenceError: {} is not defined", name)),
            ),
            JsError::RangeError { message } => crate::value::JsValue::String(
                crate::value::JsString::from(format!("RangeError: {}", message)),
            ),
            JsError::SyntaxError { message, .. } => crate::value::JsValue::String(
                crate::value::JsString::from(format!("SyntaxError: {}", message)),
            ),
            JsError::RuntimeError { kind, message, .. } => crate::value::JsValue::String(
                crate::value::JsString::from(format!("{}: {}", kind, message)),
            ),
            JsError::ModuleError { message } => crate::value::JsValue::String(
                crate::value::JsString::from(format!("ModuleError: {}", message)),
            ),
            JsError::Internal(msg) => crate::value::JsValue::String(crate::value::JsString::from(
                format!("InternalError: {}", msg),
            )),
            JsError::Thrown => crate::value::JsValue::Undefined,
            // OptionalChainShortCircuit should never escape to user code - it's an internal marker
            JsError::OptionalChainShortCircuit => crate::value::JsValue::Undefined,
        }
    }
}
