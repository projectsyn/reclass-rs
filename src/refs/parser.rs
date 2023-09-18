use nom::{
    branch::alt,
    bytes::complete::{tag, take},
    character::complete::none_of,
    combinator::{all_consuming, map, not, peek},
    error::{context, VerboseError},
    multi::many1,
    sequence::{delimited, preceded, tuple},
    IResult,
};

use super::Token;

/// Merges adjacent literal tokens into a single literal token to reduce the number of tokens in
/// parsed references.
fn coalesce_literals(tokens: Vec<Token>) -> Vec<Token> {
    let mut tokiter = tokens.into_iter();
    let mut res = vec![tokiter.next().unwrap()];
    for tok in tokiter {
        if res.last().unwrap().is_literal() && tok.is_literal() {
            // TODO(sg): Move the if-let bindings into the if above this comment once the
            // corresponding Rust feature is stabilized.
            if let Token::Literal(t) = res.pop().unwrap() {
                if let Token::Literal(tok) = tok {
                    res.push(Token::Literal(format!("{t}{tok}")));
                } else {
                    unreachable!("Literal token isn't a literal?");
                }
            } else {
                unreachable!("Literal token isn't a literal?");
            }
        } else {
            res.push(tok);
        }
    }
    res
}

fn ref_open(input: &str) -> IResult<&str, &str, VerboseError<&str>> {
    context("ref_open", tag("${"))(input)
}

fn ref_close(input: &str) -> IResult<&str, &str, VerboseError<&str>> {
    context("ref_close", tag("}"))(input)
}

fn inv_open(input: &str) -> IResult<&str, &str, VerboseError<&str>> {
    context("inv_open", tag("$["))(input)
}

fn ref_escape_open(input: &str) -> IResult<&str, String, VerboseError<&str>> {
    map(
        context("ref_escape_open", preceded(tag("\\"), ref_open)),
        String::from,
    )(input)
}

fn inv_escape_open(input: &str) -> IResult<&str, String, VerboseError<&str>> {
    map(
        context("inv_escape_open", preceded(tag("\\"), inv_open)),
        String::from,
    )(input)
}

fn ref_escape_close(input: &str) -> IResult<&str, String, VerboseError<&str>> {
    map(
        context("ref_escape_close", preceded(tag("\\"), ref_close)),
        String::from,
    )(input)
}

fn double_escape(input: &str) -> IResult<&str, String, VerboseError<&str>> {
    map(
        context(
            "double_escape",
            tuple((tag(r"\\"), peek(alt((ref_open, ref_close))))),
        ),
        |_| r"\".to_string(),
    )(input)
}

fn ref_not_open(input: &str) -> IResult<&str, (), VerboseError<&str>> {
    // don't advance parse position, just check for ref_open variants
    map(
        context(
            "ref_not_open",
            tuple((
                not(tag("${")),
                not(tag("\\${")),
                not(tag("\\\\${")),
                not(tag("\\$[")),
            )),
        ),
        |_| (),
    )(input)
}

/// Parses a section of the input which can't contain a reference (escaped or otherwise)
fn ref_content(input: &str) -> IResult<&str, String, VerboseError<&str>> {
    fn ref_not_close(input: &str) -> IResult<&str, (), VerboseError<&str>> {
        // don't advance parse position, just check for ref_close variants
        map(
            context(
                "ref_not_close",
                tuple((not(tag("}")), not(tag("\\}")), not(tag("\\\\}")))),
            ),
            |(_, _, _)| (),
        )(input)
    }

    fn ref_text(input: &str) -> IResult<&str, String, VerboseError<&str>> {
        context(
            "ref_text",
            alt((
                map(many1(none_of("\\${}")), |ch| ch.iter().collect::<String>()),
                map(tuple((not(tag("}")), take(1usize))), |(_, c): (_, &str)| {
                    c.to_string()
                }),
            )),
        )(input)
    }

    map(
        context(
            "ref_content",
            tuple((ref_not_open, ref_not_close, ref_text)),
        ),
        |(_, _, t)| t,
    )(input)
}

/// Parses a section of the contents of a reference which doesn't contain nested Reclass
/// references, taking into account escaped reference start markers
fn ref_string(input: &str) -> IResult<&str, String, VerboseError<&str>> {
    map(
        context(
            "ref_string",
            many1(alt((
                double_escape,
                ref_escape_open,
                ref_escape_close,
                inv_escape_open,
                ref_content,
            ))),
        ),
        |s| s.join(""),
    )(input)
}

/// Parses the contents of a reference, taking into account that there may be nested references
fn ref_item(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
    context(
        "ref_item",
        alt((reference, map(ref_string, Token::Literal))),
    )(input)
}

/// Parses a single Reclass reference which may contain nested references
fn reference(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
    context(
        "reference",
        map(delimited(ref_open, many1(ref_item), ref_close), |tokens| {
            Token::Ref(coalesce_literals(tokens))
        }),
    )(input)
}

/// Parses a section of the input which doesn't contain any Reclass references
fn string(input: &str) -> IResult<&str, String, VerboseError<&str>> {
    fn text(input: &str) -> IResult<&str, String, VerboseError<&str>> {
        context(
            "text",
            alt((
                map(many1(none_of("${}\\")), |ch| ch.iter().collect::<String>()),
                map(take(1usize), std::string::ToString::to_string),
            )),
        )(input)
    }

    fn content(input: &str) -> IResult<&str, String, VerboseError<&str>> {
        context(
            "content",
            map(many1(tuple((ref_not_open, text))), |strings| {
                strings.iter().map(|((), s)| s.clone()).collect::<String>()
            }),
        )(input)
    }

    context(
        "string",
        alt((double_escape, ref_escape_open, inv_escape_open, content)),
    )(input)
}

/// Parses either a Reclass reference or a section of the input with no references
fn item(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
    context("item", alt((reference, map(string, Token::Literal))))(input)
}

/// Parses a string containing zero or more Reclass references
pub fn parse_ref(input: &str) -> IResult<&str, Token, VerboseError<&str>> {
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

    #[test]
    fn test_parse_escape_then_double_escaped_ref() {
        // Reclass's reference parsing only requires escaping backslashes that should be literals
        // when they precede a reference opening or closing symbol. Other backslashes don't need to
        // be escaped. The parser will try to parse backslashes as single characters first, and
        // will only interpret them as escape characters when they precede a reference opening or
        // closing symbol.
        //
        // Therefore the string `\\\${foo}` is parsed as a freestanding `\` followed by a
        // double-escaped reference resulting in `\\` followed by the contents of `${foo}` once
        // interpolated.
        let refstr = r#"\\\${foo}"#.to_string();
        assert_eq!(
            parse_ref(&refstr),
            Ok((
                "",
                Token::Combined(vec![
                    Token::literal_from_str(r"\\"),
                    Token::Ref(vec![Token::literal_from_str("foo")])
                ])
            ))
        )
    }

    #[test]
    fn test_parse_escape_escape_then_double_escaped_ref() {
        // Reclass's reference parsing only requires escaping backslashes that should be literals
        // when they precede a reference opening or closing symbol. Other backslashes don't need to
        // be escaped. The parser will try to parse backslashes as single characters first, and
        // will only interpret them as escape characters when they precede a reference opening or
        // closing symbol.
        //
        // Therefore the string `\\\\${foo}` is parsed as two freestanding `\` followed by a
        // double-escaped reference resulting in `\\\` followed by the contents of `${foo}` once
        // interpolated.
        let refstr = r#"\\\\${foo}"#.to_string();
        assert_eq!(
            parse_ref(&refstr),
            Ok((
                "",
                Token::Combined(vec![
                    Token::literal_from_str(r"\\\"),
                    Token::Ref(vec![Token::literal_from_str("foo")])
                ])
            ))
        )
    }

    #[test]
    fn test_parse_escape_then_double_escaped_ref_close() {
        // Reclass's reference parsing only requires escaping backslashes that should be literals
        // when they precede a reference opening or closing symbol. Other backslashes don't need to
        // be escaped. The parser will try to parse backslashes as single characters first, and
        // will only interpret them as escape characters when they precede a reference opening or
        // closing symbol.
        //
        // Therefore the string `${foo\\\}` is parsed as a reference to `foo\\`. The first `\` in
        // the reference is parsed as a freestanding `\` and the following `\\` is parsed as a
        // double-escaped reference closing symbol.
        let refstr = r#"${foo\\\}"#.to_string();
        assert_eq!(
            parse_ref(&refstr),
            Ok(("", Token::Ref(vec![Token::literal_from_str(r"foo\\")])))
        )
    }

    #[test]
    fn test_parse_inventory_query_escape() {
        // To ensure compatibility with Python reclass's reference parser, we parse `\$[` as `$[`
        // even though we don't support inventory queries yet.
        let refstr = r#"\$['foo']['bar']"#;
        assert_eq!(
            parse_ref(&refstr),
            Ok(("", Token::literal_from_str(r"$['foo']['bar']")))
        )
    }

    #[test]
    fn test_parse_inventory_query_escaped_embedded() {
        // To ensure compatibility with Python reclass's reference parser, we parse `\$[` as `$[`
        // even though we don't support inventory queries yet.
        let refstr = r#"foo: \$['foo']['bar']"#;
        assert_eq!(
            parse_ref(&refstr),
            Ok(("", Token::literal_from_str(r"foo: $['foo']['bar']")))
        )
    }

    #[test]
    fn test_parse_inventory_query() {
        // Non-escaped inventory queries are also parsed as literals.
        let refstr = r#"$[foo:bar]"#;
        assert_eq!(
            parse_ref(&refstr),
            Ok(("", Token::literal_from_str(r"$[foo:bar]")))
        )
    }

    #[test]
    fn test_parse_inventory_query_double_escape() {
        // Double-escaped inventory query is parsed as `\` followed by escaped inventory query.
        let refstr = r#"\\$[foo:bar]"#;
        assert_eq!(
            parse_ref(&refstr),
            Ok(("", Token::literal_from_str(r"\$[foo:bar]")))
        )
    }
}
