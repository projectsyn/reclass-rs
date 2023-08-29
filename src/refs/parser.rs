use nom::{
    branch::alt,
    bytes::complete::{tag, take},
    character::complete::none_of,
    combinator::{all_consuming, map, not, peek},
    multi::many1,
    sequence::{delimited, preceded, tuple},
    IResult,
};

use super::token::Token;

fn coalesce_literals(tokens: Vec<Token>) -> Vec<Token> {
    let mut tokiter = tokens.into_iter();
    let mut res = vec![tokiter.next().unwrap()];
    for tok in tokiter {
        if res.last().unwrap().is_literal() && tok.is_literal() {
            let t = res.pop().unwrap();
            res.push(Token::Literal(format!(
                "{}{}",
                t.as_string(),
                tok.as_string()
            )));
        } else {
            res.push(tok);
        }
    }
    res
}

fn ref_open(input: &str) -> IResult<&str, &str> {
    tag("${")(input)
}

fn ref_close(input: &str) -> IResult<&str, &str> {
    tag("}")(input)
}

fn ref_escape_open(input: &str) -> IResult<&str, String> {
    map(preceded(tag("\\"), ref_open), String::from)(input)
}

fn ref_escape_close(input: &str) -> IResult<&str, String> {
    map(preceded(tag("\\"), ref_close), String::from)(input)
}

fn double_escape(input: &str) -> IResult<&str, String> {
    map(
        tuple((tag(r"\\"), peek(alt((ref_open, ref_close))))),
        |_| r"\".to_string(),
    )(input)
}

fn ref_not_open(input: &str) -> IResult<&str, ()> {
    // don't advance parse position, just check for ref_open variants
    map(
        tuple((not(tag("${")), not(tag("\\${")), not(tag("\\\\${")))),
        |(_, _, _)| (),
    )(input)
}

fn ref_content(input: &str) -> IResult<&str, String> {
    fn ref_not_close(input: &str) -> IResult<&str, ()> {
        // don't advance parse position, just check for ref_close variants
        map(
            tuple((not(tag("}")), not(tag("\\}")), not(tag("\\\\}")))),
            |(_, _, _)| (),
        )(input)
    }

    fn ref_text(input: &str) -> IResult<&str, String> {
        alt((
            map(many1(none_of("\\${}")), |ch| ch.iter().collect::<String>()),
            map(tuple((not(tag("}")), take(1usize))), |(_, c): (_, &str)| {
                c.to_string()
            }),
        ))(input)
    }

    map(
        tuple((ref_not_open, ref_not_close, ref_text)),
        |(_, _, t)| t,
    )(input)
}

fn ref_string(input: &str) -> IResult<&str, String> {
    map(
        many1(alt((
            double_escape,
            ref_escape_open,
            ref_escape_close,
            ref_content,
        ))),
        |s| s.join(""),
    )(input)
}

fn ref_item(input: &str) -> IResult<&str, Token> {
    alt((reference, map(ref_string, Token::Literal)))(input)
}

fn reference(input: &str) -> IResult<&str, Token> {
    map(delimited(ref_open, many1(ref_item), ref_close), |tokens| {
        Token::Ref(coalesce_literals(tokens))
    })(input)
}

fn string(input: &str) -> IResult<&str, String> {
    fn text(input: &str) -> IResult<&str, String> {
        alt((
            map(many1(none_of("${}\\")), |ch| ch.iter().collect::<String>()),
            map(take(1usize), std::string::ToString::to_string),
        ))(input)
    }

    fn content(input: &str) -> IResult<&str, String> {
        map(many1(tuple((ref_not_open, text))), |strings| {
            strings
                .iter()
                .map(|((), s)| s.clone())
                .collect::<Vec<String>>()
                .join("")
        })(input)
    }

    alt((double_escape, ref_escape_open, content))(input)
}

fn item(input: &str) -> IResult<&str, Token> {
    alt((reference, map(string, Token::Literal)))(input)
}

pub fn parse_ref(input: &str) -> IResult<&str, Token> {
    map(all_consuming(many1(item)), |tokens| {
        let tokens = coalesce_literals(tokens);
        if tokens.len() > 1 {
            Token::Combined(tokens)
        } else {
            tokens.into_iter().next().unwrap()
        }
    })(input)
}

#[cfg(test)]
mod test_parser_funcs {
    use super::*;

    #[test]
    fn test_simple_ref() {
        assert_eq!(
            parse_ref(&"${foo}".to_string()),
            Ok(("", Token::Ref(vec![Token::literal_from_str("foo")])))
        );
    }

    #[test]
    fn test_parse_literal_dollar() {
        assert_eq!(
            parse_ref(&"$".to_string()),
            Ok(("", Token::literal_from_str("$")))
        );
    }

    #[test]
    fn test_parse_escape_in_literal() {
        assert_eq!(
            parse_ref(&"foo\\bar".to_string()),
            Ok(("", Token::literal_from_str("foo\\bar")))
        );
    }

    #[test]
    fn test_parse_literal_dollar_begin() {
        assert_eq!(
            parse_ref(&"$foo".to_string()),
            Ok(("", Token::literal_from_str("$foo"),))
        );
    }

    #[test]
    fn test_parse_literal_double_dollar() {
        assert_eq!(
            parse_ref(&"foo$$foo".to_string()),
            Ok(("", Token::literal_from_str("foo$$foo")))
        );
    }

    #[test]
    fn test_parse_literal_dollar_end() {
        assert_eq!(
            parse_ref(&"foo$".to_string()),
            Ok(("", Token::literal_from_str("foo$")))
        );
    }

    #[test]
    fn test_parse_literal_double_dollar_end() {
        assert_eq!(
            parse_ref(&"foo$$".to_string()),
            Ok(("", Token::literal_from_str("foo$$")))
        );
    }

    #[test]
    fn test_parse_leading_dollar_ref() {
        let refstr = r#"$${foo}"#.to_string();
        assert_eq!(
            parse_ref(&refstr),
            Ok((
                "",
                Token::Combined(vec![
                    Token::literal_from_str("$"),
                    Token::Ref(vec![Token::literal_from_str("foo")])
                ])
            ))
        )
    }

    #[test]
    fn test_parse_full_string_ref() {
        assert_eq!(
            parse_ref(&"${foo:bar:baz}".to_string()),
            Ok((
                "",
                Token::Ref(vec![Token::literal_from_str("foo:bar:baz"),])
            ))
        );
    }

    #[test]
    fn test_parse_ref_at_start() {
        assert_eq!(
            parse_ref(&"${foo}bar".to_string()),
            Ok((
                "",
                Token::Combined(vec![
                    Token::Ref(vec![Token::literal_from_str("foo")]),
                    Token::literal_from_str("bar")
                ])
            ))
        );
    }

    #[test]
    fn test_parse_ref_at_end() {
        assert_eq!(
            parse_ref(&"foo${bar}".to_string()),
            Ok((
                "",
                Token::Combined(vec![
                    Token::literal_from_str("foo"),
                    Token::Ref(vec![Token::literal_from_str("bar")])
                ])
            ))
        );
    }

    #[test]
    fn test_parse_ref_followed_by_ref() {
        assert_eq!(
            parse_ref(&"${foo}${bar}".to_string()),
            Ok((
                "",
                Token::Combined(vec![
                    Token::Ref(vec![Token::literal_from_str("foo")]),
                    Token::Ref(vec![Token::literal_from_str("bar")])
                ])
            ))
        );
    }

    #[test]
    fn test_parse_interspersed_refs() {
        assert_eq!(
            parse_ref(&"a-${foo}-${bar}-b".to_string()),
            Ok((
                "",
                Token::Combined(vec![
                    Token::literal_from_str("a-"),
                    Token::Ref(vec![Token::literal_from_str("foo")]),
                    Token::literal_from_str("-"),
                    Token::Ref(vec![Token::literal_from_str("bar")]),
                    Token::literal_from_str("-b")
                ])
            ))
        );
    }

    #[test]
    fn test_parse_nested_refs() {
        assert_eq!(
            parse_ref(&"${foo:${bar}}".to_string()),
            Ok((
                "",
                Token::Ref(vec![
                    Token::literal_from_str("foo:"),
                    Token::Ref(vec![Token::literal_from_str("bar")])
                ])
            ))
        );
    }

    #[test]
    fn test_parse_nested_refs_complex_1() {
        assert_eq!(
            parse_ref(&"${foo:${bar}:baz}".to_string()),
            Ok((
                "",
                Token::Ref(vec![
                    Token::literal_from_str("foo:"),
                    Token::Ref(vec![Token::literal_from_str("bar")]),
                    Token::literal_from_str(":baz")
                ])
            ))
        );
    }

    #[test]
    fn test_parse_nested_refs_complex_2() {
        assert_eq!(
            parse_ref(&"${foo:${bar:${baz}}}".to_string()),
            Ok((
                "",
                Token::Ref(vec![
                    Token::literal_from_str("foo:"),
                    Token::Ref(vec![
                        Token::literal_from_str("bar:"),
                        Token::Ref(vec![Token::literal_from_str("baz")])
                    ])
                ])
            ))
        );
    }

    #[test]
    fn test_parse_escaped_ref() {
        let refstr = r#"\${foo}"#.to_string();
        assert_eq!(
            parse_ref(&refstr),
            Ok(("", Token::literal_from_str("${foo}")))
        )
    }

    #[test]
    fn test_parse_escaped_ref_embedded() {
        let refstr = r#"pass \${foo}"#.to_string();
        assert_eq!(
            parse_ref(&refstr),
            Ok(("", Token::literal_from_str("pass ${foo}")))
        )
    }

    #[test]
    fn test_parse_double_escaped_ref() {
        let refstr = r#"\\${foo}"#.to_string();
        assert_eq!(
            parse_ref(&refstr),
            Ok((
                "",
                Token::Combined(vec![
                    Token::literal_from_str("\\"),
                    Token::Ref(vec![Token::literal_from_str("foo")])
                ])
            ))
        )
    }

    #[test]
    fn test_parse_escaped_ref_close() {
        let refstr = r#"${foo\}}"#.to_string();
        assert_eq!(
            parse_ref(&refstr),
            Ok(("", Token::Ref(vec![Token::literal_from_str("foo}")])))
        )
    }

    #[test]
    fn test_parse_escaped_ref_close_embedded() {
        let refstr = r#"foo$-${foo\}}-\${bar}"#.to_string();
        assert_eq!(
            parse_ref(&refstr),
            Ok((
                "",
                Token::Combined(vec![
                    Token::literal_from_str("foo$-"),
                    Token::Ref(vec![Token::literal_from_str("foo}")]),
                    Token::literal_from_str("-${bar}"),
                ])
            ))
        )
    }

    #[test]
    fn test_parse_escaped_escape_close_in_refpath() {
        let refstr = r#"${foo:${bar\}:${baz}}}"#.to_string();
        assert_eq!(
            parse_ref(&refstr),
            Ok((
                "",
                Token::Ref(vec![
                    Token::literal_from_str("foo:"),
                    Token::Ref(vec![
                        Token::literal_from_str("bar}:"),
                        Token::Ref(vec![Token::literal_from_str("baz")])
                    ])
                ])
            ))
        )
    }

    #[test]
    fn test_parse_embedded_nested_ref() {
        let refstr = r#"${foo:bar${bar}}"#.to_string();

        assert_eq!(
            parse_ref(&refstr),
            Ok((
                "",
                Token::Ref(vec![
                    Token::literal_from_str("foo:bar"),
                    Token::Ref(vec![Token::literal_from_str("bar")])
                ])
            ))
        );
    }

    #[test]
    fn test_parse_embedded_nested_ref_escaped() {
        let refstr = r#"${foo:bar\\${bar}}"#.to_string();

        assert_eq!(
            parse_ref(&refstr),
            Ok((
                "",
                Token::Ref(vec![
                    Token::literal_from_str("foo:bar\\"),
                    Token::Ref(vec![Token::literal_from_str("bar")])
                ])
            ))
        );
    }

    #[test]
    fn test_parse_incomplete_ref_error() {
        let refstr = r#"${foo:${bar}"#.to_string();

        let res = parse_ref(&refstr);
        // TODO(sg): the nom error are currently useless, figure out how to wrap them in nice parse
        // errors, maybe nom-supreme or some other wrapper/helper crate can help.
        println!("{:#?}", res);
        assert!(res.is_err());
    }

    #[test]
    fn test_parse_incomplete_ref_error_2() {
        let refstr = r#"${bar}${bar"#.to_string();

        let res = parse_ref(&refstr);
        println!("{:#?}", res);
        assert!(res.is_err());
    }

    #[test]
    fn test_parse_incomplete_ref_escaped() {
        let refstr = r#"\${foo:${bar}"#.to_string();

        assert_eq!(
            parse_ref(&refstr),
            Ok((
                "",
                Token::Combined(vec![
                    Token::literal_from_str("${foo:"),
                    Token::Ref(vec![Token::literal_from_str("bar")])
                ])
            ))
        );
    }

    #[test]
    fn test_parse_incomplete_ref_double_escaped_error() {
        let refstr = r#"\\${foo:${bar}"#.to_string();
        println!("{}", refstr);

        let res = parse_ref(&refstr);
        println!("{:#?}", res);
        assert!(res.is_err());
    }

    #[test]
    fn test_parse_unmatched_closing_brace() {
        let refstr = r#"foo}bar"#.to_string();
        assert_eq!(
            parse_ref(&refstr),
            Ok(("", Token::literal_from_str("foo}bar")))
        );
    }
}
