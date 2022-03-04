mod error;

pub use error::{JsonError, Location};
pub use hashlink::linked_hash_map::{Iter, LinkedHashMap};

use pest::{iterators::Pair, Parser, Span};
use pest_derive::Parser;

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
  Integer(i64, Option<Location>),
  Float(f64, Option<Location>),
  String(String, Option<Location>),
  Array(Vec<JsonNode>, Option<Location>),
  Object(LinkedHashMap<String, JsonNode>, Option<Location>),
}

/// JSON5 parser
#[derive(Parser)]
#[grammar = "json5.pest"]
struct Json5Parser;

/// Parse a JSON5 string into [`JsonNode`]'s
pub fn parse<'a>(input: &'a str) -> Result<JsonNode, JsonError> {
  parse_pair(Json5Parser::parse(Rule::text, input)?.next().unwrap())
}

fn parse_pair<'a>(pair: Pair<'a, Rule>) -> Result<JsonNode, JsonError> {
  let location = Some(Location::from(&pair.as_span()));
  let node: JsonNode = match pair.as_rule() {
    Rule::null => JsonNode::Null(location),
    Rule::boolean => JsonNode::Bool(pair.as_str() == "true", location),
    Rule::string | Rule::identifier => JsonNode::String(parse_string(pair)?, location),
    Rule::number => {
      if is_int(pair.as_str()) {
        JsonNode::Integer(parse_integer(&pair)?, location)
      } else {
        JsonNode::Float(parse_float(&pair)?, location)
      }
    }
    Rule::array => JsonNode::Array(
      pair
        .into_inner()
        .map(parse_pair)
        .collect::<Result<Vec<_>, _>>()?,
      location,
    ),
    Rule::object => {
      let mut map: LinkedHashMap<String, JsonNode> = LinkedHashMap::new();

      for pair in pair.into_inner() {
        let mut key_value_pairs = pair.into_inner();
        let key = parse_string(key_value_pairs.next().unwrap())?;
        let value = parse_pair(key_value_pairs.next().unwrap())?;

        map.insert(key, value);
      }

      JsonNode::Object(map, location)
    }
    _ => unreachable!(),
  };

  Ok(node)
}

fn parse_string(pair: Pair<'_, Rule>) -> Result<String, JsonError> {
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
          Err(_) => return Err(JsonError::NumberFormat(location)),
        };

        match char::from_u32(hex_escape) {
          Some(c) => result.push(c),
          None => return Err(JsonError::NumberFormat(location)),
        }
      }
      Rule::unicode_escape_sequence => {
        let hex_escape = match u32::from_str_radix(component.as_str(), 16) {
          Ok(n) => n,
          Err(_) => return Err(JsonError::NumberFormat(location)),
        };

        match hex_escape {
          0xDC00..=0xDFFF => {
            // Expecting a low surrogate (trail surrogate)
            return Err(JsonError::Unicode(location));
          }

          // Non-BMP characters are encoded as a sequence of to hex escapes,
          // representing UTF-16 surrogate
          rc1 @ 0xD800..=0xDBFF => {
            let rc2 = match component_iter.next() {
              Some(pc2) => {
                let hex_escape = match u32::from_str_radix(pc2.as_str(), 16) {
                  Ok(n) => n,
                  Err(_) => return Err(JsonError::NumberFormat(location)),
                };

                match hex_escape {
                  rc2 @ 0xDC00..=0xDFFF => rc2,
                  _ => return Err(JsonError::Unicode(location)),
                }
              }
              None => {
                // Missing a low surrogate (trail surrogate)
                return Err(JsonError::Unicode(location));
              }
            };

            // Join together
            let rc = ((rc1 - 0xD800) << 10) | (rc2 - 0xDC00) + 0x1_0000;
            match char::from_u32(rc) {
              Some(c) => {
                result.push(c);
              }
              None => {
                return Err(JsonError::Unicode(location));
              }
            }
          }

          rc => match char::from_u32(rc) {
            Some(c) => {
              result.push(c);
            }
            None => {
              return Err(JsonError::Unicode(location));
            }
          },
        }
      }

      _ => unreachable!(),
    }
  }

  Ok(result)
}

fn is_hex_literal(s: &str) -> bool {
  s.len() > 2 && (&s[..2] == "0x" || &s[..2] == "0X")
}

fn is_infinite(s: &str) -> bool {
  s == "Infinity" || s == "-Infinity"
}

fn is_nan(s: &str) -> bool {
  s == "NaN" || s == "-NaN"
}

fn is_int(s: &str) -> bool {
  !s.contains('.')
    && (is_hex_literal(s) || (!s.contains('e') && !s.contains('E')))
    && !is_infinite(s)
    && !is_nan(s)
}

fn parse_integer(pair: &Pair<'_, Rule>) -> Result<i64, JsonError> {
  let location = Some(Location::from(&pair.as_span()));

  match pair.as_str() {
    s if is_hex_literal(s) => {
      i64::from_str_radix(&s[2..], 16).or_else(|_| Err(JsonError::NumberFormat(location)))
    }
    s => s
      .parse::<i64>()
      .or_else(|_| Err(JsonError::NumberFormat(location))),
  }
}

fn parse_float<'a>(pair: &Pair<'a, Rule>) -> Result<f64, JsonError> {
  let location = Some(Location::from(&pair.as_span()));

  match pair.as_str() {
    "Infinity" => Ok(f64::INFINITY),
    "-Infinity" => Ok(f64::NEG_INFINITY),
    "NaN" | "-NaN" => Ok(f64::NAN),
    s => match s.parse::<f64>() {
      Ok(f) => {
        if f.is_finite() {
          Ok(f)
        } else {
          Err(JsonError::NumberRange(location))
        }
      }
      Err(_) => Err(JsonError::NumberFormat(location)),
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
    Integer(i, _) => format!("{}", i),
    Float(f, _) => format!("{}", f),
    Bool(b, _) => format!("{}", b),
    Null(_) => format!("null"),
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn test_null() {
    assert_eq!(
      parse("null").unwrap(),
      JsonNode::Null(Some(Location { column: 1, line: 1 }))
    );
  }

  #[test]
  fn test_bool() {
    assert_eq!(
      parse("true").unwrap(),
      JsonNode::Bool(true, Some(Location { column: 1, line: 1 }))
    );
    assert_eq!(
      parse("false").unwrap(),
      JsonNode::Bool(false, Some(Location { column: 1, line: 1 }))
    );
  }

  #[test]
  fn test_integer() {
    assert_eq!(
      parse("1").unwrap(),
      JsonNode::Integer(1, Some(Location { column: 1, line: 1 }))
    );
  }

  #[test]
  fn test_float() {
    assert_eq!(
      parse("Infinity").unwrap(),
      JsonNode::Float(f64::INFINITY, Some(Location { column: 1, line: 1 }))
    );
  }

  #[test]
  fn test_string_escapes() {
    assert_eq!(
      parse("\"\\b\\f\\n\\r\\t\\v\\z\\x0A\\u0041\\0\"").unwrap(),
      JsonNode::String(
        String::from("\u{0008}\u{000C}\n\r\t\u{000B}z\u{000A}A\u{0000}"),
        Some(Location { column: 1, line: 1 })
      )
    );
    assert_eq!(
      parse(r#""\uD83C\uDDEF\uD83C\uDDF5""#).unwrap(),
      JsonNode::String(
        String::from("\u{1F1EF}\u{1F1F5}"),
        Some(Location { column: 1, line: 1 })
      )
    );
  }

  #[test]
  fn test_empty_array() {
    assert_eq!(
      parse("[]").unwrap(),
      JsonNode::Array(vec![], Some(Location { column: 1, line: 1 }))
    );
  }

  #[test]
  fn test_array() {
    assert_eq!(
      parse("[1.0,2.0]").unwrap(),
      JsonNode::Array(
        vec![
          JsonNode::Float(1.0, Some(Location { column: 2, line: 1 })),
          JsonNode::Float(2.0, Some(Location { column: 6, line: 1 }))
        ],
        Some(Location { column: 1, line: 1 })
      )
    );
  }

  #[test]
  fn test_empty_object() {
    assert_eq!(
      parse("{}").unwrap(),
      JsonNode::Object(LinkedHashMap::new(), Some(Location { column: 1, line: 1 }))
    );
  }

  #[test]
  fn test_object() {
    assert_eq!(
      parse("{a: 1, \"b c\": 2}").unwrap(),
      JsonNode::Object(
        LinkedHashMap::from_iter(
          [
            (
              "a".to_string(),
              JsonNode::Integer(1, Some(Location { column: 5, line: 1 }))
            ),
            (
              "b c".to_string(),
              JsonNode::Integer(
                2,
                Some(Location {
                  column: 15,
                  line: 1
                })
              )
            ),
          ]
          .into_iter()
        ),
        Some(Location { column: 1, line: 1 })
      )
    );
  }

  #[test]
  fn test_bad_object() {
    match parse("{a:") {
      Err(_) => (),
      Ok(_) => panic!("Unexpected result"),
    }
  }

  #[test]
  fn test_error_display() {
    println!("{}", JsonError::Syntax("".to_string(), None));
    println!("{}", JsonError::NumberFormat(None));
    println!("{}", JsonError::NumberRange(None));
    println!("{}", JsonError::Unicode(None));
  }

  #[test]
  fn test_round_trip() {
    const JSON5_STRING: &str = "{a:1,b:true,c:\"xyz\",d:null,e:[1,2,3],\"a b\":88}";
    let node_tree = parse(JSON5_STRING).unwrap();

    assert_eq!(
      node_tree,
      JsonNode::Object(
        LinkedHashMap::from_iter(
          [
            (
              "a".to_string(),
              JsonNode::Integer(1, Some(Location { line: 1, column: 4 })),
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
                  JsonNode::Integer(
                    1,
                    Some(Location {
                      line: 1,
                      column: 31,
                    }),
                  ),
                  JsonNode::Integer(
                    2,
                    Some(Location {
                      line: 1,
                      column: 33,
                    }),
                  ),
                  JsonNode::Integer(
                    3,
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
            (
              "a b".to_string(),
              JsonNode::Integer(
                88,
                Some(Location {
                  line: 1,
                  column: 44,
                }),
              ),
            ),
          ]
          .into_iter()
        ),
        Some(Location { column: 1, line: 1 })
      )
    );
    assert_eq!(stringify(&node_tree), JSON5_STRING);
  }
}
