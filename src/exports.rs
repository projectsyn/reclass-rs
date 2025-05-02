use std::collections::HashMap;

use anyhow::Result;

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
        let mut exports = HashMap::new();
        for n in r.nodes.keys() {
            let node = Node::parse(r, n)?;
            assert_eq!(n, &node.meta.name);
            exports.insert(n.clone(), node);
        }
        Ok(Self {
            exports,
            reclass: Some(r),
        })
    }
}
