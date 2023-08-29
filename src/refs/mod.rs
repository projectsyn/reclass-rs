mod parser;
mod token;

use nom::error::{convert_error, VerboseError};

pub use self::token::Token;

#[derive(Debug)]
pub struct ParseError<'a> {
    input: &'a str,
    nom_err: Option<VerboseError<&'a str>>,
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
}
