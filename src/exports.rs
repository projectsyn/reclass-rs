use std::collections::HashMap;

use anyhow::Result;
use rayon::prelude::*;

use crate::node::Node;
use crate::Reclass;

#[derive(Debug, Default)]
pub struct Exports<'a> {
    pub(crate) exports: HashMap<String, Node>,
    pub(crate) reclass: Option<&'a Reclass>,
}

impl<'a> Exports<'a> {
    // TODO(sg): ensure that we do proper ref lookups when rendering exports
    pub(crate) fn new(r: &'a Reclass) -> Result<Self> {
        let nodes: Vec<_> = r
            .nodes
            .par_iter()
            .map(|(n, _)| -> Result<(String, Node)> {
                let node = Node::parse(r, n)?;
                assert_eq!(n, &node.meta.name);
                Ok((n.clone(), node))
            })
            .collect();
        let mut exports = HashMap::new();
        for v in nodes {
            let (name, node) = v?;
            exports.insert(name, node);
        }
        Ok(Self {
            exports,
            reclass: Some(r),
        })
    }
}
