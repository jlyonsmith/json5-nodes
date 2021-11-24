mod error;

pub use error::{Error, Location};

use pest::{iterators::Pair, Parser, Span};
use pest_derive::Parser;
use std::collections::BTreeMap;

impl From<&Span<'_>> for Location {
    fn from(s: &Span<'_>) -> Self {
        let (line, column) = s.start_pos().line_col();
        Self { line, column }
    }
}

/// AST node type
#[derive(Clone, Debug, PartialEq)]
pub enum JsonNode<'a> {
    Null(Option<Location>),
    Bool(bool, Option<Location>),
    Number(f64, Option<Location>),
    String(&'a str, Option<Location>),
    Array(Vec<JsonNode<'a>>, Option<Location>),
    Object(BTreeMap<&'a str, JsonNode<'a>>, Option<Location>),
}

/// JSON5 parser
#[derive(Parser)]
#[grammar = "json5.pest"]
struct Json5Parser;

pub fn parse<'a>(input: &'a str) -> Result<JsonNode<'a>, Error> {
    fn parse_pair<'a>(pair: Pair<'a, Rule>) -> Result<JsonNode, Error> {
        let span = pair.as_span();
        let location = Some(Location::from(&span));
        let node: JsonNode = match pair.as_rule() {
            Rule::null => JsonNode::Null(location),
            Rule::boolean => JsonNode::Bool(pair.as_str().parse().unwrap(), location),
            Rule::string | Rule::identifier => {
                JsonNode::String(pair.into_inner().next().unwrap().as_str(), location)
            }
            Rule::number => JsonNode::Number(pair.as_str().parse().unwrap(), location),
            Rule::array => JsonNode::Array(
                pair.into_inner()
                    .map(parse_pair)
                    .collect::<Result<Vec<_>, _>>()?,
                location,
            ),
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

                JsonNode::Object(map, location)
            }
            _ => unreachable!(),
        };

        Ok(node)
    }

    parse_pair(Json5Parser::parse(Rule::text, input)?.next().unwrap())
}

pub fn stringify(node: &JsonNode) -> String {
    use JsonNode::*;

    match node {
        Object(o, _) => {
            let contents: Vec<_> = o
                .iter()
                .map(|(name, value)| format!("\"{}\":{}", name, stringify(value)))
                .collect();
            format!("{{{}}}", contents.join(","))
        }
        Array(a, _) => {
            let contents: Vec<_> = a.iter().map(stringify).collect();
            format!("[{}]", contents.join(","))
        }
        String(s, _) => format!("\"{}\"", s),
        Number(n, _) => format!("{}", n),
        Bool(b, _) => format!("{}", b),
        Null(_) => format!("null"),
    }
}
