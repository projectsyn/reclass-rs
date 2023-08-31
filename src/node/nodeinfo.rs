use chrono::offset::Local;
use chrono::DateTime;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use serde_yaml::{Mapping, Value};

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

    /// Generates a serde_yaml::Mapping suitable to use as meta parameter `_reclass_`
    pub(crate) fn as_reclass(&self) -> Mapping {
        let mut namedata = Mapping::new();
        namedata.insert("full".into(), self.name.clone().into());
        namedata.insert(
            "parts".into(),
            Value::Sequence(vec![self.name.clone().into()]),
        );
        namedata.insert("path".into(), self.name.clone().into());
        namedata.insert("short".into(), self.name.clone().into());

        let mut pmeta = Mapping::new();
        pmeta.insert("environment".into(), self.environment.clone().into());
        pmeta.insert("name".into(), Value::Mapping(namedata));

        pmeta
    }
}

#[pymethods]
impl NodeInfoMeta {
    fn __repr__(&self) -> String {
        format!("{:#?}", self)
    }
}

/// Rendered data for a Reclass node
#[pyclass]
#[derive(Clone, Debug)]
pub struct NodeInfo {
    pub reclass: NodeInfoMeta,
    #[pyo3(get)]
    pub applications: Vec<String>,
    #[pyo3(get)]
    pub classes: Vec<String>,
    pub parameters: serde_yaml::Mapping,
}

impl From<super::Node> for NodeInfo {
    /// Creates a `NodeInfo` struct from a `Node`
    fn from(n: super::Node) -> Self {
        NodeInfo {
            reclass: n.meta,
            applications: n.applications.into(),
            classes: n.classes.into(),
            parameters: n._params,
        }
    }
}

/// Converts a serde_yaml::Value into a PyObject
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

/// Converts a serde_yaml::Mapping into a PyDict
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
    fn __repr__(&self) -> String {
        format!("{:#?}", self)
    }

    /// Returns the NodeInfo data as a PyDict
    ///
    /// This method generates a PyDict which should be structured identically to Python Reclass's
    /// `nodeinfo` return value.
    fn as_dict(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new(py);
        dict.set_item("__reclass__", self.__reclass__(py)?)?;
        dict.set_item("applications", self.applications.clone().into_py(py))?;
        dict.set_item("classes", self.classes.clone().into_py(py))?;
        dict.set_item("environment", self.reclass.environment.clone().into_py(py))?;
        dict.set_item("parameters", self.parameters(py)?)?;
        Ok(dict.into())
    }

    /// Returns the NodeInfo `parameters` field as a PyDict
    #[getter]
    fn parameters(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        as_py_dict(&self.parameters, py)
    }

    /// Returns the NodeInfo `meta` field as a PyDict
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

#[cfg(test)]
mod nodeinfo_tests {
    use super::*;

    #[test]
    fn test_as_py_dict() {
        let m = r#"
        a: a
        b: ['b', 'b']
        c: 3
        d:
          d: d
        e: true
        "#;
        let m: serde_yaml::Mapping = serde_yaml::from_str(m).unwrap();
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let d = as_py_dict(&m, py).unwrap();
            assert!(d.as_ref(py).is_instance_of::<PyDict>());
            let locals = PyDict::new(py);
            locals.set_item("d", d).unwrap();
            py.run(
                r#"assert d == {"a": "a", "b": ["b", "b"], "c": 3,"d": {"d": "d"}, "e": True} "#,
                None,
                Some(locals),
            )
            .unwrap();
        });
    }

    #[test]
    fn test_as_py_obj_null() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let pyv = as_py_obj(&Value::Null, py).unwrap();
            let v = pyv.as_ref(py);
            assert!(v.is_none());
        });
    }

    #[test]
    fn test_as_py_obj_bool() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let pyb = as_py_obj(&Value::Bool(true), py).unwrap();
            let b = pyb.as_ref(py);
            assert!(b.is_instance_of::<pyo3::types::PyBool>());
            assert!(b.downcast_exact::<pyo3::types::PyBool>().unwrap().is_true());
        });
    }

    #[test]
    fn test_as_py_obj_int() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let nums: Vec<Value> = vec![5.into(), (-2i64).into()];
            for n in nums {
                let pyn = as_py_obj(&n, py).unwrap();
                let n = pyn.as_ref(py);
                assert!(n.is_instance_of::<pyo3::types::PyInt>());
                assert!(n
                    .downcast_exact::<pyo3::types::PyInt>()
                    .unwrap()
                    .eq(n.into_py(py))
                    .unwrap());
            }
        });
    }

    #[test]
    fn test_as_py_obj_float() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let pyn = as_py_obj(&3.14.into(), py).unwrap();
            let n = pyn.as_ref(py);
            assert!(n.is_instance_of::<pyo3::types::PyFloat>());
            assert!(n
                .downcast_exact::<pyo3::types::PyFloat>()
                .unwrap()
                .eq(3.14.into_py(py))
                .unwrap());
        });
    }

    #[test]
    fn test_as_py_obj_sequence() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let pys = as_py_obj(&vec![1, 2, 3].into(), py).unwrap();
            let s = pys.as_ref(py);
            assert!(s.is_instance_of::<pyo3::types::PyList>());
            assert!(s
                .downcast_exact::<pyo3::types::PyList>()
                .unwrap()
                .eq(pyo3::types::PyList::new(py, vec![1, 2, 3]))
                .unwrap());
        });
    }

    #[test]
    fn test_as_py_obj_string() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let pys = as_py_obj(&"hello, world".into(), py).unwrap();
            let s = pys.as_ref(py);
            assert!(s.is_instance_of::<pyo3::types::PyString>());
            assert_eq!(
                s.downcast_exact::<pyo3::types::PyString>()
                    .unwrap()
                    .to_str()
                    .unwrap(),
                "hello, world"
            );
        });
    }
}
