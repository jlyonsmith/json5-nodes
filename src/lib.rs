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

/// JSON5 node type which holds a value and it's location in the input string
#[derive(Clone, Debug, PartialEq)]
pub enum JsonNode {
    Null(Option<Location>),
    Bool(bool, Option<Location>),
    Number(f64, Option<Location>),
    String(String, Option<Location>),
    Array(Vec<JsonNode>, Option<Location>),
    Object(BTreeMap<String, JsonNode>, Option<Location>),
}

/// JSON5 parser
#[derive(Parser)]
#[grammar = "json5.pest"]
struct Json5Parser;

/// Parse a JSON5 string into [`JsonNode`]'s
pub fn parse<'a>(input: &'a str) -> Result<JsonNode, Error> {
    parse_pair(Json5Parser::parse(Rule::text, input)?.next().unwrap())
}

fn parse_pair<'a>(pair: Pair<'a, Rule>) -> Result<JsonNode, Error> {
    let location = Some(Location::from(&pair.as_span()));
    let node: JsonNode = match pair.as_rule() {
        Rule::null => JsonNode::Null(location),
        Rule::boolean => JsonNode::Bool(
            match pair.as_str() {
                "true" => true,
                "false" => false,
                _ => unreachable!(),
            },
            location,
        ),
        Rule::string | Rule::identifier => parse_string(pair)?,
        Rule::number => parse_number(&pair)?,
        Rule::array => JsonNode::Array(
            pair.into_inner()
                .map(parse_pair)
                .collect::<Result<Vec<_>, _>>()?,
            location,
        ),
        Rule::object => {
            let mut map: BTreeMap<String, JsonNode> = BTreeMap::new();

            println!("{:?}", pair.as_str());

            for pair in pair.into_inner() {
                let mut key_value_pairs = pair.into_inner();
                let key = key_value_pairs.next().unwrap().as_str();
                let value = parse_pair(key_value_pairs.next().unwrap())?;

                map.insert(key.to_string(), value);
            }

            JsonNode::Object(map, location)
        }
        _ => unreachable!(),
    };

    Ok(node)
}

fn parse_string(pair: Pair<'_, Rule>) -> Result<JsonNode, Error> {
    let location = Some(Location::from(&pair.as_span()));
    let mut result = String::new();
    let mut component_iter = pair.into_inner();

    fn parse_char_escape_sequence<'a>(pair: &'a Pair<'_, Rule>) -> &'a str {
        match pair.as_str() {
            "b" => "\u{0008}",
            "f" => "\u{000C}",
            "n" => "\n",
            "r" => "\r",
            "t" => "\t",
            "v" => "\u{000B}",
            c => c,
        }
    }

    while let Some(component) = component_iter.next() {
        match component.as_rule() {
            Rule::char_literal => result.push_str(component.as_str()),
            Rule::char_escape_sequence => result.push_str(parse_char_escape_sequence(&component)),
            Rule::nul_escape_sequence => result.push_str("\u{0000}"),
            Rule::hex_escape_sequence => {
                let hex_escape = match u32::from_str_radix(component.as_str(), 16) {
                    Ok(n) => n,
                    Err(_) => return Err(Error::NumberFormat(location)),
                };

                match char::from_u32(hex_escape) {
                    Some(c) => result.push(c),
                    None => return Err(Error::NumberFormat(location)),
                }
            }
            Rule::unicode_escape_sequence => {
                let hex_escape = match u32::from_str_radix(component.as_str(), 16) {
                    Ok(n) => n,
                    Err(_) => return Err(Error::NumberFormat(location)),
                };

                match hex_escape {
                    0xDC00..=0xDFFF => {
                        // Expecting a low surrogate (trail surrogate)
                        return Err(Error::Unicode(location));
                    }

                    // Non-BMP characters are encoded as a sequence of to hex escapes,
                    // representing UTF-16 surrogate
                    rc1 @ 0xD800..=0xDBFF => {
                        let rc2 = match component_iter.next() {
                            Some(pc2) => {
                                let hex_escape = match u32::from_str_radix(pc2.as_str(), 16) {
                                    Ok(n) => n,
                                    Err(_) => return Err(Error::NumberFormat(location)),
                                };

                                match hex_escape {
                                    rc2 @ 0xDC00..=0xDFFF => rc2,
                                    _ => return Err(Error::Unicode(location)),
                                }
                            }
                            None => {
                                // Missing a low surrogate (trail surrogate)
                                return Err(Error::Unicode(location));
                            }
                        };

                        // Join together
                        let rc = ((rc1 - 0xD800) << 10) | (rc2 - 0xDC00) + 0x1_0000;
                        match char::from_u32(rc) {
                            Some(c) => {
                                result.push(c);
                            }
                            None => {
                                return Err(Error::Unicode(location));
                            }
                        }
                    }

                    rc => match char::from_u32(rc) {
                        Some(c) => {
                            result.push(c);
                        }
                        None => {
                            return Err(Error::Unicode(location));
                        }
                    },
                }
            }

            _ => unreachable!(),
        }
    }

    Ok(JsonNode::String(result, location))
}

fn parse_number<'a>(pair: &Pair<'a, Rule>) -> Result<JsonNode, Error> {
    let location = Some(Location::from(&pair.as_span()));

    fn is_hex_literal(s: &str) -> bool {
        s.len() > 2 && (&s[..2] == "0x" || &s[..2] == "0X")
    }

    match pair.as_str() {
        "Infinity" => Ok(JsonNode::Number(f64::INFINITY, location)),
        "-Infinity" => Ok(JsonNode::Number(f64::NEG_INFINITY, location)),
        "NaN" | "-NaN" => Ok(JsonNode::Number(f64::NAN, location)),
        s if is_hex_literal(s) => u32::from_str_radix(pair.as_str(), 16).map_or_else(
            |_| Err(Error::NumberFormat(location)),
            |n| Ok(JsonNode::Number(n as f64, location)),
        ),
        s => match s.parse::<f64>() {
            Ok(f) => {
                if f.is_finite() {
                    Ok(JsonNode::Number(f, location))
                } else {
                    Err(Error::NumberRange(location))
                }
            }
            Err(_) => Err(Error::NumberFormat(location)),
        },
    }
}

/// Stringify a node tree into canonical JSON5 format. This includes:
pub fn stringify(node: &JsonNode) -> String {
    use JsonNode::*;

    match node {
        Object(o, _) => {
            let contents: Vec<_> = o
                .iter()
                .map(|(name, value)| {
                    // Only quote key values containing whitespace
                    if name.contains(char::is_whitespace) {
                        format!("\"{}\":{}", name, stringify(value))
                    } else {
                        format!("{}:{}", name, stringify(value))
                    }
                })
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

#[cfg(test)]
mod test {
    use super::*;

    const JSON5_STRING: &str = "{a:1,b:true,c:\"xyz\",d:null,e:[1,2,3]}";

    fn get_node_tree() -> JsonNode {
        JsonNode::Object(
            BTreeMap::from([
                (
                    "a".to_string(),
                    JsonNode::Number(1.0, Some(Location { line: 1, column: 4 })),
                ),
                (
                    "b".to_string(),
                    JsonNode::Bool(true, Some(Location { line: 1, column: 8 })),
                ),
                (
                    "c".to_string(),
                    JsonNode::String(
                        "xyz".to_string(),
                        Some(Location {
                            line: 1,
                            column: 15,
                        }),
                    ),
                ),
                (
                    "d".to_string(),
                    JsonNode::Null(Some(Location {
                        line: 1,
                        column: 23,
                    })),
                ),
                (
                    "e".to_string(),
                    JsonNode::Array(
                        vec![
                            JsonNode::Number(
                                1.0,
                                Some(Location {
                                    line: 1,
                                    column: 31,
                                }),
                            ),
                            JsonNode::Number(
                                2.0,
                                Some(Location {
                                    line: 1,
                                    column: 33,
                                }),
                            ),
                            JsonNode::Number(
                                3.0,
                                Some(Location {
                                    line: 1,
                                    column: 35,
                                }),
                            ),
                        ],
                        Some(Location {
                            line: 1,
                            column: 30,
                        }),
                    ),
                ),
            ]),
            Some(Location { line: 1, column: 1 }),
        )
    }

    #[test]
    fn test_parse() {
        assert_eq!(parse(JSON5_STRING).unwrap(), get_node_tree());
    }

    #[test]
    fn test_stringify() {
        assert_eq!(stringify(&get_node_tree()), JSON5_STRING);
    }
}
