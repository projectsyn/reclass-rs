use chrono::offset::Local;
use chrono::DateTime;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use serde_yaml::{Mapping, Value};

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
}

#[pyclass]
pub struct NodeInfo {
    pub reclass: NodeInfoMeta,
    #[pyo3(get)]
    pub applications: Vec<String>,
    #[pyo3(get)]
    pub classes: Vec<String>,
    pub parameters: serde_yaml::Mapping,
}

impl From<super::Node> for NodeInfo {
    fn from(n: super::Node) -> Self {
        //name:
        //  full: n1
        //  parts:
        //    - n1
        //  path: n1
        //  short: n1
        let mut namedata = Mapping::new();
        namedata.insert("full".into(), n.meta.name.clone().into());
        namedata.insert(
            "parts".into(),
            Value::Sequence(vec![n.meta.name.clone().into()]),
        );
        namedata.insert("path".into(), n.meta.name.clone().into());
        namedata.insert("short".into(), n.meta.name.clone().into());

        let mut pmeta = Mapping::new();
        pmeta.insert("environment".into(), n.meta.environment.clone().into());
        pmeta.insert("name".into(), Value::Mapping(namedata));

        let mut params = n._params.clone();
        params.insert("_reclass_".into(), Value::Mapping(pmeta));

        NodeInfo {
            reclass: n.meta,
            applications: n.applications.into(),
            classes: n.classes.into(),
            parameters: params,
        }
    }
}

fn as_py_obj(v: &Value, py: Python<'_>) -> PyResult<PyObject> {
    let obj = match v {
        Value::Null => Option::<()>::None.into_py(py),
        Value::Bool(b) => b.into_py(py),
        Value::Number(n) => {
            if n.is_i64() {
                n.as_i64().unwrap().into_py(py)
            } else if n.is_u64() {
                n.as_u64().unwrap().into_py(py)
            } else if n.is_f64() {
                n.as_f64().unwrap().into_py(py)
            } else {
                unreachable!("as_py_obj: Number isn't a i64, u64, or f64?");
            }
        }
        Value::Sequence(s) => {
            let mut pyseq = vec![];
            for v in s.iter() {
                pyseq.push(as_py_obj(v, py)?);
            }
            pyseq.into_py(py)
        }
        Value::Mapping(m) => as_py_dict(m, py)?.into(),
        Value::String(s) => s.into_py(py),
        _ => todo!("NYI: {v:#?}"),
    };
    Ok(obj)
}

fn as_py_dict(m: &Mapping, py: Python<'_>) -> PyResult<Py<PyDict>> {
    let dict = PyDict::new(py);

    for (k, v) in m.iter() {
        let pyk = as_py_obj(k, py)?;
        let pyv = as_py_obj(v, py)?;
        dict.set_item(pyk, pyv)?;
    }

    Ok(dict.into())
}

#[pymethods]
impl NodeInfo {
    fn as_dict(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new(py);
        dict.set_item("__reclass__", self.__reclass__(py)?)?;
        dict.set_item("applications", self.applications.clone().into_py(py))?;
        dict.set_item("classes", self.classes.clone().into_py(py))?;
        dict.set_item("environment", self.reclass.environment.clone().into_py(py))?;
        dict.set_item("parameters", self.parameters(py)?)?;
        Ok(dict.into())
    }

    #[getter]
    fn parameters(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        as_py_dict(&self.parameters, py)
    }

    #[getter]
    fn __reclass__(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
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
