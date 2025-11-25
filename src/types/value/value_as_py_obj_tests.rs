use super::*;
#[test]
fn test_as_py_obj_null() {
    Python::initialize();
    Python::attach(|py| {
        let v = Value::Null.as_py_obj(py).unwrap();
        assert!(v.is_none());
    });
}

#[test]
fn test_as_py_obj_bool() {
    Python::initialize();
    Python::attach(|py| {
        let b = Value::Bool(true).as_py_obj(py).unwrap();
        assert!(b.is_instance_of::<pyo3::types::PyBool>());
        assert!(b.cast_exact::<pyo3::types::PyBool>().unwrap().is_true());
    });
}

#[test]
fn test_as_py_obj_int() {
    Python::initialize();
    Python::attach(|py| {
        let nums: Vec<Value> = vec![5.into(), (-2i64).into()];
        for n in nums {
            let pyn = n.as_py_obj(py).unwrap();
            let n = n.as_i64().unwrap();
            assert!(pyn.is_instance_of::<pyo3::types::PyInt>());
            assert!(pyn.cast_exact::<pyo3::types::PyInt>().unwrap().eq(&n));
        }
    });
}

#[test]
fn test_as_py_obj_float() {
    Python::initialize();
    Python::attach(|py| {
        let n: Value = 3.14.into();
        let n = n.as_py_obj(py).unwrap();
        assert!(n.is_instance_of::<pyo3::types::PyFloat>());
        assert!(n.cast_exact::<pyo3::types::PyFloat>().unwrap().eq(&3.14));
    });
}

#[test]
fn test_as_py_obj_sequence() {
    Python::initialize();
    Python::attach(|py| {
        let s: Value = vec![1, 2, 3].into();
        let s = s.as_py_obj(py).unwrap();
        assert!(s.is_instance_of::<pyo3::types::PyList>());
        assert!(
            s.cast_exact::<pyo3::types::PyList>()
                .unwrap()
                .eq(&vec![1, 2, 3])
                .unwrap()
        );
    });
}

#[test]
fn test_as_py_obj_string() {
    Python::initialize();
    Python::attach(|py| {
        let s = std::convert::Into::<Value>::into("hello, world")
            .as_py_obj(py)
            .unwrap();
        assert!(s.is_instance_of::<pyo3::types::PyString>());
        assert_eq!(
            s.cast_exact::<pyo3::types::PyString>()
                .unwrap()
                .to_str()
                .unwrap(),
            "hello, world"
        );
    });
}
