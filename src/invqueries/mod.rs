use anyhow::{anyhow, Result};

use parser::parse_query;

use crate::types::{Mapping, Value};

mod parser;

struct Expression {}

pub(crate) struct Query {
    qstr: String,
    expr: Expression,
}

impl Query {
    pub(crate) fn parse(s: &str) -> Result<Self> {
        parse_query(s).map_err(|e| anyhow!("Error while parsing inventory query: {}", e))
    }

    pub(crate) fn resolve(&self, exports: &Mapping) -> Result<Value> {
        Ok(Value::Literal(self.qstr.clone()))
    }
}
