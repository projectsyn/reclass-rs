use super::{Mapping, Value};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PySequence};

impl From<&str> for Value {
    /// Converts a string slice into a `Value::String`.
    fn from(s: &str) -> Self {
        Self::String(s.to_string())
    }
}

impl From<String> for Value {
    /// Converts a String into a `Value::String`.
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<serde_yaml::Value> for Value {
    /// Converts a `serde_yaml::Value` into a `Value`.
    ///
    /// `serde_yaml::Value::String` is always converted into `Value::String`.
    ///
    /// `serde_yaml::Tagged` values are not supported yet.
    fn from(v: serde_yaml::Value) -> Self {
        match v {
            serde_yaml::Value::Null => Self::Null,
            serde_yaml::Value::Bool(b) => Self::Bool(b),
            serde_yaml::Value::Number(n) => Self::Number(n),
            serde_yaml::Value::String(s) => Self::String(s),
            serde_yaml::Value::Sequence(s) => {
                let mut seq: Vec<Value> = Vec::with_capacity(s.len());
                for v in s {
                    seq.push(Value::from(v));
                }
                Self::Sequence(seq)
            }
            serde_yaml::Value::Mapping(m) => Self::Mapping(Mapping::from(m)),
            serde_yaml::Value::Tagged(_) => {
                todo!("Tagged YAML values are not supported yet");
            }
        }
    }
}

impl From<Value> for serde_yaml::Value {
    /// Converts a `Value` into a `serde_yaml::Value`.
    ///
    /// `Value::String` and `Value::Literal` are both converted to `serde_yaml::Value::String`.
    ///
    /// `Value::ValueList` is converted to `serde_yaml::Value::Sequence`.
    fn from(v: Value) -> Self {
        match v {
            Value::Null => Self::Null,
            Value::Bool(b) => Self::Bool(b),
            Value::Number(n) => Self::Number(n),
            Value::Literal(s) | Value::String(s) => Self::String(s),
            Value::Sequence(s) | Value::ValueList(s) => {
                let mut seq: Vec<serde_yaml::Value> = Vec::with_capacity(s.len());
                for v in s {
                    seq.push(serde_yaml::Value::from(v));
                }
                Self::Sequence(seq)
            }
            Value::Mapping(m) => Self::Mapping(serde_yaml::Mapping::from(m)),
        }
    }
}

impl From<Mapping> for Value {
    /// Converts a `Mapping` into a `Value::Mapping`.
    fn from(value: Mapping) -> Self {
        Value::Mapping(value)
    }
}
impl From<serde_yaml::Mapping> for Value {
    /// Converts a `serde_yaml::Mapping` into a `Value::Mapping`
    fn from(value: serde_yaml::Mapping) -> Self {
        Value::Mapping(value.into())
    }
}

// inspired by serde_yaml::Value, saves us some repetition
macro_rules! from_number {
    ($($ty:ident)*) => {
        $(
            impl From<$ty> for Value {
                fn from(n: $ty) -> Self {
                    Value::Number(n.into())
                }
            }
        )*
    }
}

from_number! {
    i8 i16 i32 i64 isize
    u8 u16 u32 u64 usize
    f32 f64
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    /// Converts a `Vec` into a `Value::Sequence`.
    ///
    /// This implementation works for any `Vec<T>` whose element type can be converted into a
    /// `Value`.
    fn from(value: Vec<T>) -> Self {
        Value::Sequence(value.into_iter().map(Into::into).collect())
    }
}

impl<'a, T: Clone + Into<Value>> From<&'a [T]> for Value {
    /// Converts a slice into a `Value::Sequence`.
    ///
    /// This implementation works for any slice `&[T]` whose element type can be converted into a
    /// `Value`.
    fn from(value: &'a [T]) -> Self {
        Value::Sequence(value.iter().cloned().map(Into::into).collect())
    }
}

impl TryFrom<Bound<'_, PyAny>> for Value {
    type Error = PyErr;

    fn try_from(value: Bound<'_, PyAny>) -> PyResult<Self> {
        match value.get_type().name()?.to_str()? {
            "str" => {
                let v = value.extract::<&str>()?;
                Ok(Self::String(v.to_string()))
            }
            "list" => {
                let v = value.downcast::<PySequence>()?;
                let mut items: Vec<Value> = vec![];
                for it in v.try_iter()? {
                    items.push(TryInto::try_into(it?)?);
                }
                Ok(Self::Sequence(items))
            }
            "dict" => {
                let dict = value.downcast::<PyDict>()?;
                let mut mapping = crate::types::Mapping::new();
                for (k, v) in dict {
                    let kv = TryInto::try_into(k)?;
                    let vv = TryInto::try_into(v)?;
                    mapping.insert(kv, vv).map_err(|e| {
                        PyValueError::new_err(format!("Error inserting into mapping: {e}"))
                    })?;
                }
                Ok(Self::Mapping(mapping))
            }
            "bool" => {
                let v = value.extract::<bool>()?;
                Ok(Self::Bool(v))
            }
            "int" | "float" => {
                let v = value.extract::<f64>()?;
                let n = serde_yaml::Number::from(v);
                Ok(Self::Number(n))
            }

            _ => Err(PyValueError::new_err(format!(
                "Conversion from Python type to reclass_rs::Value isn't implemented for <class '{}'>",
                value.get_type().name()?
            ))),
        }
    }
}
