use pest::{error::Error, iterators::Pair, Parser, Span};
use pest_derive::Parser;
use std::collections::BTreeMap;

/// AST node type
#[derive(Clone, Debug)]
pub enum JsonValue<'a> {
    Null,
    Bool(bool),
    Number(f64),
    String(&'a str),
    Array(Vec<JsonNode<'a>>),
    Object(BTreeMap<&'a str, JsonNode<'a>>),
}

/// One-based line and column at which the error was detected.
#[derive(Clone, Debug, PartialEq)]
pub struct Location {
    /// The one-based line number of the error.
    pub line: usize,
    /// The one-based column number of the error.
    pub column: usize,
}

/// The AST node structure
#[derive(Clone, Debug)]
pub struct JsonNode<'a> {
    pub value: JsonValue<'a>,
    pub location: Option<Location>,
}

impl From<&Span<'_>> for Location {
    fn from(s: &Span<'_>) -> Self {
        let (line, column) = s.start_pos().line_col();
        Self { line, column }
    }
}

#[derive(Parser)]
#[grammar = "json5.pest"]
struct Json5Parser;

pub fn from_str<'a>(input: &'a str) -> Result<JsonNode, Error<Rule>> {
    parse_pair(Json5Parser::parse(Rule::text, input)?.next().unwrap())
}

fn parse_pair<'a>(pair: Pair<'a, Rule>) -> Result<JsonNode, Error<Rule>> {
    let span = pair.as_span();
    let location = Some(Location::from(&span));
    let node: JsonNode = match pair.as_rule() {
        Rule::null => JsonNode {
            value: JsonValue::Null,
            location,
        },
        Rule::boolean => JsonNode {
            value: JsonValue::Bool(pair.as_str().parse().unwrap()),
            location,
        },
        Rule::string | Rule::identifier => JsonNode {
            value: JsonValue::String(pair.into_inner().next().unwrap().as_str()),
            location,
        },
        Rule::number => JsonNode {
            value: JsonValue::Number(pair.as_str().parse().unwrap()),
            location,
        },
        Rule::array => JsonNode {
            value: JsonValue::Array(
                pair.into_inner()
                    .map(parse_pair)
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            location,
        },
        Rule::object => {
            let mut map: BTreeMap<&'a str, JsonNode<'a>> = BTreeMap::new();

            for pair in pair.into_inner() {
                let mut inner_rules = pair.into_inner();
                let name = inner_rules
                    .next()
                    .unwrap()
                    .into_inner()
                    .next()
                    .unwrap()
                    .as_str();
                let value = parse_pair(inner_rules.next().unwrap())?;

                map.insert(name, value);
            }

            JsonNode {
                value: JsonValue::Object(map),
                location,
            }
        }
        _ => unreachable!(),
    };

    Ok(node)
}
