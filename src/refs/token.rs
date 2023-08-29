#[derive(Debug, PartialEq, Eq)]
pub enum Token {
    Literal(String),
    Ref(Vec<Token>),
    Combined(Vec<Token>),
}

impl Token {
    pub fn as_string(&self) -> String {
        match self {
            Token::Literal(s) => s.clone(),
            Token::Ref(ts) | Token::Combined(ts) => ts.iter().fold(String::new(), |mut st, t| {
                st.push_str(&t.as_string());
                st
            }),
        }
    }

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

#[cfg(test)]
mod test_token {
    use super::*;

    use crate::refs::parse_ref;

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
}
