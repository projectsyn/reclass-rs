mod parser;

use anyhow::{anyhow, Result};
use nom::error::{convert_error, VerboseError};

#[derive(Debug, PartialEq, Eq)]
/// Represents a parsed Reclass reference
pub enum Token {
    /// A parsed input string which doesn't contain any Reclass references
    Literal(String),
    /// A parsed reference
    Ref(Vec<Token>),
    /// A parsed input string which is composed of one or more references, potentially with
    /// interspersed non-reference sections.
    Combined(Vec<Token>),
}

impl Token {
    /// Parses an arbitrary string into a `Token`. Returns None, if the string doesn't contain any
    /// opening reference markers.
    pub fn parse(s: &str) -> Result<Option<Self>> {
        if !s.contains("${") {
            // return None for strings which don't contain any references
            return Ok(None);
        }

        let token = parse_ref(s).map_err(|e| anyhow!("Error while parsing ref: {}", e.summary))?;
        Ok(Some(token))
    }

    #[cfg(test)]
    pub fn literal_from_str(l: &str) -> Self {
        Self::Literal(l.to_string())
    }

    /// Returns true if the Token is a `Token::Ref`
    pub fn is_ref(&self) -> bool {
        matches!(self, Self::Ref(_))
    }

    /// Returns true if the Token is a `Token::Literal`
    pub fn is_literal(&self) -> bool {
        matches!(self, Self::Literal(_))
    }
}

impl std::fmt::Display for Token {
    /// Returns the string representation of the Token.
    ///
    /// `format!("{}", parse_ref(<input string>))` should result in the original input string.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn flatten(ts: &[Token]) -> String {
            ts.iter().fold(String::new(), |mut st, t| {
                st.push_str(&format!("{t}"));
                st
            })
        }
        match self {
            Token::Literal(s) => {
                write!(f, "{}", s.clone().replace('\\', r"\\").replace('$', r"\$"))
            }
            Token::Ref(ts) => {
                let refcontent = flatten(ts);
                write!(f, "${{{refcontent}}}")
            }
            Token::Combined(ts) => write!(f, "{}", flatten(ts)),
        }
    }
}

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

/// Parses the provided input string and emits a `Token` which represents any Reclass references
/// that were found in the input string.
///
/// The function currently doesn't allow customizing the Reclass reference start and end markers,
/// or the escape character. The default Reclass reference format `${...}` and the default escape
/// character '\' are recognized by the parser.
///
/// Users should use `Token::parse()` which converts the internal `ParseError` into a format
/// suitable to be handled with `anyhow::Result`.
fn parse_ref(input: &str) -> Result<Token, ParseError> {
    use self::parser::parse_ref;
    let (uncons, token) = parse_ref(input).map_err(|e| match e {
        nom::Err::Error(e) | nom::Err::Failure(e) => ParseError {
            input,
            nom_err: Some(e),
            summary: format!("Error parsing reference '{input}'"),
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
        unreachable!(
            "Trailing data '{}' occurred when parsing '{}', this shouldn't happen! Parsed result: {}",
            uncons, input, token
        );
    };
    Ok(token)
}

#[cfg(test)]
mod token_tests;

#[cfg(test)]
mod parse_ref_tests;

