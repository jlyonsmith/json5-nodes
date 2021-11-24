use json5_nodes::*;
use std::collections::BTreeMap;

#[test]
fn test_parse_nodes() {
  assert_eq!(
    json5_nodes::parse("{a: 1, b: true, c: 'xyz', d: null, e: [1, 2, 3]}").unwrap(),
    JsonNode::Object(BTreeMap::from([]), Some(Location { line: 1, column: 1 }))
  );
}

#[test]
fn test_stringify() {
  assert_eq!(
    json5_nodes::stringify(&JsonNode::Object(BTreeMap::from([]), None)),
    "{}"
  );
}
