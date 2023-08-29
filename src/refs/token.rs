#[derive(Debug, PartialEq, Eq)]
pub enum Token {
    Literal(String),
    Ref(Vec<Token>),
    Combined(Vec<Token>),
}

impl Token {
    #[cfg(test)]
    pub fn literal_from_str(l: &str) -> Self {
        Self::Literal(l.to_string())
    }

    #[allow(unused)]
    pub fn is_ref(&self) -> bool {
        matches!(self, Self::Ref(_))
    }

    pub fn is_literal(&self) -> bool {
        matches!(self, Self::Literal(_))
    }
}

impl std::fmt::Display for Token {
    /// Returns the string representation of the Token.
    ///
    /// format!("{}", parse_ref(<input string>)) should result in the original input string.
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

#[cfg(test)]
mod test_token {
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
}
