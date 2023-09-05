mod from;
pub(crate) mod mapping;
mod value;

pub use mapping::Mapping;
pub use value::Value;

/// A YAML sequence in which the elements are `reclass_rs::value::Value`
pub type Sequence = Vec<Value>;

/// Represents special key types in Reclass
#[derive(Debug, Eq, PartialEq)]
enum KeyPrefix {
    /// Represents a key which is marked as constant
    ///
    /// Constant keys can't be overridden any more.
    Constant, // '=',
    /// Represents a key which should be overridden instead of merged
    ///
    /// Keys prefixed with the override marker are taken as the new base value, discarding any
    /// previous content of the key.
    Override, // '~',
}

impl KeyPrefix {
    fn from(c: char) -> Option<Self> {
        match c {
            '=' => Some(Self::Constant),
            '~' => Some(Self::Override),
            _ => None,
        }
    }
}

impl std::fmt::Display for KeyPrefix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Constant => write!(f, "="),
            Self::Override => write!(f, "~"),
        }
    }
}
