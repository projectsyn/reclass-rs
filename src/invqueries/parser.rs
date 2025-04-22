use anyhow::{anyhow, Result};
use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case},
    character::complete::{digit1, none_of, one_of},
    combinator::{all_consuming, map, map_res, recognize},
    multi::{many0, many1},
    number::complete::double,
    sequence::{preceded, terminated, tuple},
    IResult,
};

use super::{Expression, Item, Operator, Query, QueryOption, Test};

fn sign(input: &str) -> IResult<&str, &str> {
    tag("-")(input)
}

fn integer(input: &str) -> IResult<&str, Item> {
    map(
        alt((
            map_res(preceded(sign, digit1), |n| {
                i64::from_str_radix(n, 10).map(|v| -v)
            }),
            map_res(digit1, |n| i64::from_str_radix(n, 10)),
        )),
        |i| Item::Integer(i),
    )(input)
}

fn real(input: &str) -> IResult<&str, Item> {
    map(double, |d| Item::Real(d))(input)
}

fn ignore_errors(input: &str) -> IResult<&str, QueryOption> {
    map(tag_no_case("+IgnoreErrors"), |_| QueryOption::IgnoreErrors)(input)
}

fn all_envs(input: &str) -> IResult<&str, QueryOption> {
    map(tag_no_case("+AllEnvs"), |_| QueryOption::AllEnvs)(input)
}

fn eq(input: &str) -> IResult<&str, Operator> {
    map(tag("=="), |_| Operator::Eq)(input)
}

fn neq(input: &str) -> IResult<&str, Operator> {
    map(tag("!="), |_| Operator::Neq)(input)
}

fn and(input: &str) -> IResult<&str, Operator> {
    map(tag_no_case("AND"), |_| Operator::And)(input)
}

fn or(input: &str) -> IResult<&str, Operator> {
    map(tag_no_case("OR"), |_| Operator::Or)(input)
}

fn whitespace(input: &str) -> IResult<&str, &str> {
    recognize(many1(one_of(" \t")))(input)
}

fn options(input: &str) -> IResult<&str, Vec<QueryOption>> {
    many0(alt((
        terminated(ignore_errors, whitespace),
        terminated(all_envs, whitespace),
    )))(input)
}

fn operator_test(input: &str) -> IResult<&str, Operator> {
    alt((eq, neq))(input)
}

fn operator_logical(input: &str) -> IResult<&str, Operator> {
    alt((and, or))(input)
}

fn begin_if(input: &str) -> IResult<&str, &str> {
    dbg!(&input);
    tag_no_case("IF")(input)
}

fn obj(input: &str) -> IResult<&str, Item> {
    dbg!("obj");
    dbg!(&input);
    map(recognize(many1(none_of(" \t"))), |s: &str| {
        Item::Obj(s.to_owned())
    })(input)
}

fn expritem(input: &str) -> IResult<&str, Item> {
    dbg!("expritem");
    dbg!(&input);
    alt((integer, real, obj))(input)
}

fn single_test(input: &str) -> IResult<&str, Test> {
    map_res(
        tuple((
            expritem,
            preceded(whitespace, operator_test),
            preceded(whitespace, expritem),
        )),
        |(a, op, b)| Test::make(op, a, b),
    )(input)
}

fn additional_test(input: &str) -> IResult<&str, (Operator, Test)> {
    tuple((operator_logical, preceded(whitespace, single_test)))(input)
}

fn expr_var(input: &str) -> IResult<&str, (Option<String>, Option<Expression>)> {
    map_res(all_consuming(obj), |o| match o {
        Item::Obj(v) => Ok((Some(v), None)),
        _ => Err(anyhow!("expr_var should only match Item::Obj, got {o:?}")),
    })(input)
}

fn expr_test(input: &str) -> IResult<&str, (Option<String>, Option<Expression>)> {
    map_res(
        all_consuming(tuple((
            obj,
            preceded(whitespace, begin_if),
            preceded(whitespace, single_test),
            many0(preceded(whitespace, additional_test)),
        ))),
        |(v, _, t, ts)| {
            let Item::Obj(var) = v else {
                return Err(anyhow!("Expected value to be an Item::Obj, got {v:?}"));
            };
            let expr = Expression::Expr(t, ts);
            Ok((Some(var), Some(expr)))
        },
    )(input)
}

fn expr_list_test(input: &str) -> IResult<&str, (Option<String>, Option<Expression>)> {
    map(
        all_consuming(tuple((
            begin_if,
            preceded(whitespace, single_test),
            many0(preceded(whitespace, additional_test)),
        ))),
        |(_, t, ts)| {
            let expr = Expression::Expr(t, ts);
            (None, Some(expr))
        },
    )(input)
}

fn line(input: &str) -> IResult<&str, (Vec<QueryOption>, (Option<String>, Option<Expression>))> {
    tuple((options, alt((expr_test, expr_var, expr_list_test))))(input)
}

pub(super) fn parse_query(s: &str) -> Result<Query> {
    let (uncons, (opts, (var, expr))) =
        line(s).map_err(|e| anyhow!("While parsing inventory query: {e}"))?;
    if uncons != "" {
        return Err(anyhow!("Parsing inventory query didn't consume '{uncons}'"));
    }

    let all_envs = opts.contains(&QueryOption::AllEnvs);
    let ignore_errors = opts.contains(&QueryOption::IgnoreErrors);
    Ok(Query {
        qstr: s.to_owned(),
        var,
        expr,
        all_envs,
        ignore_errors,
    })
}

#[cfg(test)]
mod tests {
    use super::parse_query;

    #[test]
    fn parse_simple() {
        let qstr = "exports:foo";
        let q = parse_query(qstr).unwrap();
        assert_eq!(q.qstr, qstr);
        assert_eq!(q.var, Some("exports:foo".to_owned()));
        assert!(q.expr.is_none());
        assert_eq!(q.all_envs, false);
        assert_eq!(q.ignore_errors, false);
    }

    #[test]
    fn parse_simple_expr() {
        let qstr = "exports:foo if exports:foo == bar";
        let q = parse_query(qstr).unwrap();
        assert_eq!(q.qstr, qstr);
        assert_eq!(q.var, Some("exports:foo".to_owned()));
        assert!(q.expr.is_some());
        assert_eq!(q.all_envs, false);
        assert_eq!(q.ignore_errors, false);
    }

    #[test]
    fn parse_option_all_envs() {
        let qstr = "+AllEnvs exports:foo";
        let q = parse_query(qstr).unwrap();
        assert_eq!(q.qstr, qstr);
        assert_eq!(q.var, Some("exports:foo".to_owned()));
        assert!(q.expr.is_none());
        assert_eq!(q.all_envs, true);
        assert_eq!(q.ignore_errors, false);
    }

    #[test]
    fn parse_option_ignore_errors() {
        let qstr = "+IgnoreErrors exports:foo";
        let q = parse_query(qstr).unwrap();
        assert_eq!(q.qstr, qstr);
        assert_eq!(q.var, Some("exports:foo".to_owned()));
        assert!(q.expr.is_none());
        assert_eq!(q.all_envs, false);
        assert_eq!(q.ignore_errors, true);
    }

    #[test]
    fn parse_options_multi() {
        let qstr = "+IgnoreErrors +AllEnvs +IgnoreErrors exports:foo";
        let q = parse_query(qstr).unwrap();
        assert_eq!(q.qstr, qstr);
        assert_eq!(q.var, Some("exports:foo".to_owned()));
        assert!(q.expr.is_none());
        assert_eq!(q.all_envs, true);
        assert_eq!(q.ignore_errors, true);
    }

    #[test]
    fn parse_no_value_expr() {
        let qstr = "if exports:foo == bar";
        let q = parse_query(qstr).unwrap();
        assert_eq!(q.qstr, qstr);
        assert_eq!(q.var, None);
        assert!(q.expr.is_some());
        assert_eq!(q.all_envs, false);
        assert_eq!(q.ignore_errors, false);
    }
}
