mod parser;
mod token;

use anyhow::{anyhow, Result};

pub use self::token::Token;

#[allow(unused)]
pub fn parse_ref(input: &str) -> Result<Token> {
    use self::parser::parse_ref;
    let (uncons, token) =
        parse_ref(input).map_err(|e| anyhow!("Error parsing reference: {}", e))?;
    if !uncons.is_empty() {
        return Err(anyhow!("Failed to parse '{}': trailing {}", input, uncons));
    }
    Ok(token)
}

#[cfg(test)]
mod test_refs {
    use super::*;

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
}
