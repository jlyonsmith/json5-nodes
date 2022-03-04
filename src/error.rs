use crate::Rule; // From the pest grammar
use std::fmt::{self, Display};

/// A location within the JSON5 file
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Location {
  /// The one-based line number of the error.
  pub line: usize,
  /// The one-based column number of the error.
  pub column: usize,
}

/// This crates error enum
#[derive(Clone, Debug, PartialEq)]
pub enum JsonError {
  /// Errors caused by a bad parse
  Syntax(String, Option<Location>),
  /// Errors caused by badly formatted numbers
  NumberFormat(Option<Location>),
  /// Errors caused by numbers out of range
  NumberRange(Option<Location>),
  /// Errors caused by bad Unicode
  Unicode(Option<Location>),
}

impl From<pest::error::Error<Rule>> for JsonError {
  fn from(err: pest::error::Error<Rule>) -> Self {
    let (line, column) = match err.line_col {
      pest::error::LineColLocation::Pos((l, c)) => (l, c),
      pest::error::LineColLocation::Span((l, c), (_, _)) => (l, c),
    };
    JsonError::Syntax(err.to_string(), Some(Location { line, column }))
  }
}

impl Display for JsonError {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      JsonError::Syntax(ref msg, _) => write!(formatter, "{}", msg),
      JsonError::NumberFormat(_) => write!(formatter, "bad number format"),
      JsonError::NumberRange(_) => write!(formatter, "bad number range"),
      JsonError::Unicode(_) => write!(formatter, "bad Unicode characters"),
    }
  }
}

impl std::error::Error for JsonError {}
