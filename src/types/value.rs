// Inspired by `serde_yaml::Value`

use anyhow::Result;
use pyo3::prelude::*;
use serde_yaml::Number;
use std::hash::{Hash, Hasher};
use std::mem;

use super::KeyPrefix;
use super::{Mapping, Sequence};

/// Represents a YAML value in a form suitable for processing Reclass parameters.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// Represents a YAML null value.
    Null,
    /// Represents a YAML boolean value.
    Bool(bool),
    /// Represents a raw string value which may contain reclass references.
    String(String),
    /// Represents a string literal value which can't contain reclass references.
    Literal(String),
    /// Represents a YAML numerical value.
    Number(Number),
    /// Represents a YAML mapping.
    Mapping(Mapping),
    /// Represents a YAML sequence.
    Sequence(Sequence),
    /// Represents a list of layered values which may have different types. ValueLists are
    /// flattened during reference interpolation.
    ValueList(Sequence),
}

impl std::fmt::Display for Value {
    /// Pretty prints the `Value`
    ///
    /// Note that the pretty-printed format doesn't distinguish `String` and `Literal`, and
    /// `Sequence` and `ValueList`. If you need a format where you can distinguish these types, use
    /// the Debug formatter.
    ///
    /// # Example
    ///
    /// ```
    /// use reclass_rs::types::{Mapping, Value};
    /// use std::str::FromStr;
    ///
    /// let input = r#"
    /// foo: bar
    /// baz: True
    /// bar:
    ///   qux: [1,2,3,4.5]
    ///   zap: ~
    /// "#;

    /// let v = Value::from(Mapping::from_str(input).unwrap());
    /// assert_eq!(
    ///     v.to_string(),
    ///     r#"{"foo": "bar", "baz": true, "bar": {"qux": [1, 2, 3, 4.5], "zap": Null}}"#
    /// );
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Null => write!(f, "Null"),
            Self::Bool(b) => write!(f, "{}", b),
            Self::Number(n) => write!(f, "{}", n),
            Self::String(s) | Self::Literal(s) => write!(f, "\"{}\"", s),
            Self::Sequence(seq) | Self::ValueList(seq) => {
                write!(f, "[")?;
                for (i, v) in seq.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", v)?;
                }
                write!(f, "]")
            }
            Self::Mapping(m) => write!(f, "{}", m),
        }
    }
}

impl Eq for Value {}

impl Hash for Value {
    fn hash<H: Hasher>(&self, state: &mut H) {
        mem::discriminant(self).hash(state);
        match self {
            Self::Null => {}
            Self::Bool(v) => v.hash(state),
            Self::Number(v) => v.hash(state),
            Self::String(v) => v.hash(state),
            Self::Sequence(v) => v.hash(state),
            Self::Mapping(v) => v.hash(state),
            Self::ValueList(v) => v.hash(state),
            Self::Literal(v) => v.hash(state),
        }
    }
}

/// The default value is `Value::Null`.
impl Default for Value {
    fn default() -> Self {
        Self::Null
    }
}

impl Value {
    /// Checks if the `Value` is `Null`.
    #[inline]
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// Checks if the `Value` is a boolean.
    #[inline]
    pub fn is_bool(&self) -> bool {
        matches!(self, Self::Bool(_))
    }

    /// If the `Value` is a Boolean, return the associated bool. Returns None otherwise.
    #[inline]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Returns true if the `Value` is an integer between `i64::MIN` and `i64::MAX`.
    ///
    /// For any value for which `is_i64` returns true, `as_i64` is guaranteed to return the
    /// integer value.
    #[inline]
    pub fn is_i64(&self) -> bool {
        match self {
            Self::Number(n) => n.is_i64(),
            _ => false,
        }
    }

    /// If the `Value` is an integer, represent it as i64 if possible. Returns None otherwise.
    #[inline]
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Number(n) => n.as_i64(),
            _ => None,
        }
    }

    /// Returns true if the `Value` is an integer between `u64::MIN` and `u64::MAX`.
    ///
    /// For any value for which `is_u64` returns true, `as_u64` is guaranteed to return the
    /// integer value.
    #[inline]
    pub fn is_u64(&self) -> bool {
        match self {
            Self::Number(n) => n.is_u64(),
            _ => false,
        }
    }

    /// If the `Value` is an integer, represent it as u64 if possible. Returns None otherwise.
    #[inline]
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Self::Number(n) => n.as_u64(),
            _ => None,
        }
    }

    /// Returns true if the `Value` can be represented by f64.
    ///
    /// For any value for which `is_f64` returns true, `as_f64` is guaranteed to return the
    /// floating point value.
    ///
    /// Because we rely on the `serde_yaml::Number` type to implement this function, it currently
    /// returns true if and only if both `is_i64` and `is_u64` return false, but since serde_yaml
    /// doesn't guarantee this behavior in the future, this may change.
    #[inline]
    pub fn is_f64(&self) -> bool {
        match self {
            Self::Number(n) => n.is_f64(),
            _ => false,
        }
    }

    /// If the `Value` is a number, represent it as f64 if possible. Returns None otherwise.
    #[inline]
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Number(n) => n.as_f64(),
            _ => None,
        }
    }

    /// Checks if the `Value` is a String.
    ///
    /// For any value for which `is_string()` returns true, `as_str` is guaranteed to return the
    /// string slice.
    #[inline]
    pub fn is_string(&self) -> bool {
        matches!(self, Self::String(_))
    }

    /// Checks if the `Value` is a Literal.
    ///
    /// For any value for which `is_literal()` returns true, `as_str` is guaranteed to return the
    /// string slice.
    #[inline]
    pub fn is_literal(&self) -> bool {
        matches!(self, Self::Literal(_))
    }

    /// If the `Value` is a String or Literal, return the associated `str`. Returns None otherwise.
    #[inline]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            Self::Literal(s) => Some(s),
            _ => None,
        }
    }

    /// Checks if the `Value` is a Mapping.
    #[inline]
    pub fn is_mapping(&self) -> bool {
        matches!(self, Self::Mapping(_))
    }

    /// If the value is a Mapping, return a reference to it. Returns None otherwise.
    #[inline]
    pub fn as_mapping(&self) -> Option<&Mapping> {
        match self {
            Self::Mapping(m) => Some(m),
            _ => None,
        }
    }

    /// If the value is a Mapping, return a mutable reference to it. Returns None otherwise.
    #[inline]
    pub fn as_mapping_mut(&mut self) -> Option<&mut Mapping> {
        match self {
            Self::Mapping(m) => Some(m),
            _ => None,
        }
    }

    /// Checks if the `Value` is a Sequence.
    #[inline]
    pub fn is_sequence(&self) -> bool {
        matches!(self, Self::Sequence(_))
    }

    /// If the value is a Sequence, return a reference to it. Returns None otherwise.
    #[inline]
    pub fn as_sequence(&self) -> Option<&Sequence> {
        match self {
            Self::Sequence(s) => Some(s),
            _ => None,
        }
    }

    /// If the value is a Sequence, return a mutable reference to it. Returns None otherwise.
    #[inline]
    pub fn as_sequence_mut(&mut self) -> Option<&mut Sequence> {
        match self {
            Self::Sequence(s) => Some(s),
            _ => None,
        }
    }

    /// Checks if the `Value` is a ValueList.
    #[inline]
    pub fn is_value_list(&self) -> bool {
        matches!(self, Self::ValueList(_))
    }

    /// If the value is a ValueList, return a reference to it. Returns None otherwise.
    #[inline]
    pub fn as_value_list(&self) -> Option<&Sequence> {
        match self {
            Self::ValueList(l) => Some(l),
            _ => None,
        }
    }

    /// If the value is a ValueList, return a mutable reference to it. Returns None otherwise.
    #[inline]
    pub fn as_value_list_mut(&mut self) -> Option<&mut Sequence> {
        match self {
            Self::ValueList(l) => Some(l),
            _ => None,
        }
    }

    /// Access elements in a Sequence or Mapping, returning a reference to the value if the given
    /// key exists. Returns None otherwise.
    ///
    /// An arbitrary `Value` key can be used to access a value in a Mapping. A `Value::Number`
    /// which is within the bounds of the underlying sequence can be used to access a value in a
    /// Sequence or a ValueList.
    ///
    /// Returns None for invalid keys, or keys which don't exist in the `Value`.
    #[inline]
    pub fn get(&self, k: &Value) -> Option<&Value> {
        match self {
            Self::Mapping(m) => m.get(k),
            Self::Sequence(s) | Self::ValueList(s) => {
                if let Some(idx) = k.as_u64() {
                    let idx = idx as usize;
                    if idx < s.len() {
                        return Some(&s[idx]);
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Access elements in a Sequence or Mapping, returning a mutable reference to the value if the
    /// given key exists. Returns None otherwise.
    ///
    /// An arbitrary `Value` key can be used to access a value in a Mapping. A `Value::Number`
    /// which is within the bounds of the underlying sequence can be used to access a value in a
    /// Sequence or a ValueList.
    ///
    /// Returns None for invalid keys, or keys which don't exist in the `Value`.
    /// Returns an error when trying to access a constant key in a Mapping.
    #[inline]
    pub fn get_mut(&mut self, k: &Value) -> Result<Option<&mut Value>> {
        match self {
            Self::Mapping(m) => m.get_mut(k),
            Self::Sequence(s) | Self::ValueList(s) => {
                if let Some(idx) = k.as_u64() {
                    let idx = idx as usize;
                    if idx < s.len() {
                        return Ok(Some(&mut s[idx]));
                    }
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    /// Provides a nice string for each enum variant for debugging and pretty-printing.
    #[allow(unused)]
    pub(crate) fn variant(&self) -> &str {
        match self {
            Self::Bool(_) => "Value::Bool",
            Self::Mapping(_) => "Value::Mapping",
            Self::Null => "Value::Null",
            Self::Number(_) => "Value::Number",
            Self::Sequence(_) => "Value::Sequence",
            Self::String(_) => "Value::String",
            Self::Literal(_) => "Value::Literal",
            Self::ValueList(_) => "Value::ValueList",
        }
    }

    /// Converts the `Value` into a `PyObject`.
    pub fn as_py_obj(&self, py: Python<'_>) -> PyResult<PyObject> {
        let obj = match self {
            Value::Literal(s) | Value::String(s) => s.into_py(py),
            Value::Bool(b) => b.into_py(py),
            Value::Number(n) => {
                if n.is_i64() {
                    n.as_i64().unwrap().into_py(py)
                } else if n.is_u64() {
                    n.as_u64().unwrap().into_py(py)
                } else if n.is_f64() {
                    n.as_f64().unwrap().into_py(py)
                } else {
                    Option::<()>::None.into_py(py)
                }
            }
            Value::Sequence(s) => {
                let mut pyseq = vec![];
                for v in s.iter() {
                    pyseq.push(v.as_py_obj(py)?);
                }
                pyseq.into_py(py)
            }
            Value::Mapping(m) => m.as_py_dict(py)?.into(),
            Value::Null => Option::<()>::None.into_py(py),
            // ValueList should never get emitted to Python
            Value::ValueList(_) => unreachable!(),
        };
        Ok(obj)
    }

    /// Handles special mapping key prefix values for `String` Values.
    ///
    /// For String values, if a mapping key prefix is present, the prefix is stripped from the
    /// String, and the corresponding `KeyPrefix` variant is returned. Otherwise, the string is
    /// cloned and returned.
    ///
    /// For non-String values, the value is unconditionally cloned and returned unmodified.
    #[inline]
    pub(super) fn strip_prefix(&self) -> (Self, Option<KeyPrefix>) {
        match self {
            Self::String(s) => {
                if s.is_empty() {
                    return (self.clone(), None);
                }
                let p = KeyPrefix::from(s.chars().next().unwrap());
                if p.is_some() {
                    (Self::String(s[1..].to_string()), p)
                } else {
                    (self.clone(), None)
                }
            }
            _ => (self.clone(), None),
        }
    }
}

#[cfg(test)]
mod value_tests {
    use super::*;
    use paste::paste;

    #[test]
    fn test_is_null() {
        assert!(Value::Null.is_null());
        assert!(!Value::Bool(true).is_null());
    }

    #[test]
    fn test_is_bool() {
        assert!(!Value::Null.is_bool());
        assert!(Value::Bool(true).is_bool());
    }

    #[test]
    fn test_as_bool() {
        let b = Value::Bool(true);
        assert_eq!(b.as_bool(), Some(true));
        assert_eq!(Value::Null.as_bool(), None);
    }

    macro_rules! test_number {
        ($($ty:ident $val:expr)*) => {
            $(
                paste! {
                #[test]
                fn [<test_is_ $ty>]() {
                    assert!(!Value::Null.[<is_ $ty>]());
                    let n: $ty = $val;
                    let n = Value::Number(n.into());
                    assert!(n.[<is_ $ty>]());
                }

                #[test]
                fn [<test_as_ $ty>]() {
                    assert_eq!(Value::Null.[<as_ $ty>](), None);
                    let n: $ty = $val;
                    let n = Value::Number(n.into());
                    assert_eq!(n.[<as_ $ty>](), Some($val));
                }
                }
            )*
        }
    }
    test_number! { u64 5 i64 -3 f64 3.14 }

    #[test]
    fn test_is_string() {
        assert!(!Value::Null.is_string());
        let s = Value::from("foo");
        assert!(s.is_string());
        assert!(!s.is_literal());
    }

    #[test]
    fn test_is_literal() {
        assert!(!Value::Null.is_literal());
        let s = Value::Literal("foo".into());
        assert!(s.is_literal());
        assert!(!s.is_string());
    }

    #[test]
    fn test_as_str() {
        assert_eq!(Value::Null.as_str(), None);

        let s = Value::Literal("foo".into());
        assert_eq!(s.as_str(), Some("foo"));

        let s = Value::from("foo");
        assert_eq!(s.as_str(), Some("foo"));
    }

    #[test]
    fn test_is_mapping() {
        assert!(!Value::Null.is_mapping());
        let m = Value::from(Mapping::new());
        assert!(m.is_mapping());
    }

    #[test]
    fn test_as_mapping() {
        assert_eq!(Value::Null.as_mapping(), None);
        let m = Value::from(Mapping::new());
        assert_eq!(m.as_mapping(), Some(&Mapping::new()));
    }

    #[test]
    fn test_as_mapping_mut() {
        assert_eq!(Value::Null.as_mapping_mut(), None);
        let mut m = Value::from(Mapping::new());
        let map = m.as_mapping_mut().unwrap();
        map.insert("foo".into(), "bar".into()).unwrap();
        assert_eq!(
            m.as_mapping(),
            Some(&Mapping::from_iter(vec![("foo".into(), "bar".into())]))
        );
    }

    #[test]
    fn test_is_sequence() {
        assert!(!Value::Null.is_sequence());
        let s = Value::from(Sequence::new());
        assert!(s.is_sequence());
    }

    #[test]
    fn test_as_sequence() {
        assert_eq!(Value::Null.as_sequence(), None);
        let s = Value::from(Sequence::new());
        assert_eq!(s.as_sequence(), Some(&Sequence::new()));
    }

    #[test]
    fn test_as_sequence_mut() {
        assert_eq!(Value::Null.as_sequence_mut(), None);
        let mut s = Value::from(Sequence::new());
        let seq = s.as_sequence_mut().unwrap();
        seq.push("foo".into());
        assert_eq!(
            s.as_sequence(),
            Some(&Sequence::from_iter(vec!["foo".into()]))
        );
    }

    #[test]
    fn test_is_value_list() {
        assert!(!Value::Null.is_value_list());
        let l = Value::ValueList(Sequence::new());
        assert!(l.is_value_list());
    }

    #[test]
    fn test_as_value_list() {
        assert_eq!(Value::Null.as_value_list(), None);
        let l = Value::ValueList(Sequence::new());
        assert_eq!(l.as_value_list(), Some(&Sequence::new()));
    }

    #[test]
    fn test_as_value_list_mut() {
        assert_eq!(Value::Null.as_value_list_mut(), None);
        let mut l = Value::ValueList(Sequence::new());
        let seq = l.as_value_list_mut().unwrap();
        seq.push("foo".into());
        assert_eq!(
            l.as_value_list(),
            Some(&Sequence::from_iter(vec!["foo".into()]))
        );
    }

    #[test]
    fn test_get_mapping() {
        let m = Mapping::from_iter(vec![("a".into(), 1.into()), (2.into(), "foo".into())]);
        let m = Value::from(m);

        assert_eq!(m.get(&"a".into()), Some(&1.into()));
        assert_eq!(m.get(&2.into()), Some(&"foo".into()));
        assert_eq!(m.get(&"b".into()), None);
    }

    #[test]
    fn test_get_mut_mapping() {
        let m = Mapping::from_iter(vec![("a".into(), 1.into())]);
        let mut m = Value::from(m);

        assert_eq!(m.get(&"a".into()), Some(&1.into()));
        let a = m.get_mut(&"a".into()).unwrap().unwrap();
        *a = "foo".into();
        assert_eq!(m.get(&"a".into()), Some(&"foo".into()));
        assert_eq!(m.get_mut(&"b".into()).unwrap(), None);
    }

    #[test]
    fn test_get_mut_mapping_const_key() {
        let m = Mapping::from_iter(vec![("=a".into(), 1.into())]);
        let mut m = Value::from(m);

        assert_eq!(m.get(&"a".into()), Some(&1.into()));
        assert!(m.get_mut(&"a".into()).is_err());
    }

    #[test]
    fn test_get_sequence() {
        let s = Sequence::from_iter(vec!["a".into(), 2.into(), 3.14.into()]);
        let s = Value::from(s);

        // non-u64 and out of bounds accesses return None
        assert_eq!(s.get(&(-1).into()), None);
        assert_eq!(s.get(&3.14.into()), None);
        assert_eq!(s.get(&3.into()), None);

        // non-number accesses return None
        assert_eq!(s.get(&"foo".into()), None);

        assert_eq!(s.get(&0.into()), Some(&"a".into()));
        assert_eq!(s.get(&1.into()), Some(&2.into()));
        assert_eq!(s.get(&2.into()), Some(&3.14.into()));
    }

    #[test]
    fn test_get_mut_sequence() {
        let s = Sequence::from_iter(vec!["a".into(), 2.into(), 3.14.into()]);
        let mut s = Value::from(s);

        assert_eq!(s.get(&0.into()), Some(&"a".into()));
        let e0 = s.get_mut(&0.into()).unwrap().unwrap();
        *e0 = "foo".into();
        assert_eq!(s.get(&0.into()), Some(&"foo".into()));
        assert_eq!(s.get_mut(&3.into()).unwrap(), None);
    }

    #[test]
    fn test_get_valuelist() {
        let s = Sequence::from_iter(vec!["a".into(), 2.into(), 3.14.into()]);
        let l = Value::ValueList(s);

        // non-u64 and out of bounds accesses return None
        assert_eq!(l.get(&(-1).into()), None);
        assert_eq!(l.get(&3.14.into()), None);
        assert_eq!(l.get(&3.into()), None);

        // non-number accesses return None
        assert_eq!(l.get(&"foo".into()), None);

        assert_eq!(l.get(&0.into()), Some(&"a".into()));
        assert_eq!(l.get(&1.into()), Some(&2.into()));
        assert_eq!(l.get(&2.into()), Some(&3.14.into()));
    }

    #[test]
    fn test_get_mut_valuelist() {
        let s = Sequence::from_iter(vec!["a".into(), 2.into(), 3.14.into()]);
        let mut l = Value::ValueList(s);

        assert_eq!(l.get(&0.into()), Some(&"a".into()));
        let e0 = l.get_mut(&0.into()).unwrap().unwrap();
        *e0 = "foo".into();
        assert_eq!(l.get(&0.into()), Some(&"foo".into()));
        assert_eq!(l.get_mut(&3.into()).unwrap(), None);
    }

    #[test]
    fn test_get_other_types() {
        assert_eq!(Value::Null.get(&"a".into()), None);
        assert_eq!(Value::Bool(true).get(&"a".into()), None);
        assert_eq!(Value::String("foo".into()).get(&"a".into()), None);
        // Strings can't be treated as sequences
        assert_eq!(Value::String("foo".into()).get(&0.into()), None);
        assert_eq!(Value::Literal("foo".into()).get(&"a".into()), None);
        assert_eq!(Value::Number(1.into()).get(&"a".into()), None);
    }

    #[test]
    fn test_get_mut_other_types() {
        assert_eq!(Value::Null.get_mut(&"a".into()).unwrap(), None);
        assert_eq!(Value::Bool(true).get_mut(&"a".into()).unwrap(), None);
        assert_eq!(
            Value::String("foo".into()).get_mut(&"a".into()).unwrap(),
            None
        );
        // Strings can't be treated as sequences
        assert_eq!(
            Value::String("foo".into()).get_mut(&0.into()).unwrap(),
            None
        );
        assert_eq!(
            Value::Literal("foo".into()).get_mut(&"a".into()).unwrap(),
            None
        );
        assert_eq!(Value::Number(1.into()).get_mut(&"a".into()).unwrap(), None);
    }

    #[test]
    fn test_strip_prefix() {
        let k1 = Value::from("=foo");
        let k2 = Value::from("~foo");
        let k3 = Value::from("foo");
        let k4 = Value::from(3);
        assert_eq!(
            k1.strip_prefix(),
            (Value::from("foo"), Some(KeyPrefix::Constant))
        );
        assert_eq!(
            k2.strip_prefix(),
            (Value::from("foo"), Some(KeyPrefix::Override))
        );
        assert_eq!(k3.strip_prefix(), (Value::from("foo"), None));
        assert_eq!(k4.strip_prefix(), (Value::from(3), None));
    }
}

#[cfg(test)]
mod value_as_py_obj_tests {
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
}
