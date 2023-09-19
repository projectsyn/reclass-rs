// Inspired by `serde_yaml::Value`

use anyhow::{anyhow, Result};
use pyo3::prelude::*;
use serde_yaml::Number;
use std::hash::{Hash, Hasher};
use std::mem;

use super::KeyPrefix;
use super::{Mapping, Sequence};
use crate::refs::{ResolveState, Token};

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
    /// The pretty-printed format doesn't distinguish `String` and `Literal`, and `Sequence` and
    /// `ValueList`. If you need a format where you can distinguish these types, use the Debug
    /// formatter.
    ///
    /// Note that this formatter isn't suitable for generating strings which are suitable for
    /// reference interpolation parameter lookups. Use `Value::raw_string()` if you need a
    /// formatter which generates strings which are compatible with Python's `str()`.
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
            Self::Bool(b) => write!(f, "{b}"),
            Self::Number(n) => write!(f, "{n}"),
            Self::String(s) | Self::Literal(s) => write!(f, "\"{s}\""),
            Self::Sequence(seq) | Self::ValueList(seq) => {
                write!(f, "[")?;
                for (i, v) in seq.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{v}")?;
                }
                write!(f, "]")
            }
            Self::Mapping(m) => write!(f, "{m}"),
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
            Self::Literal(v) | Self::String(v) => v.hash(state),
            Self::Mapping(v) => v.hash(state),
            Self::Sequence(v) | Self::ValueList(v) => v.hash(state),
        }
    }
}

/// The default value is `Value::Null`.
impl Default for Value {
    fn default() -> Self {
        Self::Null
    }
}

impl From<Value> for serde_json::Value {
    fn from(v: Value) -> Self {
        match v {
            Value::Null => Self::Null,
            Value::Bool(b) => Self::Bool(b),
            Value::Number(n) => {
                if n.is_nan() || n.is_infinite() {
                    // Render NaN and -+inf as strings, since JSON's number type doesn't support
                    // those values.
                    return Self::String(n.to_string());
                }
                let jn = if n.is_i64() {
                    // While the lint is enabled generally, we don't care if we lose some precision
                    // here. If this turns out to be a real problem, we can enable serde_json's
                    // arbitrary precision numbers feature.
                    #[allow(clippy::cast_precision_loss)]
                    serde_json::Number::from_f64(n.as_i64().unwrap() as f64).unwrap()
                } else if n.is_u64() {
                    #[allow(clippy::cast_precision_loss)]
                    serde_json::Number::from_f64(n.as_u64().unwrap() as f64).unwrap()
                } else if n.is_f64() {
                    serde_json::Number::from_f64(n.as_f64().unwrap()).unwrap()
                } else {
                    unreachable!("Serializing Number to JSON: {} is neither NaN, inf, or representable as i64, u64, or f64?", n);
                };
                serde_json::Value::Number(jn)
            }
            Value::Literal(s) | Value::String(s) => Self::String(s),
            Value::Sequence(s) => {
                let mut seq: Vec<Self> = Vec::with_capacity(s.len());
                for v in s {
                    seq.push(Self::from(v));
                }
                Self::Array(seq)
            }
            Value::Mapping(m) => Self::Object(serde_json::Map::<String, Self>::from(m)),
            Value::ValueList(_) => todo!(),
        }
    }
}

impl Value {
    /// Checks if the `Value` is `Null`.
    #[inline]
    #[must_use]
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// Checks if the `Value` is a boolean.
    #[inline]
    #[must_use]
    pub fn is_bool(&self) -> bool {
        matches!(self, Self::Bool(_))
    }

    /// If the `Value` is a Boolean, return the associated bool. Returns None otherwise.
    #[inline]
    #[must_use]
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
    #[must_use]
    pub fn is_i64(&self) -> bool {
        match self {
            Self::Number(n) => n.is_i64(),
            _ => false,
        }
    }

    /// If the `Value` is an integer, represent it as i64 if possible. Returns None otherwise.
    #[inline]
    #[must_use]
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
    #[must_use]
    pub fn is_u64(&self) -> bool {
        match self {
            Self::Number(n) => n.is_u64(),
            _ => false,
        }
    }

    /// If the `Value` is an integer, represent it as u64 if possible. Returns None otherwise.
    #[inline]
    #[must_use]
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
    #[must_use]
    pub fn is_f64(&self) -> bool {
        match self {
            Self::Number(n) => n.is_f64(),
            _ => false,
        }
    }

    /// If the `Value` is a number, represent it as f64 if possible. Returns None otherwise.
    #[inline]
    #[must_use]
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
    #[must_use]
    pub fn is_string(&self) -> bool {
        matches!(self, Self::String(_))
    }

    /// Checks if the `Value` is a Literal.
    ///
    /// For any value for which `is_literal()` returns true, `as_str` is guaranteed to return the
    /// string slice.
    #[inline]
    #[must_use]
    pub fn is_literal(&self) -> bool {
        matches!(self, Self::Literal(_))
    }

    /// If the `Value` is a String or Literal, return the associated `str`. Returns None otherwise.
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Literal(s) | Self::String(s) => Some(s),
            _ => None,
        }
    }

    /// Checks if the `Value` is a Mapping.
    #[inline]
    #[must_use]
    pub fn is_mapping(&self) -> bool {
        matches!(self, Self::Mapping(_))
    }

    /// If the value is a Mapping, return a reference to it. Returns None otherwise.
    #[inline]
    #[must_use]
    pub fn as_mapping(&self) -> Option<&Mapping> {
        match self {
            Self::Mapping(m) => Some(m),
            _ => None,
        }
    }

    /// If the value is a Mapping, return a mutable reference to it. Returns None otherwise.
    #[inline]
    #[must_use]
    pub fn as_mapping_mut(&mut self) -> Option<&mut Mapping> {
        match self {
            Self::Mapping(m) => Some(m),
            _ => None,
        }
    }

    /// Checks if the `Value` is a Sequence.
    #[inline]
    #[must_use]
    pub fn is_sequence(&self) -> bool {
        matches!(self, Self::Sequence(_))
    }

    /// If the value is a Sequence, return a reference to it. Returns None otherwise.
    #[inline]
    #[must_use]
    pub fn as_sequence(&self) -> Option<&Sequence> {
        match self {
            Self::Sequence(s) => Some(s),
            _ => None,
        }
    }

    /// If the value is a Sequence, return a mutable reference to it. Returns None otherwise.
    #[inline]
    #[must_use]
    pub fn as_sequence_mut(&mut self) -> Option<&mut Sequence> {
        match self {
            Self::Sequence(s) => Some(s),
            _ => None,
        }
    }

    /// Checks if the `Value` is a ValueList.
    #[inline]
    #[must_use]
    pub fn is_value_list(&self) -> bool {
        matches!(self, Self::ValueList(_))
    }

    /// If the value is a ValueList, return a reference to it. Returns None otherwise.
    #[inline]
    #[must_use]
    pub fn as_value_list(&self) -> Option<&Sequence> {
        match self {
            Self::ValueList(l) => Some(l),
            _ => None,
        }
    }

    /// If the value is a ValueList, return a mutable reference to it. Returns None otherwise.
    #[inline]
    #[must_use]
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
    #[must_use]
    pub fn get(&self, k: &Value) -> Option<&Value> {
        match self {
            Self::Mapping(m) => m.get(k),
            Self::Sequence(s) | Self::ValueList(s) => {
                if let Some(idx) = k.as_u64() {
                    if let Ok(idx) = usize::try_from(idx) {
                        if idx < s.len() {
                            return Some(&s[idx]);
                        }
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
                    let idx = usize::try_from(idx)?;
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
    #[allow(clippy::missing_panics_doc)]
    pub fn as_py_obj(&self, py: Python<'_>) -> PyResult<PyObject> {
        let obj = match self {
            Value::Literal(s) | Value::String(s) => s.into_py(py),
            Value::Bool(b) => b.into_py(py),
            Value::Number(n) => {
                if n.is_i64() {
                    // NOTE(sg): We allow the missing panics doc because we already checked that
                    // `as_i64()` can't panic here.
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
                for v in s {
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
    pub(super) fn strip_prefix(self) -> (Self, Option<KeyPrefix>) {
        match self {
            Self::String(s) => {
                if s.is_empty() {
                    return (Self::String(s), None);
                }
                let p = KeyPrefix::from(s.chars().next().unwrap());
                if p.is_some() {
                    (Self::String(s[1..].to_string()), p)
                } else {
                    (Self::String(s), None)
                }
            }
            _ => (self, None),
        }
    }

    /// Renders the value as a string which is suitable for doing value lookups during parameter
    /// interpolation. Returns an error when called on ValueLists or Strings.
    ///
    /// Notably, this implementation won't quote Literal values, and will try to emit
    /// strings which match Python's `str()` for other types.
    ///
    #[inline]
    pub(crate) fn raw_string(&self) -> Result<String> {
        match self {
            Value::Literal(s) => Ok(s.clone()),
            // We serialize Null as `None` to be compatible with Python's str()
            Value::Null => Ok("None".to_string()),
            // We need custom formatting for bool instead of `format!("{b}")`, so that this
            // function returns strings which match Python's `str()` implementation.
            Value::Bool(b) => match b {
                true => Ok("True".to_owned()),
                false => Ok("False".to_owned()),
            },
            // NOTE(sg): We render maps and sequences as JSON to mimic python reclass's behavior of
            // just using `str(obj)`.
            // This doesn't result in 100% identical output (e.g. double quotes instead of single
            // quotes), but works similar enough in the resulting YAML. Serializing to YAML doesn't
            // work cleanly for embedded references in multiline strings which contain YAML, as the
            // indentation will break.
            Value::Mapping(m) => {
                let m = serde_json::Map::<String, serde_json::Value>::from(m.clone());
                serde_json::to_string(&m).map_err(|e| anyhow!(e))
            }
            Value::Sequence(_) => {
                let v = serde_json::Value::from(self.clone());
                serde_json::to_string(&v).map_err(|e| anyhow!(e))
            }
            Value::Number(n) => Ok(n.to_string()),
            _ => Err(anyhow!(
                "Value::raw_string isn't implemented for {}",
                self.variant()
            )),
        }
    }

    /// Parses and interpolates any Reclass references present in the value.  The returned value
    /// will never be a `Value::String`.
    ///
    /// Note that users should prefer calling `Value::rendered()` or one of its in-place variants
    /// over this method.
    pub(crate) fn interpolate(&self, root: &Mapping, state: &mut ResolveState) -> Result<Self> {
        Ok(match self {
            Self::String(s) => {
                // String interpolation parses any Reclass references in the String and resolves
                // them. The result of `Token::render()` can be an arbitrary Value, except for
                // `Value::String()`, since `render()` will recursively call `interpolate()`.
                if let Some(token) = Token::parse(s)? {
                    token.render(root, state)?
                } else {
                    // If Token::parse() returns None, we can be sure that there's no references
                    // int the String, and just return the string as a `Value::Literal`.
                    Self::Literal(s.clone())
                }
            }
            // Mappings are interpolated by calling `Mapping::interpolate()`.
            Self::Mapping(m) => Self::Mapping(m.interpolate(root, state)?),
            Self::Sequence(s) => {
                // Sequences are interpolated by calling interpolate() for each element.
                let mut seq = vec![];
                for it in s {
                    // References in separate entries in sequences can't form loops. Therefore we
                    // pass a copy of the current resolution state to the recursive call for each
                    // element. We don't need to update the input state after we're done with a
                    // Sequence either, since there's no potential to start recursing again, if
                    // we've fully interpolated a Sequence.
                    let mut st = state.clone();
                    let e = it.interpolate(root, &mut st)?;
                    seq.push(e);
                }
                Self::Sequence(seq)
            }
            Self::ValueList(l) => {
                // iteratively interpolate each element of the ValueList, by starting with a base
                // Value::Null, and merging each interpolated element over that base value. This
                // correctly handles cases where an intermediate layer of a ValueList is a
                // reference to a Mapping.
                // NOTE(sg): Empty ValueLists are interpolated as Value::Null.
                let mut r = Value::Null;
                for v in l {
                    // For each ValueList layer, we pass a copy of the current resolution state to
                    // the recursive call to interpolate, since references in different ValueList
                    // layers can't form loops with each other (Intuitively: either we manage to
                    // resolve all references in a ValueList layer, or we don't, but once we're
                    // done with a layer, any references that we saw there have been successfully
                    // resolved, and don't matter for the next layer we're interpolating).
                    let mut st = state.clone();
                    r.merge(v.interpolate(root, &mut st)?)?;
                }
                // Depending on the structure of the ValueList, we may end up with a final
                // interpolated Value which contains more ValueLists due to mapping merges. Such
                // ValueLists can themselves contain further references. To handle this case, we
                // call `interpolate()` again to resolve those references. This recursion stops
                // once `Token::render()` doesn't produce new `Value::String()`.
                // For this interpolation, we need to actually update the resolution state, so we
                // pass in the `state` which we were called with.
                r.interpolate(root, state)?
            }
            _ => self.clone(),
        })
    }

    /// Merges Value `other` into self, consuming `other`.
    ///
    /// This method assumes that it's called from [`Value::flatten()`], and will raise an error
    /// when called on a [`Value::ValueList`]. Additionally, the method assumes
    /// [`Value::interpolate()`] has already been called, and will raise an error when called on a
    /// [`Value::String`].
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
                Self::Mapping(other) => m.merge(&other)?,
                _ => return Err(anyhow!("Can't merge {} over mapping", other.variant())),
            },
            Self::Sequence(s) => match other {
                // merge sequence and sequence
                Self::Sequence(mut other) => s.append(&mut other),
                _ => return Err(anyhow!("Can't merge {} over sequence", other.variant())),
            },
            Self::Literal(_) | Self::Bool(_) | Self::Number(_) => {
                if other.is_mapping() || other.is_sequence() {
                    // We can't merge simple non-null types over mappings or sequences
                    return Err(anyhow!(
                        "Can't merge {} over {}",
                        other.variant(),
                        self.variant()
                    ));
                }
                // overwrite self with the value that's being merged
                let _prev = std::mem::replace(self, other);
            }
            Self::String(_) => {
                unreachable!("Encountered unparsed String as merge target, this shouldn't happen!");
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
    ///
    /// Note that we don't recommend calling `flattened()` on arbitrary Values. Users should always
    /// prefer calling [`Value::rendered()`] or one of the in-place variations of that method.
    pub(crate) fn flattened(&self) -> Result<Self> {
        match self {
            // Flatten ValueList by iterating over its elements and merging each element into a
            // base Value.
            Self::ValueList(l) => {
                // NOTE(sg): Empty ValueLists get flattened to Value::Null
                let mut base = Value::Null;
                for v in l {
                    base.merge(v.clone())?;
                }
                Ok(base)
            }
            // Flatten Mapping by flattening each value and inserting it into a new Mapping.
            Self::Mapping(m) => Ok(Self::Mapping(m.flattened()?)),
            // Flatten Sequence by flattening each element and inserting it into a new Sequence
            Self::Sequence(s) => {
                let mut n = Vec::with_capacity(s.len());
                for v in s {
                    n.push(v.flattened()?);
                }
                Ok(Self::Sequence(n))
            }
            // Simple values are flattened as themselves
            Self::Null | Self::Bool(_) | Self::Literal(_) | Self::Number(_) => Ok(self.clone()),
            // Flattening an unparsed string is an error
            Self::String(_) => Err(anyhow!(
                "Can't flatten unparsed String, did you mean to call `rendered()`?"
            )),
        }
    }

    /// Flattens the Value in-place.
    ///
    /// See [`Value::flattened()`] for details.
    pub(super) fn flatten(&mut self) -> Result<()> {
        let _prev = std::mem::replace(self, self.flattened()?);
        Ok(())
    }

    /// Renders the Value by interpolating Reclass references and flattening ValueLists.
    ///
    /// This method should be preferred over calling `Value::interpolate()` and
    /// `Value::flattened()` directly.
    ///
    /// The method first interpolates any Reclass references found in the Value by looking up the
    /// reference keys in `root`. After all references have been interpolated, the method flattens
    /// any remaining ValueLists and returns the final "flattened" value.
    pub fn rendered(&self, root: &Mapping) -> Result<Self> {
        let mut state = ResolveState::default();
        let mut v = self
            .interpolate(root, &mut state)
            .map_err(|e| anyhow!("While resolving references in {self}: {e}"))?;
        v.flatten()?;
        Ok(v)
    }

    /// Renders the Value in-place.
    ///
    /// See [`Value::rendered()`] for details.
    pub fn render(&mut self, root: &Mapping) -> Result<()> {
        let _prev = std::mem::replace(self, self.rendered(root)?);
        Ok(())
    }

    /// Renders the Value in-place if it's a Mapping, using itself as the parameter lookup source.
    /// Returns an error when called for a Value variant other than `Value::Mapping`.
    ///
    /// See [`Value::rendered()`] for details on how Reclass references are rendered.
    pub fn render_with_self(&mut self) -> Result<()> {
        let m = self.as_mapping().ok_or_else(|| {
            anyhow!(
                "Can't render {} with itself as the parameter source",
                self.variant()
            )
        })?;
        let n = self.rendered(m)?;
        let _prev = std::mem::replace(self, n);
        Ok(())
    }
}

#[cfg(test)]
mod value_tests;

#[cfg(test)]
mod value_flattened_tests;

#[cfg(test)]
mod value_interpolate_tests;

#[cfg(test)]
mod value_as_py_obj_tests;
