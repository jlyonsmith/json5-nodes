use json5_ast::*;
use std::collections::BTreeMap;

#[test]
fn test_parse_nodes() {
  assert_eq!(
    json5_ast::parse("{}").unwrap(),
    JsonNode::Object(BTreeMap::from([]), Some(Location { line: 1, column: 1 }))
  );
}

#[test]
fn test_stringify() {
  assert_eq!(
    json5_ast::stringify(&JsonNode::Object(BTreeMap::from([]), None)),
    "{}"
  );
}
