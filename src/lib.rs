use pest::Parser as P;
use pest::Span;
use pest_derive::Parser;

// Alias for a `Result` with error type `json5::Error`
pub type Result<T> = std::result::Result<T, Error>;

/// One-based line and column at which the error was detected.
#[derive(Clone, Debug, PartialEq)]
pub struct Location {
    /// The one-based line number of the error.
    pub line: usize,
    /// The one-based column number of the error.
    pub column: usize,
}

impl From<&Span<'_>> for Location {
    fn from(s: &Span<'_>) -> Self {
        let (line, column) = s.start_pos().line_col();
        Self { line, column }
    }
}

impl From<pest::error::Error<Rule>> for Error {
    fn from(err: pest::error::Error<Rule>) -> Self {
        let (line, column) = match err.line_col {
            pest::error::LineColLocation::Pos((l, c)) => (l, c),
            pest::error::LineColLocation::Span((l, c), (_, _)) => (l, c),
        };
        Error::Message {
            msg: err.to_string(),
            location: Some(Location { line, column }),
        }
    }
}

/// A bare bones error type which currently just collapses all the underlying errors in to a single
/// string.
#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    Message {
        /// The error message.
        msg: String,
        /// The location of the error, if applicable.
        location: Option<Location>,
    },
}

#[derive(Parser)]
#[grammar = "json5.pest"]
struct Parser;

pub fn parse_nodes(input: &str) -> Result<()> {
    Parser::parse(Rule::text, input)?.next().unwrap();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_nodes() {
        parse_nodes("{}").unwrap();
    }
}
