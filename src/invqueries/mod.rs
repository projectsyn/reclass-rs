use anyhow::{anyhow, Result};

use parser::parse_query;
use serde_yaml::Number;

use crate::{
    types::{Mapping, Value},
    Exports,
};

mod parser;

#[derive(Debug)]
enum Expression {
    Expr(Test, Vec<(Operator, Test)>),
}

impl Expression {
    fn evaluate(&self, exports: &Exports, ignore_errors: bool) -> Result<bool> {
        match self {
            Self::Expr(o, rem) => {
                let res = o.evaluate(exports, ignore_errors)?;
                dbg!(&res);
                for (op, operation) in rem {
                    match op {
                        Operator::And => res && operation.evaluate(exports, ignore_errors)?,
                        Operator::Or => res || operation.evaluate(exports, ignore_errors)?,
                        _ => unreachable!("unexpected test op"),
                    };
                }
                Ok(res)
            }
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
enum QueryOption {
    AllEnvs,
    IgnoreErrors,
}

#[derive(Debug)]
enum Operator {
    Eq,
    Neq,
    And,
    Or,
}

#[derive(Debug)]
enum Test {
    Eq(Item, Item),
    Neq(Item, Item),
}

impl Test {
    fn make(op: Operator, lhs: Item, rhs: Item) -> Result<Self> {
        match op {
            Operator::Eq => Ok(Self::Eq(lhs, rhs)),
            Operator::Neq => Ok(Self::Neq(lhs, rhs)),
            _ => Err(anyhow!("Unexpected operator {op:?} in Test::make")),
        }
    }

    fn evaluate(&self, exports: &Exports, ignore_errors: bool) -> Result<bool> {
        match self {
            Self::Eq(a, b) => a.eval_eq(b, exports, ignore_errors),
            Self::Neq(a, b) => a.eval_neq(b, exports, ignore_errors),
        }
    }
}

#[derive(Debug)]
enum Item {
    Integer(i64),
    Real(f64),
    Obj(String),
}

impl Item {
    fn value(&self, exports: &Exports, ignore_errors: bool) -> Result<Option<Value>> {
        match self {
            Self::Obj(k) => {
                if let Some((ktype, kpath)) = k.split_once(":") {
                    dbg!(&ktype);
                    dbg!(&kpath);
                    let mut kparts = kpath.split(":");
                    let k0 = kparts.next().ok_or(anyhow!("expected at least one key"))?;
                    dbg!(&k0);
                    dbg!(&exports);
                    match ktype {
                        "exports" => {
                            let v = exports.exports.get(&k0.into());
                            if ignore_errors && v.is_none() {
                                return Ok(None);
                            }
                            let mut v = v.ok_or(anyhow!("{k} doesn't exist"))?;
                            while let Some(k) = kparts.next() {
                                let n = v.get(&k.into());
                                if ignore_errors && n.is_none() {
                                    return Ok(None);
                                }
                                v = n.ok_or(anyhow!(
                                    "Failed to get export {kpath}, {k} doesn't exist"
                                ))?;
                            }
                            Ok(Some(v.clone()))
                        }
                        "self" => todo!("self references in exports NYI"),
                        _ => Err(anyhow!("Unexpected lookup type {ktype}")),
                    }
                } else {
                    let v = k.to_lowercase();
                    // true or false literals
                    match &v[..] {
                        "true" => Ok(Some(Value::Bool(true))),
                        "false" => Ok(Some(Value::Bool(true))),
                        _ => Ok(Some(Value::Literal(v))),
                    }
                }
            }
            &Self::Integer(n) => Ok(Some(Value::Number(Number::from(n)))),
            &Self::Real(n) => Ok(Some(Value::Number(Number::from(n)))),
        }
    }

    fn eval_eq(&self, other: &Self, exports: &Exports, ignore_errors: bool) -> Result<bool> {
        // TODO(sg): figure out how to properly propagate missing values on `ignore_errors`
        // what's the semantics of ignore_errors, do we silently skip the output for failing
        // lookups?
        let sv = self.value(exports, ignore_errors)?;
        let ov = other.value(exports, ignore_errors)?;
        Ok(sv == ov)
    }

    fn eval_neq(&self, other: &Self, exports: &Exports, ignore_errors: bool) -> Result<bool> {
        let sv = self.value(exports, ignore_errors)?;
        let ov = other.value(exports, ignore_errors)?;
        Ok(sv != ov)
    }
}

#[derive(Debug)]
pub(crate) struct Query {
    qstr: String,
    var: Option<String>,
    expr: Option<Expression>,
    // TODO(sg): figure out what these really do and implement them correctly
    all_envs: bool,
    ignore_errors: bool,
}

impl Query {
    pub(crate) fn parse(s: &str) -> Result<Self> {
        let q = s.trim();
        parse_query(q).map_err(|e| anyhow!("Error while parsing inventory query: {}", e))
    }

    // exports has structure
    // key1:
    //  node1: value1
    //  node2: value1
    // key2:
    //  node1: value2
    //  node2: value2
    pub(crate) fn resolve(&self, exports: &Exports) -> Result<Value> {
        if let Some(var) = &self.var {
            let o = Item::Obj(var.clone());
            let mut r = Mapping::new();
            if let Some(v) = o.value(exports, self.ignore_errors)? {
                if let Some(e) = self.expr.as_ref() {
                    for (n, nv) in v
                        .as_mapping()
                        .ok_or(anyhow!("expected inv query result to be a mapping"))?
                    {
                        if e.evaluate(exports, self.ignore_errors)? {
                            r.insert(n.clone(), nv.clone()).unwrap();
                        }
                    }
                } else {
                    return Ok(v);
                }
            };

            Ok(Value::Mapping(r))
        } else {
            let mut r = vec![];
            if let Some(e) = self.expr.as_ref() {
                for n in &exports.nodes {
                    if e.evaluate(exports, self.ignore_errors)? {
                        r.push(n.clone().into());
                    }
                }
            }
            Ok(Value::Sequence(r))
        }
    }
}

#[cfg(test)]
mod invqueries_test {
    use super::Query;
    use crate::exports::Exports;
    use crate::types::{Mapping, Value};

    #[test]
    fn test_resolve_simple_1() {
        let qstr = " exports:foo ";
        let q = Query::parse(qstr).unwrap();
        let mut m = Mapping::new();
        let mut expected = Mapping::new();
        for n in ["n1", "n2", "n3"] {
            m.insert(n.into(), "bar".into()).unwrap();
            expected
                .insert(n.into(), Value::String("bar".to_owned()))
                .unwrap();
        }
        let mut exports = Exports::default();
        exports
            .exports
            .insert("foo".into(), Value::Mapping(m))
            .unwrap();
        let v = q.resolve(&exports).unwrap();

        assert_eq!(v, Value::Mapping(expected));
    }
}
