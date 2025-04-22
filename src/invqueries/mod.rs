use anyhow::{anyhow, Result};

use parser::parse_query;
use serde_yaml::Number;

use crate::types::{Mapping, Value};

mod parser;

#[derive(Debug)]
enum Expression {
    Expr(Test, Vec<(Operator, Test)>),
}

impl Expression {
    fn evaluate(&self, exports: &Mapping, ignore_errors: bool) -> Result<bool> {
        match self {
            Self::Expr(o, rem) => {
                let res = o.evaluate(exports, ignore_errors)?;
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

    fn evaluate(&self, exports: &Mapping, ignore_errors: bool) -> Result<bool> {
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
    fn value(&self, exports: &Mapping, ignore_errors: bool) -> Result<Option<Value>> {
        match self {
            Self::Obj(k) => {
                if let Some((ktype, kpath)) = k.split_once(":") {
                    let mut kparts = kpath.split(":");
                    let k0 = kparts.next().ok_or(anyhow!("expected at least one key"))?;
                    match ktype {
                        "exports" => {
                            let v = exports.get(&k0.into());
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
                        _ => Err(anyhow!("Unexpected literal {k}")),
                    }
                }
            }
            &Self::Integer(n) => Ok(Some(Value::Number(Number::from(n)))),
            &Self::Real(n) => Ok(Some(Value::Number(Number::from(n)))),
        }
    }

    fn eval_eq(&self, other: &Self, exports: &Mapping, ignore_errors: bool) -> Result<bool> {
        // TODO(sg): figure out how to properly propagate missing values on `ignore_errors`
        // what's the semantics of ignore_errors, do we silently skip the output for failing
        // lookups?
        let sv = self.value(exports, ignore_errors)?;
        let ov = other.value(exports, ignore_errors)?;
        Ok(sv == ov)
    }

    fn eval_neq(&self, other: &Self, exports: &Mapping, ignore_errors: bool) -> Result<bool> {
        let sv = self.value(exports, ignore_errors)?;
        let ov = other.value(exports, ignore_errors)?;
        Ok(sv != ov)
    }
}

pub(crate) struct Query {
    qstr: String,
    var: Option<String>,
    expr: Option<Expression>,
    // TODO(sg): figure out what these do and implement them
    all_envs: bool,
    ignore_errors: bool,
}

impl Query {
    pub(crate) fn parse(s: &str) -> Result<Self> {
        parse_query(s).map_err(|e| anyhow!("Error while parsing inventory query: {}", e))
    }

    pub(crate) fn resolve(&self, exports: &Mapping) -> Result<Value> {
        if let Some(var) = &self.var {
            let o = Item::Obj(var.clone());
            let mut v = Mapping::new();
            for (n, n_exports) in exports.as_map() {
                let n_exports = n_exports.as_mapping().ok_or(anyhow!(
                    "expected exports to be mapping for {n}, got {}",
                    n_exports.variant()
                ))?;
                if let Some(e) = self.expr.as_ref() {
                    //  TODO(sg): do we resolve exports in expr with only our own exports? or with all
                    //  of them?
                    if !e.evaluate(n_exports, self.ignore_errors)? {
                        continue;
                    }
                }
                let n_v = o.value(n_exports, self.ignore_errors)?;
                if let Some(n_v) = n_v {
                    v.insert(n.clone(), n_v)?;
                }
            }
            Ok(Value::Mapping(v))
        } else {
            // for queries without a value, we return all nodes for which the expression evaluates
            // to true.
            let mut v = vec![];
            for (n, n_exports) in exports.as_map() {
                let n_exports = n_exports.as_mapping().ok_or(anyhow!(
                    "expected exports to be mapping for {n}, got {}",
                    n_exports.variant()
                ))?;
                if let Some(e) = self.expr.as_ref() {
                    //  TODO(sg): do we resolve exports in expr with only our own exports? or with all
                    //  of them?
                    if !e.evaluate(n_exports, self.ignore_errors)? {
                        continue;
                    }
                }
                v.push(n.clone());
            }
            Ok(Value::Sequence(v))
        }
    }
}
