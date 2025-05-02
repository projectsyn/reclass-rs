use anyhow::{anyhow, Result};

use parser::parse_query;
use serde_yaml::Number;

use crate::{
    types::{Mapping, Value},
    Exports,
};

mod parser;

type AdditionalTest = (String, Test);

#[derive(Debug)]
enum Expression {
    Expr(Test, Vec<AdditionalTest>),
}

impl Expression {
    fn evaluate(&self, exports: &Mapping, ignore_errors: bool) -> Result<bool> {
        match self {
            Self::Expr(o, rem) => {
                let res = o.evaluate(exports, ignore_errors)?;
                dbg!(&res);
                for (op, operation) in rem {
                    match &op[..] {
                        "and" => res && operation.evaluate(exports, ignore_errors)?,
                        "or" => res || operation.evaluate(exports, ignore_errors)?,
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
enum Test {
    Eq(Item, Item),
    Neq(Item, Item),
}

impl Test {
    fn make(op: &str, lhs: Item, rhs: Item) -> Result<Self> {
        match op {
            "==" => Ok(Self::Eq(lhs, rhs)),
            "!=" => Ok(Self::Neq(lhs, rhs)),
            _ => Err(anyhow!("Unexpected operator {op} in Test::make")),
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
                if let Some((ktype, kpath)) = k.split_once(':') {
                    dbg!(&ktype);
                    dbg!(&kpath);
                    let mut kparts = kpath.split(':');
                    let k0 = kparts.next().ok_or(anyhow!("expected at least one key"))?;
                    dbg!(&k0);
                    dbg!(&exports);
                    match ktype {
                        "exports" => {
                            let v = exports.get(&k0.into());
                            if ignore_errors && v.is_none() {
                                return Ok(None);
                            }
                            let mut v = v.ok_or(anyhow!("{k} doesn't exist"))?;
                            for k in kparts {
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
                    // convert true or false literals into booleans; number literals are already
                    // handled in the parser.
                    match &v[..] {
                        "true" => Ok(Some(Value::Bool(true))),
                        "false" => Ok(Some(Value::Bool(false))),
                        _ => Ok(Some(Value::Literal(v))),
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
        dbg!(&sv);
        dbg!(&ov);
        Ok(sv == ov)
    }

    fn eval_neq(&self, other: &Self, exports: &Mapping, ignore_errors: bool) -> Result<bool> {
        let sv = self.value(exports, ignore_errors)?;
        let ov = other.value(exports, ignore_errors)?;
        dbg!(&sv);
        dbg!(&ov);
        Ok(sv != ov)
    }
}

#[derive(Debug)]
pub(crate) struct Query {
    qstr: String,
    var: Option<String>,
    expr: Option<Expression>,
    all_envs: bool,
    ignore_errors: bool,
}

impl Query {
    pub(crate) fn parse(s: &str) -> Result<Self> {
        let q = s.trim();
        parse_query(q).map_err(|e| anyhow!("Error while parsing inventory query: {}", e))
    }

    // TODO(sg): properly implement +IgnoreErrors which should skip nodes that produce an error
    // when rendering the exports value. To do this right, we'll have to pass the unrendered
    // `exports` to this function and render the requested values from here.
    pub(crate) fn resolve(&self, exports: &Exports) -> Result<Value> {
        if self.all_envs {
            eprintln!(
                "Warning: reclass-rs doesn't support environments yet, `+AllEnvs` has no effect"
            );
        }
        if let Some(var) = &self.var {
            let o = Item::Obj(var.clone());
            let mut r = Mapping::new();
            for (n, node) in &exports.exports {
                let m = node
                    .merged(exports.reclass.unwrap())
                    .map_err(|e| anyhow!("while rendering exports for {n}: {e}"))?;
                let n_exports = m.get_exports();
                let nv = o
                    .value(n_exports, self.ignore_errors)
                    .map_err(|e| anyhow!("while evaluating export value for {n}: {e}"))?;
                eprintln!("Got value {nv:?} for export {} for node {n}", self.qstr);
                if let Some(e) = self.expr.as_ref() {
                    let ee = e
                        .evaluate(n_exports, self.ignore_errors)
                        .map_err(|e| anyhow!("while evaluating export expression for {n}: {e}"))?;
                    eprintln!("evaluating expr {e:?} with {n_exports:?}: {ee}");
                    if ee {
                        if let Some(nv) = nv {
                            r.insert(n.clone().into(), nv.clone())?;
                        }
                    }
                } else if let Some(nv) = nv {
                    r.insert(n.clone().into(), nv)?;
                }
            }

            Ok(Value::Mapping(r))
        } else {
            let mut r = vec![];
            if let Some(e) = self.expr.as_ref() {
                for (n, node) in &exports.exports {
                    let m = node
                        .merged(exports.reclass.unwrap())
                        .map_err(|e| anyhow!("while rendering exports for {n}: {e}"))?;
                    if e.evaluate(m.get_exports(), self.ignore_errors)? {
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
    use std::path::PathBuf;

    use super::Query;
    use crate::exports::Exports;
    use crate::node::Node;
    use crate::types::{Mapping, Value};
    use crate::{NodeInfoMeta, Reclass};

    fn make_node(name: &str, contents: &str) -> Node {
        let mut npath = PathBuf::new();
        npath.set_file_name(format!("{name}.yml"));
        Node::from_str(
            NodeInfoMeta::new(
                name,
                name,
                &format!("tmp://{name}"),
                npath.clone(),
                npath,
                "base",
            ),
            None,
            contents,
        )
        .unwrap()
    }

    #[test]
    fn test_resolve_simple_1() {
        let qstr = " exports:foo ";
        let q = Query::parse(qstr).unwrap();
        let mut exports = Exports::default();
        let r = Reclass::new("./tests/inventory", "nodes", "classes", false).unwrap();
        println!("{:?}", r.nodes);
        exports.reclass = Some(&r);
        let mut expected = Mapping::new();
        for n in ["n1", "n2", "n3"] {
            let node = make_node(
                n,
                r#"
                exports:
                  foo: bar
            "#,
            );
            exports.exports.insert(n.into(), node);
            expected
                .insert(n.into(), Value::Literal("bar".to_owned()))
                .unwrap();
        }
        let v = q.resolve(&exports).unwrap();

        assert_eq!(v, Value::Mapping(expected));
    }

    #[test]
    fn test_resolve_simple_ignore_errors_1() {
        let qstr = " +IgnoreErrors exports:n1 ";
        let q = Query::parse(qstr).unwrap();
        let mut exports = Exports::default();
        let r = Reclass::new("./tests/inventory", "nodes", "classes", false).unwrap();
        exports.reclass = Some(&r);
        let mut expected = Mapping::new();
        for n in ["n1", "n2", "n3"] {
            let node = make_node(
                n,
                &format!(
                    r#"
                exports:
                  {n}: bar
            "#
                ),
            );
            exports.exports.insert(n.into(), node);
            if n == "n1" {
                expected
                    .insert(n.into(), Value::Literal("bar".to_owned()))
                    .unwrap();
            }
        }
        let v = q.resolve(&exports).unwrap();

        assert_eq!(v, Value::Mapping(expected));
    }

    #[test]
    fn test_resolve_simple_errors_1() {
        let qstr = " exports:n1 ";
        let q = Query::parse(qstr).unwrap();
        let mut exports = Exports::default();
        let r = Reclass::new("./tests/inventory", "nodes", "classes", false).unwrap();
        exports.reclass = Some(&r);
        for n in ["n1", "n2", "n3"] {
            let node = make_node(
                n,
                &format!(
                    r#"
                exports:
                  {n}: bar
            "#
                ),
            );
            exports.exports.insert(n.into(), node);
        }
        let v = q.resolve(&exports);
        assert!(v.is_err());
    }

    #[test]
    fn test_resolve_cond_1() {
        let qstr = " exports:foo if exports:foo != n3 ";
        let q = Query::parse(qstr).unwrap();
        let mut exports = Exports::default();
        let r = Reclass::new("./tests/inventory", "nodes", "classes", false).unwrap();
        exports.reclass = Some(&r);
        let mut expected = Mapping::new();
        for n in ["n1", "n2", "n3"] {
            let node = make_node(
                n,
                &format!(
                    r#"
                exports:
                  foo: {n}
            "#
                ),
            );
            exports.exports.insert(n.into(), node);
            if n != "n3" {
                expected
                    .insert(n.into(), Value::Literal(n.to_owned()))
                    .unwrap();
            }
        }
        let v = q.resolve(&exports).unwrap();

        assert_eq!(v, Value::Mapping(expected));
    }
}
