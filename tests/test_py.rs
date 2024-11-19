use std::ffi::CString;

use pyo3::prelude::*;
use pyo3::types::PyDict;

use reclass_rs::Reclass;

#[test]
fn test_reclass() {
    pyo3::prepare_freethreaded_python();
    Python::with_gil(|py| {
        let r = Reclass::new("./tests/inventory", "nodes", "classes", false)
            .unwrap()
            .into_pyobject(py)
            .unwrap();
        let locals = PyDict::new(py);
        locals.set_item("r", r).unwrap();
        py.run(
            &CString::new(r#"assert r and "Reclass" in str(type(r))"#).unwrap(),
            None,
            Some(&locals),
        )
        .unwrap();
    });
}
