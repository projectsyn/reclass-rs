use super::*;

#[test]
fn test_is_ref() {
    assert_eq!(Token::Literal("foo".into()).is_ref(), false);
    assert_eq!(
        Token::Ref(vec![Token::Literal("foo".into())]).is_ref(),
        true
    );
}

#[test]
fn test_is_literal() {
    assert_eq!(Token::Literal("foo".into()).is_literal(), true);
    assert_eq!(
        Token::Ref(vec![Token::Literal("foo".into())]).is_literal(),
        false
    );
}

#[test]
fn test_format_1() {
    assert_eq!(
        format!("{}", Token::Ref(vec![Token::literal_from_str("foo")])),
        "${foo}".to_owned(),
    );
}

#[test]
fn test_format_2() {
    assert_eq!(
        format!(
            "{}",
            Token::Combined(vec![
                Token::Ref(vec![Token::literal_from_str("foo")]),
                Token::literal_from_str("-bar")
            ])
        ),
        "${foo}-bar".to_owned(),
    );
}

#[test]
fn test_format_3() {
    assert_eq!(
        format!(
            "{}",
            Token::Combined(vec![
                Token::Ref(vec![Token::Combined(vec![
                    Token::Ref(vec![Token::literal_from_str("foo")]),
                    Token::literal_from_str(":"),
                    Token::Ref(vec![Token::literal_from_str("bar")])
                ])]),
                Token::literal_from_str("-bar")
            ])
        ),
        "${${foo}:${bar}}-bar".to_owned(),
    );
}

#[test]
fn test_format_escaped() {
    assert_eq!(
        format!(
            "{}",
            Token::Combined(vec![
                Token::Ref(vec![Token::literal_from_str("foo")]),
                Token::literal_from_str("-$bar")
            ])
        ),
        r"${foo}-\$bar".to_owned(),
    );
}

#[test]
fn test_format_double_escaped() {
    assert_eq!(
        format!(
            "{}",
            Token::Combined(vec![
                Token::Ref(vec![Token::literal_from_str("foo")]),
                Token::literal_from_str("\\"),
                Token::Ref(vec![Token::literal_from_str("bar")])
            ])
        ),
        r"${foo}\\${bar}".to_owned(),
    );
}
