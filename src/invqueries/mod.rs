use anyhow::{anyhow, Result};

use parser::parse_query;

use crate::types::{Mapping, Value};

mod parser;

pub(crate) struct Query {}

impl Query {
    pub(crate) fn parse(s: &str) -> Result<Option<Self>> {
        if !s.contains("$[") {
            return Ok(None);
        }

        let query =
            parse_query(s).map_err(|e| anyhow!("Error while parsing inventory query: {}", e))?;
        Ok(Some(query))
    }

    pub(crate) fn resolve(&self, exports: &Mapping) -> Result<Value> {
        todo!()
    }
}
