use super::*;
use std::str::FromStr;

#[test]
fn test_resolve_ref_str() {
    let token = Token::Ref(vec![Token::literal_from_str("foo")]);
    let params = Mapping::from_str("foo: bar").unwrap();

    let v = token.resolve(&params).unwrap();
    assert_eq!(v, Value::Literal("bar".into()));
}

#[test]
fn test_resolve_ref_val() {
    let token = Token::Ref(vec![Token::literal_from_str("foo")]);
    let params = Mapping::from_str("foo: True").unwrap();

    let v = token.resolve(&params).unwrap();
    assert_eq!(v, Value::Bool(true));
}

#[test]
fn test_resolve_literal() {
    let token = Token::literal_from_str("foo");
    let params = Mapping::new();

    let v = token.resolve(&params).unwrap();
    assert_eq!(v, Value::Literal("foo".into()));
}

#[test]
fn test_resolve_combined() {
    let token = Token::Combined(vec![
        Token::literal_from_str("foo"),
        Token::Ref(vec![Token::literal_from_str("foo")]),
    ]);
    let params = Mapping::from_str("{foo: bar, bar: baz}").unwrap();

    let v = token.resolve(&params).unwrap();
    assert_eq!(v, Value::Literal("foobar".into()));
}
#[test]

fn test_resolve_combined_2() {
    let token = Token::Combined(vec![
        Token::literal_from_str("foo"),
        Token::Ref(vec![Token::literal_from_str("foo")]),
    ]);
    let params = Mapping::from_str(r#"{foo: "${bar}", bar: baz}"#).unwrap();

    let v = token.resolve(&params).unwrap();
    assert_eq!(v, Value::Literal("foobaz".into()));
}

#[test]
fn test_resolve_combined_3() {
    let token = Token::Combined(vec![
        Token::literal_from_str("foo"),
        Token::Ref(vec![Token::literal_from_str("foo")]),
    ]);
    let params = r#"
    foo: \${bar}
    bar: baz
    "#;
    let params = Mapping::from_str(params).unwrap();

    let v = token.resolve(&params).unwrap();
    assert_eq!(v, Value::Literal("foo${bar}".into()));
}

#[test]
fn test_token_parse_no_ref() {
    assert_eq!(Token::parse("foo-bar-baz").unwrap(), None);
}

#[test]
fn test_token_parse_escaped_ref() {
    assert_eq!(
        Token::parse(r"foo-\${bar}-baz").unwrap(),
        Some(Token::literal_from_str("foo-${bar}-baz"))
    );
}

#[test]
fn test_token_parse_value_ref() {
    assert_eq!(
        Token::parse(r"${foo}").unwrap(),
        Some(Token::Ref(vec![Token::literal_from_str("foo")]),)
    );
}

#[test]
fn test_token_parse_embedded_ref() {
    assert_eq!(
        Token::parse(r"${foo}-bar").unwrap(),
        Some(Token::Combined(vec![
            Token::Ref(vec![Token::literal_from_str("foo")]),
            Token::literal_from_str("-bar")
        ]))
    );
}

#[test]
fn test_resolve() {
    let p = Mapping::from_str("foo: foo").unwrap();
    let reftoken = parse_ref(&"${foo}").unwrap();

    assert_eq!(reftoken.resolve(&p).unwrap(), Value::Literal("foo".into()));
}

#[test]
fn test_resolve_subkey() {
    let p = Mapping::from_str("foo: {foo: foo}").unwrap();
    let reftoken = parse_ref(&"${foo:foo}").unwrap();

    assert_eq!(reftoken.resolve(&p).unwrap(), Value::Literal("foo".into()));
}

#[test]
fn test_resolve_nested() {
    let p = Mapping::from_str("{foo: foo, bar: {foo: foo}}").unwrap();
    let reftoken = parse_ref(&"${bar:${foo}}").unwrap();

    assert_eq!(reftoken.resolve(&p).unwrap(), Value::Literal("foo".into()));
}

#[test]
fn test_resolve_nested_subkey() {
    let params = r#"
    foo:
        bar: foo
    bar:
        foo: foo"#;
    let p = Mapping::from_str(params).unwrap();

    // ${bar:${foo:bar}} == ${bar:foo} == foo
    let reftoken = parse_ref(&"${bar:${foo:bar}}").unwrap();
    assert_eq!(
        reftoken.resolve(&p).unwrap(),
        Value::Literal("foo".to_string())
    );
}

#[test]
fn test_resolve_kapitan_secret_ref() {
    let params = r#"
    baz:
        baz: baz
    "#;

    let p = Mapping::from_str(params).unwrap();

    let reftoken = parse_ref(&"?{vaultkv:foo/bar/${baz:baz}/qux}").unwrap();
    dbg!(&reftoken);
    assert_eq!(
        reftoken.resolve(&p).unwrap(),
        Value::Literal("?{vaultkv:foo/bar/baz/qux}".to_string())
    );
}

#[test]
fn test_resolve_escaped_ref() {
    let params = r#"
    foo:
      label: '\${PROJECT_LABEL}'
    PROJECT_LABEL: {}
    "#;
    let p = Mapping::from_str(params).unwrap();

    let reftoken = parse_ref("\\${PROJECT_LABEL}").unwrap();
    assert_eq!(
        reftoken.resolve(&p).unwrap(),
        Value::Literal("${PROJECT_LABEL}".to_string())
    );
}

#[test]
fn test_resolve_mapping_value() {
    let p = r#"
    foo:
      bar: bar
      baz: baz
    "#;
    let p = Mapping::from_str(p).unwrap();
    let reftoken = parse_ref("${foo}").unwrap();
    assert_eq!(
        reftoken.resolve(&p).unwrap(),
        Value::Mapping(Mapping::from_str("{bar: bar, baz: baz}").unwrap())
    );
}

#[test]
fn test_resolve_mapping_embedded() {
    let p = r#"
    foo:
      bar: bar
      baz: baz
    "#;
    let p = Mapping::from_str(p).unwrap();
    let reftoken = parse_ref("foo: ${foo}").unwrap();
    assert_eq!(
        reftoken.resolve(&p).unwrap(),
        // Mapping is serialized as JSON when embedded in a string. serde_json emits JSON maps
        // with lexically sorted keys and minimal whitespace.
        Value::Literal(r#"foo: {"bar":"bar","baz":"baz"}"#.to_string())
    );
}
