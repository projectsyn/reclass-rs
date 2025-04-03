#![deny(clippy::suspicious)]
#![warn(clippy::pedantic)]
#![warn(let_underscore_drop)]
// Allows need to be after warn/deny
#![allow(clippy::doc_markdown)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::similar_names)]

mod config;
mod fsutil;
mod inventory;
mod list;
mod node;
mod refs;
pub mod types;

use anyhow::{anyhow, Result};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyType;
use rayon::ThreadPoolBuilder;
use std::collections::HashMap;
use std::path::{Path, PathBuf, MAIN_SEPARATOR};
use walkdir::WalkDir;

use config::{CompatFlag, Config};
use fsutil::to_lexical_absolute;
use inventory::Inventory;
use node::{Node, NodeInfo, NodeInfoMeta};

const SUPPORTED_YAML_EXTS: [&str; 2] = ["yml", "yaml"];

#[derive(Clone, Debug)]
struct EntityInfo {
    path: PathBuf,
    loc: PathBuf,
}

#[derive(Eq, PartialEq)]
enum EntityKind {
    Node,
    Class,
}

impl std::fmt::Display for EntityKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntityKind::Node => write!(f, "node"),
            EntityKind::Class => write!(f, "class"),
        }
    }
}

impl EntityKind {
    fn plural(&self, capitalize: bool) -> &'static str {
        match self {
            Self::Node => {
                if capitalize {
                    "Nodes"
                } else {
                    "nodes"
                }
            }
            Self::Class => {
                if capitalize {
                    "Classes"
                } else {
                    "classes"
                }
            }
        }
    }
}

/// This struct holds configuration fields for various library behaviors
#[pyclass]
#[derive(Clone, Debug)]
pub struct Reclass {
    /// Reclass config
    #[pyo3(get)]
    pub config: Config,
    /// List of discovered Reclass classes in `classes_path`
    classes: HashMap<String, EntityInfo>,
    /// List of discovered Reclass nodes in `nodes_path`
    nodes: HashMap<String, EntityInfo>,
}

fn err_duplicate_entity(
    kind: &EntityKind,
    root: &str,
    relpath: &Path,
    cls: &str,
    prev: &Path,
) -> Result<()> {
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
        "Definition of {kind} '{cls}' in '{first}' collides with definition in '{second}'. \
            {} can only be defined once per inventory.",
        kind.plural(true)
    ))
}

fn walk_entity_dir(
    kind: &EntityKind,
    root: &str,
    entity_map: &mut HashMap<String, EntityInfo>,
    compose_node_name: bool,
) -> Result<()> {
    let entity_root = to_lexical_absolute(&PathBuf::from(root))?;

    // We need to follow symlinks when walking the root directory, so that inventories which
    // contain symlinked directories are loaded correctly.
    for entry in WalkDir::new(root).follow_links(true) {
        let entry = entry?;
        // We use `entry.path()` here to get the symlink name for symlinked files.
        let ext = if let Some(ext) = entry.path().extension() {
            ext.to_str()
        } else {
            None
        };
        if ext.is_some() && SUPPORTED_YAML_EXTS.contains(&ext.unwrap()) {
            // it's an entity (class or node), process it
            let abspath = to_lexical_absolute(entry.path())?;
            let relpath = abspath.strip_prefix(&entity_root)?;
            let cls = relpath.with_extension("");
            let (cls, loc) = if cls.ends_with("init") {
                // treat `foo/init.yml` as contents for class `foo`
                let cls = cls
                    .parent()
                    .ok_or(anyhow!(
                        "Failed to normalize entity {}",
                        entry.path().display()
                    ))?
                    .to_owned();
                // here, unwrap can't panic since we otherwise would have already returned an error
                // in the previous statement.
                let loc = relpath.parent().unwrap();
                // For `init.ya?ml` classes, the location is parent directory of the directory
                // holding the class file.
                (cls, loc.parent().unwrap_or(Path::new("")))
            } else {
                // For normal classes, the location is the directory holding the class file.
                (cls, relpath.parent().unwrap_or(Path::new("")))
            };
            let cls = cls.to_str().ok_or(anyhow!(
                "Failed to normalize entity {}",
                entry.path().display()
            ))?;
            let (cls, loc) =
                if kind == &EntityKind::Node && (cls.starts_with('_') || !compose_node_name) {
                    // special case node paths starting with _ for compose-node-name and return
                    // only base name for all nodes regardless of depth if compose-node-name isn't
                    // enabled.
                    (
                        cls.split(MAIN_SEPARATOR).next_back().ok_or(anyhow!(
                            "Can't shorten node name for {}",
                            entry.path().display()
                        ))?,
                        Path::new(""),
                    )
                } else {
                    (cls, loc)
                };
            let cls = cls.replace(MAIN_SEPARATOR, ".");
            if let Some(prev) = entity_map.get(&cls) {
                return err_duplicate_entity(kind, root, relpath, &cls, &prev.path);
            }
            entity_map.insert(
                cls,
                EntityInfo {
                    path: relpath.to_path_buf(),
                    loc: PathBuf::from(loc),
                },
            );
        }
    }
    Ok(())
}

impl Reclass {
    pub fn new(
        inventory_path: &str,
        nodes_path: &str,
        classes_path: &str,
        ignore_class_notfound: bool,
    ) -> Result<Self> {
        let config = Config::new(
            Some(inventory_path),
            Some(nodes_path),
            Some(classes_path),
            Some(ignore_class_notfound),
        )?;
        Self::new_from_config(config)
    }

    pub fn new_from_config(config: Config) -> Result<Self> {
        let mut r = Self {
            config,
            classes: HashMap::new(),
            nodes: HashMap::new(),
        };
        r.discover_nodes()
            .map_err(|e| anyhow!("Error while discovering nodes: {e}"))?;
        r.discover_classes()
            .map_err(|e| anyhow!("Error while discovering classes: {e}"))?;
        Ok(r)
    }

    /// Discover all top-level YAML files in `r.nodes_path`.
    ///
    /// This method will raise an error if multiple nodes which resolve to the same node name
    /// exist. Currently the only case where this can happen is when an inventory defines a node as
    /// both `<name>.yml` and `<name>.yaml`.
    fn discover_nodes(&mut self) -> Result<()> {
        walk_entity_dir(
            &EntityKind::Node,
            &self.config.nodes_path,
            &mut self.nodes,
            self.config.compose_node_name,
        )
    }

    /// Discover all classes in `r.classes_path` and store the resulting list in `r.known_classes`.
    ///
    /// This method will raise an error if multiple classes which resolve to the same absolute
    /// class name exist (e.g. classes `foo..bar.yml` and `foo/.bar.yml` are both included as
    /// `foo..bar`).
    fn discover_classes(&mut self) -> Result<()> {
        walk_entity_dir(
            &EntityKind::Class,
            &self.config.classes_path,
            &mut self.classes,
            true,
        )
    }

    /// Renders a single Node and returns the corresponding `NodeInfo` struct.
    pub fn render_node(&self, nodename: &str) -> Result<NodeInfo> {
        let mut n = Node::parse(self, nodename)?;
        n.render(self)?;
        Ok(NodeInfo::from(n))
    }

    pub fn render_inventory(&self) -> Result<Inventory> {
        Inventory::render(self)
    }
}

#[pymethods]
impl Reclass {
    #[new]
    #[pyo3(signature = (inventory_path=".", nodes_path=None, classes_path=None, ignore_class_notfound=None))]
    pub fn new_py(
        inventory_path: Option<&str>,
        nodes_path: Option<&str>,
        classes_path: Option<&str>,
        ignore_class_notfound: Option<bool>,
    ) -> PyResult<Self> {
        let c = Config::new(
            inventory_path,
            nodes_path,
            classes_path,
            ignore_class_notfound,
        )
        .map_err(|e| PyValueError::new_err(format!("{e}")))?;
        let r = Self::new_from_config(c).map_err(|e| PyValueError::new_err(format!("{e}")))?;
        Ok(r)
    }

    /// Creates a `Reclass` instance for the provided `inventory_path` and loads config options
    /// from the provided config file. The value of `config_file` is interpreted relative to
    /// `inventory_path`.
    ///
    /// Returns a `Reclass` instance or raises a `ValueError`
    #[classmethod]
    #[pyo3(signature = (inventory_path, config_file, verbose=false))]
    fn from_config_file(
        cls: &Bound<'_, PyType>,
        inventory_path: &str,
        config_file: &str,
        verbose: bool,
    ) -> PyResult<Self> {
        let mut c = Config::new(Some(inventory_path), None, None, None)
            .map_err(|e| PyValueError::new_err(format!("{e}")))?;
        c.load_from_file(config_file, verbose)
            .map_err(|e| PyValueError::new_err(format!("{e}")))?;
        Self::from_config(cls, c)
    }

    /// Creates a `Reclass` instance from the provided `Config` instance.
    ///
    /// Returns a `Reclass` instance or raises a `ValueError`
    #[classmethod]
    fn from_config(_cls: &Bound<'_, PyType>, config: Config) -> PyResult<Self> {
        let r = Self::new_from_config(config).map_err(|e| PyValueError::new_err(format!("{e}")))?;
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
        self.render_inventory()
            .map_err(|e| PyValueError::new_err(format!("Error while rendering inventory: {e}")))
    }

    /// Configures the number of threads to use when rendering the full inventory. Calling the
    /// method with `count=0` will configure the thread pool to have one thread per logical core of
    /// the system.
    ///
    /// Note that this method should only be called once and will print a diagnostic message if
    /// called again.
    #[classmethod]
    pub fn set_thread_count(_cls: &Bound<'_, PyType>, count: usize) {
        if let Err(e) = ThreadPoolBuilder::new().num_threads(count).build_global() {
            eprintln!("While initializing global thread pool: {e}");
        }
    }

    /// Sets the provided CompatFlag in the current Reclass instance's config object
    pub fn set_compat_flag(&mut self, flag: CompatFlag) {
        self.config.compatflags.insert(flag);
    }

    /// Unsets the provided CompatFlag in the current Reclass instance's config object
    pub fn unset_compat_flag(&mut self, flag: &CompatFlag) {
        self.config.compatflags.remove(flag);
    }

    /// Clears the compatflags set in the current Reclass instance's config object
    pub fn clear_compat_flags(&mut self) {
        self.config.compatflags.clear();
    }

    /// Returns a dict containing all discovered nodes with their paths relative to `nodes_path`.
    ///
    /// NOTE: We don't use the generated getter here, because we don't want to return the
    /// EntityInfo.
    #[getter]
    pub fn nodes(&self) -> PyResult<HashMap<String, PathBuf>> {
        let res = self
            .nodes
            .iter()
            .map(|(k, v)| (k.clone(), v.path.clone()))
            .collect::<HashMap<String, PathBuf>>();
        Ok(res)
    }

    /// Returns the dict of all discovered classes and their paths relative to `classes_path`.
    ///
    /// NOTE: We don't use the generated getter here, because we don't want to return the
    /// EntityInfo.
    #[getter]
    pub fn classes(&self) -> PyResult<HashMap<String, PathBuf>> {
        let res = self
            .classes
            .iter()
            .map(|(k, v)| (k.clone(), v.path.clone()))
            .collect::<HashMap<String, PathBuf>>();
        Ok(res)
    }

    /// Update the current Reclass instance's config object with the provided
    /// `ignore_class_notfound_regexp` patterns
    pub fn set_ignore_class_notfound_regexp(&mut self, patterns: Vec<String>) -> PyResult<()> {
        self.config
            .set_ignore_class_notfound_regexp(patterns)
            .map_err(|e| {
                PyValueError::new_err(format!(
                    "Error while setting ignore_class_notfound_regexp: {e}"
                ))
            })
    }
}

impl Default for Reclass {
    fn default() -> Self {
        Self::new(".", "nodes", "classes", false).unwrap()
    }
}

#[pymodule]
fn reclass_rs(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Register the top-level `Reclass` Python class which is used to configure the library
    m.add_class::<Reclass>()?;
    // Register the `Config` class and `CompatFlag` enum
    m.add_class::<Config>()?;
    m.add_class::<CompatFlag>()?;
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
        let n = Reclass::new("./tests/inventory", "nodes", "classes", false).unwrap();
        assert_eq!(n.config.inventory_path, "./tests/inventory");
        assert_eq!(n.config.nodes_path, "./tests/inventory/nodes");
        assert_eq!(n.config.classes_path, "./tests/inventory/classes");
        assert_eq!(n.config.ignore_class_notfound, false);
    }

    #[test]
    #[should_panic(expected = "Error while discovering classes: \
        Definition of class 'foo.bar' in './tests/broken-inventory/classes/foo.bar.yml' \
        collides with definition in './tests/broken-inventory/classes/foo/bar.yml'. \
        Classes can only be defined once per inventory.")]
    fn test_reclass_discover_classes() {
        Reclass::new("./tests/broken-inventory", "nodes", "classes", false).unwrap();
    }

    #[test]
    fn test_reclass_discover_nodes_compose_node_name() {
        let mut c = Config::new(
            Some("./tests/inventory-compose-node-name"),
            None,
            None,
            None,
        )
        .unwrap();
        c.load_from_file("reclass-config.yml", true).unwrap();
        let r = Reclass::new_from_config(c).unwrap();
        assert_eq!(r.nodes.len(), 8);
        let mut nodes = r.nodes.keys().collect::<Vec<_>>();
        nodes.sort();
        assert_eq!(
            nodes,
            vec!["a", "a.1", "b.1", "c.1", "c._c.1", "d", "d1", "d2"]
        );
        assert_eq!(r.nodes["a"].path, PathBuf::from("a.yml"));
        assert_eq!(r.nodes["a.1"].path, PathBuf::from("a.1.yml"));
        assert_eq!(r.nodes["b.1"].path, PathBuf::from("b/1.yml"));
        assert_eq!(r.nodes["c.1"].path, PathBuf::from("c/1.yml"));
        assert_eq!(r.nodes["c._c.1"].path, PathBuf::from("c/_c/1.yml"));
        assert_eq!(r.nodes["d"].path, PathBuf::from("d.yml"));
        assert_eq!(r.nodes["d1"].path, PathBuf::from("_d/d1.yml"));
        assert_eq!(r.nodes["d2"].path, PathBuf::from("_d/d/d2.yml"));
    }

    #[test]
    fn test_reclass_discover_nodes_nested() {
        let mut c = Config::new(Some("./tests/inventory-nested-nodes"), None, None, None).unwrap();
        c.compose_node_name = false;
        let r = Reclass::new_from_config(c).unwrap();
        assert_eq!(r.nodes.len(), 4);
        let mut nodes = r.nodes.keys().collect::<Vec<_>>();
        nodes.sort();
        assert_eq!(nodes, vec!["a1", "b1", "c1", "d1"]);

        assert_eq!(r.nodes["a1"].path, PathBuf::from("a/a1.yml"));
        assert_eq!(r.nodes["b1"].path, PathBuf::from("b/b1.yml"));
        assert_eq!(r.nodes["c1"].path, PathBuf::from("c/c1.yml"));
        assert_eq!(r.nodes["d1"].path, PathBuf::from("_d/d1.yml"));
    }

    #[test]
    fn test_reclass_discover_nodes_nested_composed() {
        let mut c = Config::new(Some("./tests/inventory-nested-nodes"), None, None, None).unwrap();
        c.compose_node_name = true;
        let r = Reclass::new_from_config(c).unwrap();
        assert_eq!(r.nodes.len(), 4);
        let mut nodes = r.nodes.keys().collect::<Vec<_>>();
        nodes.sort();
        assert_eq!(nodes, vec!["a.a1", "b.b1", "c.c1", "d1"]);

        assert_eq!(r.nodes["a.a1"].path, PathBuf::from("a/a1.yml"));
        assert_eq!(r.nodes["b.b1"].path, PathBuf::from("b/b1.yml"));
        assert_eq!(r.nodes["c.c1"].path, PathBuf::from("c/c1.yml"));
        assert_eq!(r.nodes["d1"].path, PathBuf::from("_d/d1.yml"));
    }
}
