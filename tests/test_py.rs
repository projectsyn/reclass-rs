use pyo3::prelude::*;
use pyo3::types::PyDict;

use reclass_rs::Reclass;

#[test]
fn test_reclass() {
    pyo3::prepare_freethreaded_python();
    Python::with_gil(|py| {
        let r = Reclass::new(
            "./tests/inventory/nodes",
            "./tests/inventory/classes",
            false,
        )
        .unwrap()
        .into_py(py);
        let locals = PyDict::new(py);
        locals.set_item("r", r).unwrap();
        py.run(
            r#"assert r and "Reclass" in str(type(r))"#,
            None,
            Some(locals),
        )
        .unwrap();
    });
}
