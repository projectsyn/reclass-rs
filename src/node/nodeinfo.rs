use anyhow::{anyhow, Result};
use chrono::offset::Local;
use chrono::DateTime;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::path::PathBuf;

use crate::config::{CompatFlag, Config};
use crate::types::{Mapping, Value};

/// Contains metadata for a Reclass node's rendered data
#[pyclass]
#[derive(Clone, Debug)]
pub struct NodeInfoMeta {
    /// Inventory path of node
    parts: PathBuf,
    /// Unmodified inventory path of node (without file extension)
    npath: PathBuf,
    /// Original node name
    #[pyo3(get)]
    pub node: String,
    /// Short node name.
    #[pyo3(get)]
    pub name: String,
    /// Path of node in inventory
    #[pyo3(get)]
    pub uri: String,
    /// Environment of the node
    #[pyo3(get)]
    pub environment: String,
    #[pyo3(get)]
    /// `chrono::DateTime<Local>` at which we started rendering the data for the node
    render_time: DateTime<Local>,
}

impl Default for NodeInfoMeta {
    fn default() -> Self {
        Self::new("", "", "", PathBuf::new(), PathBuf::new(), "")
    }
}

impl NodeInfoMeta {
    pub fn new(
        node: &str,
        name: &str,
        uri: &str,
        parts: PathBuf,
        npath: PathBuf,
        environment: &str,
    ) -> Self {
        Self {
            parts,
            npath,
            node: node.into(),
            name: name.into(),
            uri: uri.into(),
            environment: environment.into(),
            render_time: Local::now(),
        }
    }

    /// Generates a Mapping suitable to use as meta parameter `_reclass_`
    pub(crate) fn as_reclass(&self, config: &Config) -> Result<Mapping> {
        let part0 = self
            .parts
            .iter()
            .next()
            .ok_or(anyhow!("Can't extract first path segment for node"))?
            .to_str()
            .ok_or(anyhow!("Unable to convert path segment to a string"))?;
        let parts = if config.compose_node_name
            && config
                .compatflags
                .contains(&CompatFlag::ComposeNodeNameLiteralDots)
        {
            // when CompatFlag ComposeNodeNameLiteralDots is set, we naively split the node's name
            // by dots to generate the parts list for the node metadata.
            // This matches Python reclass's behavior, but is incorrect for nodes which contain
            // literal dots in the file name.
            self.name.split('.').collect::<Vec<&str>>()
        } else if part0.starts_with('_') {
            // Always drop path prefix for paths that start with `_`
            vec![self
                .parts
                .iter()
                .next_back()
                .ok_or(anyhow!("Unable to extract last segment from node"))?
                .to_str()
                .ok_or(anyhow!("Unable to convert path segment to a string"))?]
        } else {
            // If the compat flag isn't set, we generate the parts list from the provided shortened
            // pathbuf containing the path within `nodes_path` which preserves literal dots in the
            // node's filename.
            self.parts
                .iter()
                .map(|s| {
                    s.to_str()
                        .ok_or(anyhow!("Unable to convert path segment {s:?} to a string"))
                })
                .collect::<Result<Vec<&str>, _>>()?
        };
        let namedata: Vec<(Value, Value)> = vec![
            ("full".into(), self.name.clone().into()),
            (
                "parts".into(),
                Value::Sequence(parts.iter().map(|&s| s.into()).collect::<Vec<Value>>()),
            ),
            ("path".into(), parts.join("/").into()),
            (
                "short".into(),
                (*parts.iter().last().ok_or(anyhow!("Empty node name?"))?).into(),
            ),
        ];
        let namedata = Mapping::from_iter(namedata);

        let mut pmeta = Mapping::new();
        pmeta.insert("environment".into(), self.environment.clone().into())?;
        pmeta.insert("name".into(), Value::Mapping(namedata))?;

        Ok(pmeta)
    }

    /// Return the class's name or path depending on whether config option
    /// `class_mappings_match_path` is set or not
    pub(crate) fn class_mappings_match_name(&self, cfg: &Config) -> Result<&str> {
        let matchname = if cfg.class_mappings_match_path {
            self.npath
                .to_str()
                .ok_or(anyhow!("Failed to convert node path to string"))?
        } else {
            &self.name
        };
        Ok(matchname)
    }
}

#[pymethods]
impl NodeInfoMeta {
    fn __repr__(&self) -> String {
        format!("{self:#?}")
    }
}

/// Rendered data for a Reclass node
#[pyclass]
#[derive(Clone, Debug)]
pub struct NodeInfo {
    /// Reclass metadata for the node.
    #[pyo3(get, name = "__reclass__")]
    pub reclass: NodeInfoMeta,
    /// Applications included by the node.
    #[pyo3(get)]
    pub applications: Vec<String>,
    /// Classes included by the node.
    #[pyo3(get)]
    pub classes: Vec<String>,
    /// Exports defined for the node.
    /// Note that the exports functionality is not yet implemented.
    pub exports: Mapping,
    /// Parameters defined for the node.
    pub parameters: Mapping,
}

impl From<super::Node> for NodeInfo {
    /// Creates a `NodeInfo` struct from a `Node`
    fn from(n: super::Node) -> Self {
        NodeInfo {
            reclass: n.meta,
            applications: n.applications.into(),
            classes: n.classes.into(),
            parameters: n.parameters,
            // NOTE(sg): Python reclass's exports functionality is not implemented yet.
            exports: Mapping::new(),
        }
    }
}

#[pymethods]
impl NodeInfo {
    fn __repr__(&self) -> String {
        format!("{self:#?}")
    }

    /// Returns the NodeInfo `parameters` field as a PyDict
    #[getter]
    fn parameters<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        self.parameters.as_py_dict(py)
    }

    /// Returns the NodeInfo `exports` field as a PyDict
    #[getter]
    fn exports<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        #[cfg(debug_assertions)]
        eprintln!("reclass_rs doesn't support exports yet!");
        self.exports.as_py_dict(py)
    }

    /// Returns the NodeInfo data as a PyDict
    ///
    /// This method generates a PyDict which should be structured identically to Python Reclass's
    /// `nodeinfo` return value.
    pub(crate) fn as_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        dict.set_item("__reclass__", self.reclass_as_dict(py)?)?;
        dict.set_item("applications", self.applications.clone().into_pyobject(py)?)?;
        dict.set_item("classes", self.classes.clone().into_pyobject(py)?)?;
        dict.set_item(
            "environment",
            self.reclass.environment.clone().into_pyobject(py)?,
        )?;
        dict.set_item("exports", self.exports(py)?)?;
        dict.set_item("parameters", self.parameters(py)?)?;
        Ok(dict)
    }

    /// Returns the NodeInfo `meta` field as a PyDict
    fn reclass_as_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        dict.set_item("node", self.reclass.node.clone().into_pyobject(py)?)?;
        dict.set_item("name", self.reclass.name.clone().into_pyobject(py)?)?;
        dict.set_item("uri", self.reclass.uri.clone().into_pyobject(py)?)?;
        dict.set_item(
            "environment",
            self.reclass.environment.clone().into_pyobject(py)?,
        )?;
        // Format time as strftime %c for Python compatibility
        dict.set_item(
            "timestamp",
            self.reclass.render_time.format("%c").to_string(),
        )?;
        Ok(dict)
    }
}
