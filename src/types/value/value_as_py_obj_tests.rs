use super::*;
#[test]
fn test_as_py_obj_null() {
    pyo3::prepare_freethreaded_python();
    Python::with_gil(|py| {
        let v = Value::Null.as_py_obj(py).unwrap();
        assert!(v.is_none());
    });
}

#[test]
fn test_as_py_obj_bool() {
    pyo3::prepare_freethreaded_python();
    Python::with_gil(|py| {
        let b = Value::Bool(true).as_py_obj(py).unwrap();
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
            let n = n.as_i64().unwrap();
            assert!(pyn.is_instance_of::<pyo3::types::PyInt>());
            assert!(pyn.downcast_exact::<pyo3::types::PyInt>().unwrap().eq(&n));
        }
    });
}

#[test]
fn test_as_py_obj_float() {
    pyo3::prepare_freethreaded_python();
    Python::with_gil(|py| {
        let n: Value = 3.14.into();
        let n = n.as_py_obj(py).unwrap();
        assert!(n.is_instance_of::<pyo3::types::PyFloat>());
        assert!(n
            .downcast_exact::<pyo3::types::PyFloat>()
            .unwrap()
            .eq(&3.14));
    });
}

#[test]
fn test_as_py_obj_sequence() {
    pyo3::prepare_freethreaded_python();
    Python::with_gil(|py| {
        let s: Value = vec![1, 2, 3].into();
        let s = s.as_py_obj(py).unwrap();
        assert!(s.is_instance_of::<pyo3::types::PyList>());
        assert!(s
            .downcast_exact::<pyo3::types::PyList>()
            .unwrap()
            .eq(&vec![1, 2, 3])
            .unwrap());
    });
}

#[test]
fn test_as_py_obj_string() {
    pyo3::prepare_freethreaded_python();
    Python::with_gil(|py| {
        let s = std::convert::Into::<Value>::into("hello, world")
            .as_py_obj(py)
            .unwrap();
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
