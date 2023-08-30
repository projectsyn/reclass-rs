#![deny(clippy::suspicious)]
#![warn(clippy::single_match_else)]
#![warn(clippy::explicit_into_iter_loop)]
#![warn(clippy::semicolon_if_nothing_returned)]
#![warn(clippy::redundant_closure_for_method_calls)]
#![warn(let_underscore_drop)]

mod list;
mod node;
mod refs;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use crate::node::{Node, NodeInfo, NodeInfoMeta};

/// This struct holds configuration fields for various library behaviors
#[pyclass]
pub struct Reclass {
    /// Path to node definitions in inventory
    #[pyo3(get)]
    pub nodes_path: String,
    #[pyo3(get)]
    /// Path to class definitions in inventory
    pub classes_path: String,
    /// Whether to ignore included classes which don't exist (yet)
    #[pyo3(get)]
    pub ignore_class_notfound: bool,
}

#[pymethods]
impl Reclass {
    #[new]
    #[pyo3(signature = (nodes_path="./inventory/nodes", classes_path="./inventory/classes", ignore_class_notfound=false))]
    pub fn new(nodes_path: &str, classes_path: &str, ignore_class_notfound: bool) -> Self {
        Self {
            nodes_path: nodes_path.to_owned(),
            classes_path: classes_path.to_owned(),
            ignore_class_notfound,
        }
    }

    pub fn nodeinfo(&self, nodename: &str) -> PyResult<NodeInfo> {
        let n = Node::parse(self, nodename).map_err(|e| {
            PyValueError::new_err(format!("Error while processing {}: {}", nodename, e))
        })?;
        Ok(n.into())
    }
}

impl Default for Reclass {
    fn default() -> Self {
        Self::new("./inventory/nodes", "./inventory/classes", false)
    }
}

#[pymodule]
fn reclass_rs(_py: Python, m: &PyModule) -> PyResult<()> {
    // Register the top-level `Reclass` Python class which is used to configure the library
    m.add_class::<Reclass>()?;
    // Register the NodeInfoMeta and NodeInfo classes
    m.add_class::<NodeInfoMeta>()?;
    m.add_class::<NodeInfo>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reclass_new() {
        let n = Reclass::new("./inventory/nodes", "./inventory/classes", false);
        assert_eq!(n.nodes_path, "./inventory/nodes");
        assert_eq!(n.classes_path, "./inventory/classes");
        assert_eq!(n.ignore_class_notfound, false);
    }

    #[test]
    fn test_reclass_default() {
        let d = Reclass::default();
        assert_eq!(d.nodes_path, "./inventory/nodes");
        assert_eq!(d.classes_path, "./inventory/classes");
        assert_eq!(d.ignore_class_notfound, false);
    }
}
