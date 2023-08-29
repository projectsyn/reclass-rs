mod parser;
mod token;

use nom::error::{convert_error, VerboseError};

pub use self::token::Token;

#[derive(Debug)]
/// Wraps errors generated when trying to parse a string which may contain Reclass references
pub struct ParseError<'a> {
    /// Holds a reference to the original input string
    input: &'a str,
    /// Holds a `nom::error::VerboseError`, if parsing failed with a `nom::Err::Error` or `nom::Err::Failure`
    nom_err: Option<VerboseError<&'a str>>,
    /// Holds a human-readable summary of the parse error
    summary: String,
}

impl<'a> std::fmt::Display for ParseError<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:\n\n", self.summary)?;
        if let Some(e) = &self.nom_err {
            write!(f, "{}", convert_error(self.input, e.clone()))?;
        }
        Ok(())
    }
}

#[allow(unused)]
/// Parses the provided input string and emits a `Token` which represents any Reclass references
/// that were found in the input string.
///
/// The function currently doesn't allow customizing the Reclass reference start and end markers,
/// or the escape character. The default Reclass reference format `${...}` and the default escape
/// character '\' are recognized by the parser.
pub fn parse_ref(input: &str) -> Result<Token, ParseError> {
    use self::parser::parse_ref;
    let (uncons, token) = parse_ref(input).map_err(|e| match e {
        nom::Err::Error(e) | nom::Err::Failure(e) => ParseError {
            input,
            nom_err: Some(e),
            summary: format!("Error parsing reference '{}'", input),
        },
        nom::Err::Incomplete(needed) => ParseError {
            input,
            nom_err: None,
            summary: format!("Failed to parse input, need more data: {needed:?}"),
        },
    })?;
    // uncons can't be empty, since we use the all_consuming combinator in the nom parser, so
    // trailing data will result in a parse error.
    if !uncons.is_empty() {
        panic!(
            "Trailing data '{}' occurred when parsing '{}', this shouldn't happen! Parsed result: {}",
            uncons, input, token
        );
    };
    Ok(token)
}

#[cfg(test)]
mod test_refs {
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
}
