#![deny(clippy::suspicious)]
#![warn(clippy::pedantic)]
#![warn(let_underscore_drop)]
// Allows need to be after warn/deny
#![allow(clippy::doc_markdown)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::similar_names)]

mod inventory;
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

use inventory::Inventory;
use node::{Node, NodeInfo, NodeInfoMeta};

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
    /// List of discovered Reclass classes in `classes_path`
    classes: HashMap<String, PathBuf>,
    /// List of discovered Reclass nodes in `nodes_path`
    nodes: HashMap<String, PathBuf>,
}

fn err_duplicate_entity(root: &str, relpath: &Path, cls: &str, prev: &Path) -> Result<()> {
    fn stringify(p: &Path) -> Result<&str> {
        p.to_str()
            .ok_or(anyhow!("Failed to convert {} to string", p.display()))
    }
    // Reconstruct absolute entity paths for the error message
    let mut previnv = PathBuf::from(root);
    previnv.push(prev);
    let prev = stringify(&previnv)?;
    let mut pathinv = PathBuf::from(root);
    pathinv.push(relpath);
    let relpath = stringify(&pathinv)?;
    // Ensure error message is stable without having to sort the directory walk
    // iterator.
    let (first, second) = if prev.cmp(relpath).is_lt() {
        (prev, relpath)
    } else {
        (relpath, prev)
    };
    Err(anyhow!(
        "Definition of class '{cls}' in '{first}' collides with definition in '{second}'. \
            Classes can only be defined once per inventory."
    ))
}

fn walk_entity_dir(
    root: &str,
    entity_map: &mut HashMap<String, PathBuf>,
    max_depth: usize,
) -> Result<()> {
    let entity_root = PathBuf::from(root).canonicalize()?;

    for entry in WalkDir::new(root).max_depth(max_depth) {
        let entry = entry?;
        let ext = if let Some(ext) = entry.path().extension() {
            ext.to_str()
        } else {
            None
        };
        if ext.is_some() && SUPPORTED_YAML_EXTS.contains(&ext.unwrap()) {
            // it's an entity (class or node), process it
            let abspath = entry.path().canonicalize()?;
            let relpath = abspath.strip_prefix(&entity_root)?;
            let cls = relpath
                .with_extension("")
                .to_str()
                .ok_or(anyhow!(
                    "Failed to canonicalize entity {}",
                    entry.path().display()
                ))?
                .replace(MAIN_SEPARATOR, ".");
            if let Some(prev) = entity_map.get(&cls) {
                return err_duplicate_entity(root, relpath, &cls, prev);
            }
            entity_map.insert(cls, relpath.to_path_buf());
        }
    }
    Ok(())
}

impl Reclass {
    /// Discover all top-level YAML files in `r.nodes_path`.
    ///
    /// This method will raise an error if multiple nodes which resolve to the same node name
    /// exist. Currently the only case where this can happen is when an inventory defines a node as
    /// both `<name>.yml` and `<name>.yaml`.
    fn discover_nodes(&mut self) -> Result<()> {
        walk_entity_dir(&self.nodes_path, &mut self.nodes, 1)
    }

    /// Discover all classes in `r.classes_path` and store the resulting list in `r.known_classes`.
    ///
    /// This method will raise an error if multiple classes which resolve to the same absolute
    /// class name exist (e.g. classes `foo..bar.yml` and `foo/.bar.yml` are both included as
    /// `foo..bar`).
    fn discover_classes(&mut self) -> Result<()> {
        walk_entity_dir(&self.classes_path, &mut self.classes, usize::MAX)
    }

    /// Renders a single Node and returns the corresponding `NodeInfo` struct.
    fn render_node(&self, nodename: &str) -> Result<NodeInfo> {
        let mut n = Node::parse(self, nodename)?;
        n.render(self)?;
        Ok(NodeInfo::from(n))
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
            nodes: HashMap::new(),
        };
        r.discover_nodes()
            .map_err(|e| PyValueError::new_err(format!("Error while discovering nodes: {e}")))?;
        r.discover_classes()
            .map_err(|e| PyValueError::new_err(format!("Error while discovering classes: {e}")))?;
        Ok(r)
    }

    fn __repr__(&self) -> String {
        format!("{self:#?}")
    }

    /// Returns the rendered data for the node with the provided name if it exists.
    pub fn nodeinfo(&self, nodename: &str) -> PyResult<NodeInfo> {
        self.render_node(nodename)
            .map_err(|e| PyValueError::new_err(format!("Error while rendering {nodename}: {e}")))
    }

    /// Returns the rendered data for the full inventory.
    pub fn inventory(&self) -> PyResult<Inventory> {
        Inventory::render(self)
            .map_err(|e| PyValueError::new_err(format!("Error while rendering inventory: {e}")))
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
    // Register the Inventory class
    m.add_class::<Inventory>()?;
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
