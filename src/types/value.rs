// Inspired by `serde_yaml::Value`

use anyhow::{anyhow, Result};
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

    /// Merges Value `other` into self, consuming `other`.
    ///
    /// Currently, this method treats both `Value::Literal(_)` and `Value::String(_)` as literal
    /// strings. This will be changed once we implement rendering of Reclass references.
    ///
    /// This method makes use of `std::mem::replace()` to update self in-place.
    ///
    /// Note that this method will call [`Value::flatten()`] after merging two Mappings to ensure
    /// that the resulting Value doesn't contain any `ValueList` elements.
    fn merge(&mut self, other: Self) -> Result<()> {
        if other.is_null() {
            // Any value can be replaced by null,
            let _prev = std::mem::replace(self, other);
            return Ok(());
        }

        // If `other` is a ValueList, flatten it before trying to merge
        let other = if other.is_value_list() {
            other.flattened()?
        } else if let Some(s) = other.as_str() {
            // make strings into literals
            // TODO(sg): Remove this when we have parameter interpolation
            eprintln!("Transforming unparsed String to Literal");
            Self::Literal(s.to_string())
        } else {
            other
        };

        // we assume that self is already interpolated
        match self {
            // anything can be merged over null
            Self::Null => {
                let _prev = std::mem::replace(self, other);
            }
            Self::Mapping(m) => match other {
                // merge mapping and mapping
                Self::Mapping(other) => {
                    m.merge(&other)?;
                    // Mapping::merge() can produce more ValueLists, so we call flatten here to
                    // ensure that the final result of this method doesn't contain ValueLists.
                    self.flatten()?;
                }
                _ => return Err(anyhow!("Can't merge {} over mapping", other.variant())),
            },
            Self::Sequence(s) => match other {
                // merge sequence and sequence
                Self::Sequence(mut other) => s.append(&mut other),
                _ => return Err(anyhow!("Can't merge {} over sequence", other.variant())),
            },
            Self::String(_) | Self::Literal(_) | Self::Bool(_) | Self::Number(_) => {
                if other.is_mapping() || other.is_sequence() {
                    return Err(anyhow!(
                        "Can't merge {} over {}",
                        other.variant(),
                        self.variant()
                    ));
                }
                let _prev = std::mem::replace(self, other);
            }
            Self::ValueList(_) => {
                // NOTE(sg): We should never end up with nested ValueLists with our implementation
                // of `Mapping::insert()`. If a user constructs a ValueList by hand, it's their job
                // to ensure that they don't construct nested ValueLists.
                unreachable!("Encountered ValueList as merge target, this shouldn't happen!");
            }
        };
        Ok(())
    }

    /// Flattens the Value and returns the resulting Value.
    ///
    /// This method recursively flattens any `ValueList`s which are present in the value or its
    /// children (for `Value::Mapping` and `Value::Sequence`).
    ///
    /// This method leaves the original Value unchanged. Use [`Value::flatten()`] if you want to
    /// flatten a Value in-place.
    pub fn flattened(&self) -> Result<Self> {
        match self {
            Self::ValueList(l) => {
                let mut it = l.iter();
                let mut base = it
                    .next()
                    .ok_or_else(|| anyhow!("Empty valuelist?"))?
                    .clone();

                for v in it {
                    base.merge(v.clone())?;
                }
                Ok(base)
            }
            Self::Mapping(m) => {
                let mut n = Mapping::new();
                for (k, v) in m.iter() {
                    n.insert(k.clone(), v.flattened()?)?;
                }
                Ok(Self::Mapping(n))
            }
            Self::Sequence(s) => {
                let mut n = Vec::with_capacity(s.len());
                for v in s {
                    n.push(v.flattened()?);
                }
                Ok(Self::Sequence(n))
            }
            Self::String(s) => Ok(Self::Literal(s.clone())),
            Self::Null | Self::Bool(_) | Self::Literal(_) | Self::Number(_) => Ok(self.clone()),
        }
    }

    /// Flattens the Value in-place.
    ///
    /// See [`Value::flattened()`] for details.
    pub fn flatten(&mut self) -> Result<()> {
        let _prev = std::mem::replace(self, self.flattened()?);
        Ok(())
    }
}

#[cfg(test)]
mod value_tests;

#[cfg(test)]
mod value_flattened_tests;

#[cfg(test)]
mod value_as_py_obj_tests;
