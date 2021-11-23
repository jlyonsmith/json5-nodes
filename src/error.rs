use crate::Rule; // From the pest grammar
use std::fmt::{self, Display};

/// One-based line and column at which the error was detected.
#[derive(Clone, Debug, PartialEq)]
pub struct Location {
  /// The one-based line number of the error.
  pub line: usize,
  /// The one-based column number of the error.
  pub column: usize,
}

/// A bare bones error type which currently just collapses all the underlying errors in to a single
/// string... This is fine for displaying to the user, but not very useful otherwise. Work to be
/// done here.
#[derive(Clone, Debug, PartialEq)]
pub enum Error {
  /// Errors caused by bad syntax
  BadSyntax {
    /// The error message.
    msg: String,
    /// The location of the error, if applicable.
    location: Option<Location>,
  },
}

impl From<pest::error::Error<Rule>> for Error {
  fn from(err: pest::error::Error<Rule>) -> Self {
    let (line, column) = match err.line_col {
      pest::error::LineColLocation::Pos((l, c)) => (l, c),
      pest::error::LineColLocation::Span((l, c), (_, _)) => (l, c),
    };
    Error::BadSyntax {
      msg: err.to_string(),
      location: Some(Location { line, column }),
    }
  }
}

impl Display for Error {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Error::BadSyntax { ref msg, .. } => write!(formatter, "{}", msg),
    }
  }
}

impl std::error::Error for Error {}
