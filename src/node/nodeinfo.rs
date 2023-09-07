use chrono::offset::Local;
use chrono::DateTime;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::types::{Mapping, Value};

/// Contains metadata for a Reclass node's rendered data
#[pyclass]
#[derive(Clone, Debug)]
pub struct NodeInfoMeta {
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
        Self::new("", "", "", "")
    }
}

impl NodeInfoMeta {
    pub fn new(node: &str, name: &str, uri: &str, environment: &str) -> Self {
        Self {
            node: node.into(),
            name: name.into(),
            uri: uri.into(),
            environment: environment.into(),
            render_time: Local::now(),
        }
    }

    /// Generates a Mapping suitable to use as meta parameter `_reclass_`
    pub(crate) fn as_reclass(&self) -> Mapping {
        let namedata: Vec<(Value, Value)> = vec![
            ("full".into(), self.name.clone().into()),
            (
                "parts".into(),
                Value::Sequence(vec![self.name.clone().into()]),
            ),
            ("path".into(), self.name.clone().into()),
            ("short".into(), self.name.clone().into()),
        ];
        let namedata = Mapping::from_iter(namedata);

        let mut pmeta = Mapping::new();
        pmeta
            .insert("environment".into(), self.environment.clone().into())
            .unwrap();
        pmeta
            .insert("name".into(), Value::Mapping(namedata))
            .unwrap();

        pmeta
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
    fn parameters(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        self.parameters.as_py_dict(py)
    }

    /// Returns the NodeInfo `exports` field as a PyDict
    #[getter]
    fn exports(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        #[cfg(debug_assertions)]
        eprintln!("reclass_rs doesn't support exports yet!");
        self.exports.as_py_dict(py)
    }

    /// Returns the NodeInfo data as a PyDict
    ///
    /// This method generates a PyDict which should be structured identically to Python Reclass's
    /// `nodeinfo` return value.
    pub(crate) fn as_dict(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new(py);
        dict.set_item("__reclass__", self.reclass_as_dict(py)?)?;
        dict.set_item("applications", self.applications.clone().into_py(py))?;
        dict.set_item("classes", self.classes.clone().into_py(py))?;
        dict.set_item("environment", self.reclass.environment.clone().into_py(py))?;
        dict.set_item("exports", self.exports(py)?)?;
        dict.set_item("parameters", self.parameters(py)?)?;
        Ok(dict.into())
    }

    /// Returns the NodeInfo `meta` field as a PyDict
    fn reclass_as_dict(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new(py);
        dict.set_item("node", self.reclass.node.clone().into_py(py))?;
        dict.set_item("name", self.reclass.name.clone().into_py(py))?;
        dict.set_item("uri", self.reclass.uri.clone().into_py(py))?;
        dict.set_item("environment", self.reclass.environment.clone().into_py(py))?;
        // Format time as strftime %c for Python compatibility
        dict.set_item(
            "timestamp",
            self.reclass.render_time.format("%c").to_string(),
        )?;
        Ok(dict.into())
    }
}
