use anyhow::Result;
use nom::bytes::complete::{tag, tag_no_case};
use nom::IResult;

use super::{Expression, Query};

fn sign(input: &str) -> IResult<&str, &str> {
    tag("-")(input)
}

fn number(input: &str) -> IResult<&str, &str> {
    todo!()
}

fn dpoint(input: &str) -> IResult<&str, &str> {
    tag(".")(input)
}

fn ignore_errors(input: &str) -> IResult<&str, &str> {
    tag_no_case("+IgnoreErrors")(input)
}

fn all_envs(input: &str) -> IResult<&str, &str> {
    tag_no_case("+AllEnvs")(input)
}

fn eq(input: &str) -> IResult<&str, &str> {
    tag("==")(input)
}

fn neq(input: &str) -> IResult<&str, &str> {
    tag("!=")(input)
}

fn eand(input: &str) -> IResult<&str, &str> {
    tag_no_case("AND")(input)
}

fn eor(input: &str) -> IResult<&str, &str> {
    tag_no_case("OR")(input)
}

pub(super) fn parse_query(s: &str) -> Result<Query> {
    Ok(Query {
        qstr: s.to_owned(),
        expr: Expression {},
    })
}
