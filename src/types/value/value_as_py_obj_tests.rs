use super::*;
#[test]
fn test_as_py_obj_null() {
    pyo3::prepare_freethreaded_python();
    Python::with_gil(|py| {
        let pyv = Value::Null.as_py_obj(py).unwrap();
        let v = pyv.as_ref(py);
        assert!(v.is_none());
    });
}

#[test]
fn test_as_py_obj_bool() {
    pyo3::prepare_freethreaded_python();
    Python::with_gil(|py| {
        let pyb = Value::Bool(true).as_py_obj(py).unwrap();
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
            let pyn = n.as_py_obj(py).unwrap();
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
        let n: Value = 3.14.into();
        let pyn = n.as_py_obj(py).unwrap();
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
        let s: Value = vec![1, 2, 3].into();
        let pys = s.as_py_obj(py).unwrap();
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
        let pys = std::convert::Into::<Value>::into("hello, world")
            .as_py_obj(py)
            .unwrap();
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
