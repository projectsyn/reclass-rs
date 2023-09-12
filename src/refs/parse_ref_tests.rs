use super::*;

#[test]
fn test_parse_no_ref() {
    let input = "foo-bar-baz";
    let res = parse_ref(input).unwrap();
    assert_eq!(res, Token::literal_from_str("foo-bar-baz"))
}

#[test]
fn test_parse_escaped_ref() {
    let input = r"foo-bar-\${baz}";
    let res = parse_ref(input).unwrap();
    assert_eq!(res, Token::literal_from_str("foo-bar-${baz}"))
}

#[test]
fn test_parse_ref() {
    let input = "foo-${bar:baz}";
    let res = parse_ref(input).unwrap();
    assert_eq!(
        res,
        Token::Combined(vec![
            Token::Literal("foo-".to_owned()),
            Token::Ref(vec![Token::Literal("bar:baz".to_owned())])
        ])
    )
}

#[test]
fn test_parse_nested() {
    let tstr = "${foo:${bar}}";
    assert_eq!(
        parse_ref(tstr).unwrap(),
        Token::Ref(vec![
            Token::Literal("foo:".into()),
            Token::Ref(vec![Token::Literal("bar".into())])
        ])
    );
}

#[test]
fn test_parse_nested_deep() {
    let tstr = "${foo:${bar:${foo:baz}}}";
    assert_eq!(
        parse_ref(tstr).unwrap(),
        Token::Ref(vec![
            Token::Literal("foo:".into()),
            Token::Ref(vec![
                Token::Literal("bar:".into()),
                Token::Ref(vec![Token::Literal("foo:baz".into()),])
            ])
        ])
    );
}

#[test]
fn test_parse_ref_error_1() {
    let input = "foo-${bar";
    let res = parse_ref(input);
    assert!(res.is_err());
    let e = res.unwrap_err();
    println!("{}", e);
}

#[test]
fn test_parse_ref_error_2() {
    let input = "foo-${bar}${}";
    let res = parse_ref(input);
    assert!(res.is_err());
    let e = res.unwrap_err();
    println!("{}", e);
}

#[test]
fn test_parse_ref_error_3() {
    let input = "${foo-${bar}";
    let res = parse_ref(input);
    assert!(res.is_err());
    let e = res.unwrap_err();
    println!("{}", e);
}

#[test]
fn test_parse_ref_format() {
    let input = r"foo-${foo:${bar}}-${baz}-\${bar}-\\${qux}";
    let res = parse_ref(&input).unwrap();
    assert_eq!(
        res,
        Token::Combined(vec![
            Token::literal_from_str("foo-"),
            Token::Ref(vec![
                Token::literal_from_str("foo:"),
                Token::Ref(vec![Token::literal_from_str("bar")])
            ]),
            Token::literal_from_str("-"),
            Token::Ref(vec![Token::literal_from_str("baz")]),
            Token::literal_from_str(r"-${bar}-\"),
            Token::Ref(vec![Token::literal_from_str("qux")]),
        ])
    );
    assert_eq!(format!("{}", res), input);
}
