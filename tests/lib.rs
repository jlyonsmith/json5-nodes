use json5_ast;

#[test]
fn test_parse_nodes() {
  json5_ast::from_str("{}").unwrap();
}
