use super::{Mapping, Value};

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Self::String(s.to_string())
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<serde_yaml::Value> for Value {
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
    fn from(v: Value) -> Self {
        match v {
            Value::Null => Self::Null,
            Value::Bool(b) => Self::Bool(b),
            Value::Number(n) => Self::Number(n),
            Value::String(s) => Self::String(s),
            Value::Literal(s) => Self::String(s),
            Value::Sequence(s) => {
                let mut seq: Vec<serde_yaml::Value> = Vec::with_capacity(s.len());
                for v in s {
                    seq.push(serde_yaml::Value::from(v));
                }
                Self::Sequence(seq)
            }
            Value::Mapping(m) => Self::Mapping(serde_yaml::Mapping::from(m)),
            Value::ValueList(_) => todo!(),
        }
    }
}

impl From<Mapping> for Value {
    fn from(value: Mapping) -> Self {
        Value::Mapping(value)
    }
}
impl From<serde_yaml::Mapping> for Value {
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
    /// Convert a `Vec` into a `Value::Sequence`
    ///
    /// This implementation works for any `Vec<T>` whose element type can be converted into a
    /// `Value`.
    fn from(value: Vec<T>) -> Self {
        Value::Sequence(value.into_iter().map(Into::into).collect())
    }
}

impl<'a, T: Clone + Into<Value>> From<&'a [T]> for Value {
    /// Convert a slice into a `Value::Sequence`
    ///
    /// This implementation works for any slice `&[T]` whose element type can be converted into a
    /// `Value`.
    fn from(value: &'a [T]) -> Self {
        Value::Sequence(value.iter().cloned().map(Into::into).collect())
    }
}
