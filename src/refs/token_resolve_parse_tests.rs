use super::*;
use std::str::FromStr;

#[test]
fn test_resolve_ref_str() {
    let token = Token::Ref(vec![Token::literal_from_str("foo")]);
    let params = Mapping::from_str("foo: bar").unwrap();

    let mut state = ResolveState::default();
    let v = token
        .resolve(&params, &mut state, &RenderOpts::default())
        .unwrap();
    assert_eq!(v, Value::Literal("bar".into()));
}

#[test]
fn test_resolve_ref_val() {
    let token = Token::Ref(vec![Token::literal_from_str("foo")]);
    let params = Mapping::from_str("foo: True").unwrap();

    let mut state = ResolveState::default();
    let v = token
        .resolve(&params, &mut state, &RenderOpts::default())
        .unwrap();
    assert_eq!(v, Value::Bool(true));
}

#[test]
fn test_resolve_literal() {
    let token = Token::literal_from_str("foo");
    let params = Mapping::new();

    let mut state = ResolveState::default();
    let v = token
        .resolve(&params, &mut state, &RenderOpts::default())
        .unwrap();
    assert_eq!(v, Value::Literal("foo".into()));
}

#[test]
fn test_resolve_combined() {
    let token = Token::Combined(vec![
        Token::literal_from_str("foo"),
        Token::Ref(vec![Token::literal_from_str("foo")]),
    ]);
    let params = Mapping::from_str("{foo: bar, bar: baz}").unwrap();

    let mut state = ResolveState::default();
    let v = token
        .resolve(&params, &mut state, &RenderOpts::default())
        .unwrap();
    assert_eq!(v, Value::Literal("foobar".into()));
}
#[test]

fn test_resolve_combined_2() {
    let token = Token::Combined(vec![
        Token::literal_from_str("foo"),
        Token::Ref(vec![Token::literal_from_str("foo")]),
    ]);
    let params = Mapping::from_str(r#"{foo: "${bar}", bar: baz}"#).unwrap();

    let mut state = ResolveState::default();
    let v = token
        .resolve(&params, &mut state, &RenderOpts::default())
        .unwrap();
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

    let mut state = ResolveState::default();
    let v = token
        .resolve(&params, &mut state, &RenderOpts::default())
        .unwrap();
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

    let mut state = ResolveState::default();
    assert_eq!(
        reftoken
            .resolve(&p, &mut state, &RenderOpts::default())
            .unwrap(),
        Value::Literal("foo".into())
    );
}

#[test]
fn test_resolve_subkey() {
    let p = Mapping::from_str("foo: {foo: foo}").unwrap();
    let reftoken = parse_ref(&"${foo:foo}").unwrap();

    let mut state = ResolveState::default();
    let v = reftoken
        .resolve(&p, &mut state, &RenderOpts::default())
        .unwrap();
    assert_eq!(v, Value::Literal("foo".into()));
}

#[test]
fn test_resolve_nested() {
    let p = Mapping::from_str("{foo: foo, bar: {foo: foo}}").unwrap();
    let reftoken = parse_ref(&"${bar:${foo}}").unwrap();

    let mut state = ResolveState::default();
    let v = reftoken
        .resolve(&p, &mut state, &RenderOpts::default())
        .unwrap();
    assert_eq!(v, Value::Literal("foo".into()));
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
    let mut state = ResolveState::default();
    let v = reftoken
        .resolve(&p, &mut state, &RenderOpts::default())
        .unwrap();
    assert_eq!(v, Value::Literal("foo".to_string()));
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
    let mut state = ResolveState::default();
    let v = reftoken
        .resolve(&p, &mut state, &RenderOpts::default())
        .unwrap();
    assert_eq!(v, Value::Literal("?{vaultkv:foo/bar/baz/qux}".to_string()));
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
    let mut state = ResolveState::default();
    let v = reftoken
        .resolve(&p, &mut state, &RenderOpts::default())
        .unwrap();
    assert_eq!(v, Value::Literal("${PROJECT_LABEL}".to_string()));
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
    let mut state = ResolveState::default();
    let v = reftoken
        .resolve(&p, &mut state, &RenderOpts::default())
        .unwrap();
    assert_eq!(
        v,
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
    let mut state = ResolveState::default();
    let v = reftoken
        .resolve(&p, &mut state, &RenderOpts::default())
        .unwrap();
    assert_eq!(
        v,
        // Mapping is serialized as JSON when embedded in a string. serde_json emits JSON maps
        // with lexically sorted keys and minimal whitespace.
        Value::Literal(r#"foo: {"bar":"bar","baz":"baz"}"#.to_string())
    );
}

#[test]
#[should_panic(expected = "Detected reference loop with reference paths [\"foo\"].")]
fn test_resolve_recursive_error() {
    let p = r#"
    foo: ${foo}
    "#;
    let p = Mapping::from_str(p).unwrap();
    let reftoken = parse_ref("${foo}").unwrap();

    let mut state = ResolveState::default();
    let _v = reftoken
        .resolve(&p, &mut state, &RenderOpts::default())
        .unwrap();
}

#[test]
#[should_panic(expected = "Detected reference loop with reference paths [\"bar\", \"foo\"].")]
fn test_resolve_recursive_error_2() {
    let p = r#"
    foo: ${bar}
    bar: ${foo}
    "#;
    let p = Mapping::from_str(p).unwrap();
    let reftoken = parse_ref("${foo}").unwrap();

    let mut state = ResolveState::default();
    let _v = reftoken
        .resolve(&p, &mut state, &RenderOpts::default())
        .unwrap();
}

#[test]
fn test_resolve_nested_recursive_error() {
    let p = r#"
    foo: ${baz}
    baz:
      qux: ${foo}
    "#;
    let p = Mapping::from_str(p).unwrap();
    let reftoken = parse_ref("${foo}").unwrap();

    let mut state = ResolveState::default();
    let v = reftoken
        .resolve(&p, &mut state, &RenderOpts::default())
        .unwrap();
    // nested recursive error doesn't raise an error in `resolve()` anymore
    let mut expected = Mapping::new();
    expected
        .insert(
            "qux".into(),
            Value::ResolveError(
                "Detected reference loop with reference paths [\"baz\", \"foo\"].".into(),
            ),
        )
        .unwrap();
    assert_eq!(v, Value::Mapping(expected));
}

#[test]
fn test_resolve_ref_default_value() {
    let p = r#"
    bar: bar
    "#;
    let p = Mapping::from_str(p).unwrap();
    let reftoken = parse_ref("${bar::fallback}").unwrap();

    let mut state = ResolveState::default();
    let v = reftoken
        .resolve(&p, &mut state, &RenderOpts::default())
        .unwrap();
    assert_eq!(v, Value::Literal("bar".into()));
}

#[test]
fn test_resolve_missingref_default_value() {
    let p = r#"
    bar: bar
    "#;
    let p = Mapping::from_str(p).unwrap();
    let reftoken = parse_ref("${foo::fallback}").unwrap();

    let mut state = ResolveState::default();
    let v = reftoken
        .resolve(&p, &mut state, &RenderOpts::default())
        .unwrap();
    assert_eq!(v, Value::Literal("fallback".into()));
}

#[test]
fn test_resolve_missingref_default_value_int() {
    let p = Mapping::new();
    let reftoken = parse_ref("${foo::3}").unwrap();

    let mut state = ResolveState::default();
    let v = reftoken
        .resolve(&p, &mut state, &RenderOpts::default())
        .unwrap();
    assert_eq!(v, 3.into());
}

#[test]
fn test_resolve_missingref_default_value_inf() {
    let p = Mapping::new();
    let reftoken = parse_ref("${foo::.inf}").unwrap();

    let mut state = ResolveState::default();
    let v = reftoken
        .resolve(&p, &mut state, &RenderOpts::default())
        .unwrap();
    assert_eq!(v, f64::INFINITY.into());
}

#[test]
fn test_resolve_missingref_default_value_quoted_string() {
    let p = Mapping::new();
    let reftoken = parse_ref("${foo::\".inf\"}").unwrap();

    let mut state = ResolveState::default();
    let v = reftoken
        .resolve(&p, &mut state, &RenderOpts::default())
        .unwrap();
    assert_eq!(v, Value::Literal(".inf".into()));
}

#[test]
fn test_resolve_missingref_default_value_quoted_empty_string() {
    let p = Mapping::new();
    let reftoken = parse_ref("${foo::\"\"}").unwrap();

    let mut state = ResolveState::default();
    let v = reftoken
        .resolve(&p, &mut state, &RenderOpts::default())
        .unwrap();
    assert_eq!(v, Value::Literal("".into()));
}

#[test]
fn test_resolve_missingref_default_value_null_empty() {
    let p = r#"
    bar: bar
    "#;
    let p = Mapping::from_str(p).unwrap();
    let reftoken = parse_ref("${foo::}").unwrap();

    let mut state = ResolveState::default();
    let v = reftoken
        .resolve(&p, &mut state, &RenderOpts::default())
        .unwrap();
    assert_eq!(v, Value::Null);
}

#[test]
fn test_resolve_missingref_default_value_null_tilde() {
    let p = Mapping::new();
    let reftoken = parse_ref("${foo::~}").unwrap();

    let mut state = ResolveState::default();
    let v = reftoken
        .resolve(&p, &mut state, &RenderOpts::default())
        .unwrap();
    assert_eq!(v, Value::Null);
}

#[test]
fn test_resolve_missingref_default_value_null() {
    let p = Mapping::new();
    let reftoken = parse_ref("${foo::null}").unwrap();

    let mut state = ResolveState::default();
    let v = reftoken
        .resolve(&p, &mut state, &RenderOpts::default())
        .unwrap();
    assert_eq!(v, Value::Null);
}

#[test]
fn test_resolve_missingref_default_value_bool() {
    let p = Mapping::new();
    let reftoken = parse_ref("${foo::true}").unwrap();

    let mut state = ResolveState::default();
    let v = reftoken
        .resolve(&p, &mut state, &RenderOpts::default())
        .unwrap();
    assert_eq!(v, Value::Bool(true));
}

#[test]
fn test_resolve_missingref_default_value_literal_sequence() {
    let p = Mapping::new();
    let reftoken = parse_ref("${foo::[1, 2]}").unwrap();

    let mut state = ResolveState::default();
    let v = reftoken
        .resolve(&p, &mut state, &RenderOpts::default())
        .unwrap();
    let expected = Value::Sequence(vec![1.into(), 2.into()]);
    assert_eq!(v, expected);
}

#[test]
fn test_resolve_missingref_default_value_literal_sequence_mixed() {
    let p = Mapping::new();
    let reftoken = parse_ref("${foo::[1, \"2\"]}").unwrap();

    let mut state = ResolveState::default();
    let v = reftoken
        .resolve(&p, &mut state, &RenderOpts::default())
        .unwrap();
    let expected = Value::Sequence(vec![1.into(), "2".into()]);
    assert_eq!(v, expected);
}

#[test]
fn test_resolve_missingref_default_value_literal_sequence_escapedref() {
    let p = r#"
    bar: bar
    "#;
    let p = Mapping::from_str(p).unwrap();
    // because the ref parser doesn't understand it's parsing a default value, we need to escape
    // the escaped ref's closing } to get the correct parse of
    //   Literal('['), Ref('bar'), Literal(', \${bar}]')
    // for the default value since technically we're inside a ref while parsing the default value.
    let reftoken = parse_ref("${foo::[${bar}, \"\\${bar\\}\"]}").unwrap();
    eprintln!("{reftoken:?}");

    let mut state = ResolveState::default();
    let v = reftoken
        .resolve(&p, &mut state, &RenderOpts::default())
        .unwrap();
    let expected = Value::Sequence(vec!["bar".into(), "${bar}".into()]);
    assert_eq!(v, expected);
}

#[test]
fn test_resolve_missingref_default_value_literal_mapping() {
    let p = r#"
    bar: bar
    "#;
    let p = Mapping::from_str(p).unwrap();
    // because the ref parser doesn't understand it's parsing a default value, we need to escape
    // the default value's closing } to get the correct parse for the default value since
    // technically we're inside a ref while parsing the default value.
    let reftoken = parse_ref("${foo::{\\}}").unwrap();
    eprintln!("{reftoken:?}");

    let mut state = ResolveState::default();
    let v = reftoken
        .resolve(&p, &mut state, &RenderOpts::default())
        .unwrap();
    let m = Mapping::new();
    let expected = Value::Mapping(m);
    assert_eq!(v, expected);
}

#[test]
fn test_resolve_missingref_default_value_literal_mapping_2() {
    let p = r#"
    bar: bar
    "#;
    let p = Mapping::from_str(p).unwrap();
    // because the ref parser doesn't understand it's parsing a default value, we need to escape
    // the default value's closing } to get the correct parse for the default value since
    // technically we're inside a ref while parsing the default value.
    let reftoken = parse_ref("${foo::{a: a\\}}").unwrap();
    eprintln!("{reftoken:?}");

    let mut state = ResolveState::default();
    let v = reftoken
        .resolve(&p, &mut state, &RenderOpts::default())
        .unwrap();
    let mut m = Mapping::new();
    m.insert("a".into(), "a".into()).unwrap();
    let expected = Value::Mapping(m);
    assert_eq!(v, expected);
}

#[test]
fn test_resolve_missingref_default_value_literal_mapping_with_ref() {
    let p = r#"
    bar: bar
    "#;
    let p = Mapping::from_str(p).unwrap();
    // because the ref parser doesn't understand it's parsing a default value, we need to escape
    // the default value's closing } to get the correct parse for the default value since
    // technically we're inside a ref while parsing the default value.
    let reftoken = parse_ref("${foo::{a: ${bar}\\}}").unwrap();
    eprintln!("{reftoken:?}");

    let mut state = ResolveState::default();
    let v = reftoken
        .resolve(&p, &mut state, &RenderOpts::default())
        .unwrap();
    let mut m = Mapping::new();
    m.insert("a".into(), "bar".into()).unwrap();
    let expected = Value::Mapping(m);
    assert_eq!(v, expected);
}

#[test]
fn test_resolve_missingref_default_value_literal_quoted_mapping_with_ref() {
    let p = r#"
    bar: bar
    "#;
    let p = Mapping::from_str(p).unwrap();
    // because the ref parser doesn't understand it's parsing a default value, we need to escape
    // the default value's closing } to get the correct parse for the default value since
    // technically we're inside a ref while parsing the default value.
    let reftoken = parse_ref("${foo::\"{a: ${bar}\\}\"}").unwrap();
    eprintln!("{reftoken:?}");

    let mut state = ResolveState::default();
    let v = reftoken
        .resolve(&p, &mut state, &RenderOpts::default())
        .unwrap();
    assert_eq!(v, Value::Literal("{a: bar}".into()));
}

#[test]
fn test_resolve_missingref_default_value_escaped_ref() {
    let p = r#"
    bar: bar
    "#;
    let p = Mapping::from_str(p).unwrap();
    let reftoken = parse_ref("${foo::\\${bar}}").unwrap();
    eprintln!("{reftoken:?}");

    let mut state = ResolveState::default();
    let v = reftoken
        .resolve(&p, &mut state, &RenderOpts::default())
        .unwrap();
    assert_eq!(v, Value::Literal("${bar}".into()));
}

#[test]
fn test_resolve_missingref_default_value_ref() {
    let p = r#"
    bar: bar
    "#;
    let p = Mapping::from_str(p).unwrap();
    let reftoken = parse_ref("${foo::${bar}}").unwrap();

    let mut state = ResolveState::default();
    let v = reftoken
        .resolve(&p, &mut state, &RenderOpts::default())
        .unwrap();
    assert_eq!(v, Value::Literal("bar".into()));
}

#[test]
fn test_resolve_missingref_default_value_ref_complex_value() {
    let p = r#"
    bar:
      foo: bar
    "#;
    let p = Mapping::from_str(p).unwrap();
    let reftoken = parse_ref("${foo::${bar}}").unwrap();

    let mut state = ResolveState::default();
    let v = reftoken
        .resolve(&p, &mut state, &RenderOpts::default())
        .unwrap();
    let mut m = Mapping::new();
    m.insert("foo".into(), "bar".into()).unwrap();
    assert_eq!(v, Value::Mapping(m));
}

#[test]
fn test_resolve_missingref_default_value_ref_bool() {
    let p = r#"
    bar: false
    "#;
    let p = Mapping::from_str(p).unwrap();
    let reftoken = parse_ref("${foo::${bar}}").unwrap();

    let mut state = ResolveState::default();
    let v = reftoken
        .resolve(&p, &mut state, &RenderOpts::default())
        .unwrap();
    assert_eq!(v, Value::Bool(false));
}

#[test]
#[should_panic(
    expected = "Error parsing default value for reference 'foo' in parameter '': \
    did not find expected node content at line 2 column 1, while parsing a flow node"
)]
fn test_resolve_missingref_default_value_parse_error() {
    let p = r#"
    bar: false
    "#;
    let p = Mapping::from_str(p).unwrap();
    let reftoken = parse_ref("${foo::[1,}").unwrap();

    let mut state = ResolveState::default();
    let _v = reftoken
        .resolve(&p, &mut state, &RenderOpts::default())
        .unwrap();
}

#[test]
#[should_panic(
    expected = "lookup error for reference '${bar}' in parameter '': key 'bar' not found"
)]
fn test_resolve_missingref_default_value_missingref() {
    let p = Mapping::new();
    let reftoken = parse_ref("${foo::${bar}}").unwrap();

    let mut state = ResolveState::default();
    let _v = reftoken
        .resolve(&p, &mut state, &RenderOpts::default())
        .unwrap();
}

#[test]
fn test_resolve_missingref_default_value_nested_ref() {
    let p = r#"
    foo: foo
    bar:
      foo: ${foo}
      bar: ${qux:qux}
    baz: baz
    qux:
      qux:
        baz: foo
    "#;
    let p = Mapping::from_str(p).unwrap();
    let reftoken = parse_ref("${bar:qux::${bar:bar:${baz}}}").unwrap();

    let mut state = ResolveState::default();
    let v = reftoken
        .resolve(&p, &mut state, &RenderOpts::default())
        .unwrap();
    assert_eq!(v, Value::Literal("foo".into()));
}

#[test]
fn test_resolve_nested_missingref_default_value_missingref_default_value() {
    let p = r#"
    baz: baz
    "#;
    let p = Mapping::from_str(p).unwrap();
    let reftoken = parse_ref("${bar:${baz}::${foo::default}}").unwrap();

    let mut state = ResolveState::default();
    let v = reftoken
        .resolve(&p, &mut state, &RenderOpts::default())
        .unwrap();
    assert_eq!(v, Value::Literal("default".into()));
}
