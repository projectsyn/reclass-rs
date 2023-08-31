// Inspired by `serde_yaml::Value`

use serde_yaml::Number;
use std::hash::{Hash, Hasher};
use std::mem;

use super::KeyPrefix;
use super::{Mapping, Sequence};

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// Represents a YAML `null` value
    Null,
    /// Represents a YAML boolean value
    Bool(bool),
    /// Represents an unparsed string value which may contain reclass
    /// references.
    String(String),
    /// Represents a string literal value which can't contain reclass
    /// references.
    Literal(String),
    /// Represents a YAML number
    Number(Number),
    /// Represents a YAML mapping
    Mapping(Mapping),
    /// Represents a YAML sequence
    Sequence(Sequence),
    /// ValueList represents a list of layered values which may have different
    /// types.  ValueLists are flattened during reference interpolation.
    ValueList(Sequence),
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

impl Default for Value {
    fn default() -> Self {
        Self::Null
    }
}

impl Value {
    #[inline]
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    #[inline]
    pub fn is_bool(&self) -> bool {
        matches!(self, Self::Bool(_))
    }

    #[inline]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    #[inline]
    pub fn is_i64(&self) -> bool {
        match self {
            Self::Number(n) => n.is_i64(),
            _ => false,
        }
    }

    #[inline]
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Number(n) => n.as_i64(),
            _ => None,
        }
    }

    #[inline]
    pub fn is_u64(&self) -> bool {
        match self {
            Self::Number(n) => n.is_u64(),
            _ => false,
        }
    }

    #[inline]
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Self::Number(n) => n.as_u64(),
            _ => None,
        }
    }

    #[inline]
    pub fn is_f64(&self) -> bool {
        match self {
            Self::Number(n) => n.is_f64(),
            _ => false,
        }
    }

    #[inline]
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Number(n) => n.as_f64(),
            _ => None,
        }
    }

    #[inline]
    pub fn is_string(&self) -> bool {
        matches!(self, Self::String(_))
    }

    #[inline]
    pub fn is_literal(&self) -> bool {
        matches!(self, Self::Literal(_))
    }

    #[inline]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            Self::Literal(s) => Some(s),
            _ => None,
        }
    }

    #[inline]
    pub fn is_mapping(&self) -> bool {
        matches!(self, Self::Mapping(_))
    }

    #[inline]
    pub fn as_mapping(&self) -> Option<&Mapping> {
        match self {
            Self::Mapping(m) => Some(m),
            _ => None,
        }
    }

    #[inline]
    pub fn as_mapping_mut(&mut self) -> Option<&mut Mapping> {
        match self {
            Self::Mapping(m) => Some(m),
            _ => None,
        }
    }

    #[inline]
    pub fn is_sequence(&self) -> bool {
        matches!(self, Self::Sequence(_))
    }

    #[inline]
    pub fn as_sequence(&self) -> Option<&Sequence> {
        match self {
            Self::Sequence(s) => Some(s),
            _ => None,
        }
    }

    #[inline]
    pub fn as_sequence_mut(&mut self) -> Option<&mut Sequence> {
        match self {
            Self::Sequence(s) => Some(s),
            _ => None,
        }
    }

    #[inline]
    pub fn is_value_list(&self) -> bool {
        matches!(self, Self::ValueList(_))
    }

    #[inline]
    pub fn as_value_list(&self) -> Option<&Sequence> {
        match self {
            Self::ValueList(l) => Some(l),
            _ => None,
        }
    }

    #[inline]
    pub fn as_value_list_mut(&mut self) -> Option<&mut Sequence> {
        match self {
            Self::ValueList(l) => Some(l),
            _ => None,
        }
    }

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

    #[inline]
    pub fn get_mut(&mut self, k: &Value) -> Option<&mut Value> {
        match self {
            Self::Mapping(m) => m.get_mut(k),
            Self::Sequence(s) | Self::ValueList(s) => {
                if let Some(idx) = k.as_u64() {
                    let idx = idx as usize;
                    if idx < s.len() {
                        return Some(&mut s[idx]);
                    }
                }
                None
            }
            _ => None,
        }
    }

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
