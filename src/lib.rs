#![deny(clippy::suspicious)]
#![warn(clippy::explicit_into_iter_loop)]
#![warn(clippy::redundant_closure_for_method_calls)]
#![warn(clippy::semicolon_if_nothing_returned)]
#![warn(clippy::single_match_else)]
#![warn(clippy::uninlined_format_args)]
#![warn(let_underscore_drop)]

mod list;
mod node;
mod refs;
pub mod types;

use anyhow::{anyhow, Result};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf, MAIN_SEPARATOR};
use walkdir::WalkDir;

use crate::node::{Node, NodeInfo, NodeInfoMeta};

const SUPPORTED_YAML_EXTS: [&str; 2] = ["yml", "yaml"];

/// This struct holds configuration fields for various library behaviors
#[pyclass]
#[derive(Clone, Debug)]
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
    classes: HashMap<String, PathBuf>,
}

impl Reclass {
    /// Discover all classes in `r.classes_path` and store the resulting list in `r.known_classes`.
    ///
    /// This method will raise an error if multiple classes which resolve to the same absolute
    /// class name exist (e.g. classes `foo..bar.yml` and `foo/.bar.yml` are both included as
    /// `foo..bar`).
    fn discover_classes(&mut self) -> Result<()> {
        fn stringify(p: &Path) -> Result<&str> {
            p.to_str()
                .ok_or(anyhow!("Failed to convert {} to string", p.display()))
        }
        let class_root = PathBuf::from(&self.classes_path).canonicalize()?;

        for entry in WalkDir::new(&self.classes_path) {
            let entry = entry?;
            let ext = if let Some(ext) = entry.path().extension() {
                ext.to_str()
            } else {
                None
            };
            if ext.is_some() && SUPPORTED_YAML_EXTS.contains(&ext.unwrap()) {
                // it's a class, process it
                let abspath = entry.path().canonicalize()?;
                let relpath = abspath.strip_prefix(&class_root)?;
                let cls = relpath
                    .with_extension("")
                    .to_str()
                    .ok_or(anyhow!(
                        "Failed to canonicalize class {}",
                        entry.path().display()
                    ))?
                    .replace(MAIN_SEPARATOR, ".");
                if let Some(prev) = self.classes.get(&cls) {
                    let mut previnv = PathBuf::from(&self.classes_path);
                    previnv.push(prev);
                    let prev = stringify(&previnv)?;
                    let mut pathinv = PathBuf::from(&self.classes_path);
                    pathinv.push(relpath);
                    let relpath = stringify(&pathinv)?;
                    // Ensure error message is stable without having to sort the directory walk
                    // iterator.
                    let (first, second) = if prev.cmp(relpath).is_lt() {
                        (prev, relpath)
                    } else {
                        (relpath, prev)
                    };
                    return Err(anyhow!(
                        "Definition of class '{cls}' in '{first}' collides with definition in '{second}'. \
                        Classes can only be defined once per inventory."
                    ));
                }
                self.classes.insert(cls, relpath.to_path_buf());
            }
        }
        Ok(())
    }
}

#[pymethods]
impl Reclass {
    #[new]
    #[pyo3(signature = (nodes_path="./inventory/nodes", classes_path="./inventory/classes", ignore_class_notfound=false))]
    pub fn new(
        nodes_path: &str,
        classes_path: &str,
        ignore_class_notfound: bool,
    ) -> PyResult<Self> {
        let mut r = Self {
            nodes_path: nodes_path.to_owned(),
            classes_path: classes_path.to_owned(),
            ignore_class_notfound,
            classes: HashMap::new(),
        };
        r.discover_classes()
            .map_err(|e| PyValueError::new_err(format!("Error while discovering classes: {e}")))?;
        Ok(r)
    }

    fn __repr__(&self) -> String {
        format!("{self:#?}")
    }

    /// Returns the rendered data for the node with the provided name if it exists
    pub fn nodeinfo(&self, nodename: &str) -> PyResult<NodeInfo> {
        let mut n = Node::parse(self, nodename)
            .map_err(|e| PyValueError::new_err(format!("Error while parsing {nodename}: {e}")))?;
        n.render(self)
            .map_err(|e| PyValueError::new_err(format!("Error while rendering {nodename}: {e}")))?;

        Ok(n.into())
    }
}

impl Default for Reclass {
    fn default() -> Self {
        Self::new("./inventory/nodes", "./inventory/classes", false).unwrap()
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
        let n = Reclass::new(
            "./tests/inventory/nodes",
            "./tests/inventory/classes",
            false,
        )
        .unwrap();
        assert_eq!(n.nodes_path, "./tests/inventory/nodes");
        assert_eq!(n.classes_path, "./tests/inventory/classes");
        assert_eq!(n.ignore_class_notfound, false);
    }

    #[test]
    #[should_panic(expected = "Error while discovering classes: \
        Definition of class 'foo.bar' in './tests/broken-inventory/classes/foo.bar.yml' \
        collides with definition in './tests/broken-inventory/classes/foo/bar.yml'. \
        Classes can only be defined once per inventory.")]
    fn test_reclass_discover_classes() {
        pyo3::prepare_freethreaded_python();
        Reclass::new(
            "./tests/broken-inventory/nodes",
            "./tests/broken-inventory/classes",
            false,
        )
        .unwrap();
    }
}
