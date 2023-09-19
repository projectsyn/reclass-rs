// This implementation is inspired by `serde_yaml::Mapping`

use anyhow::{anyhow, Result};
use indexmap::IndexMap;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};

use super::value::Value;
use super::KeyPrefix;
use crate::refs::ResolveState;

/// Represents a YAML mapping in a form suitable to manage Reclass parameters.
///
/// The map supports keeping track of "value lists" (through `Value::ValueList`) which are
/// essentially lists of layers for a single key produced through Reclass class includes.
///
/// Additionally, The map provides support for managing constant keys and overrides.
///
/// Constant keys can't be overwritten anymore, and operations which would try to do so, or would
/// allow users to do so (e.g. `get_mut` and `insert`) will return an Error when called for a key
/// which is marked constant.
///
/// Existing map keys can be marked constant if `insert()` is called with the existing key marked
/// as constant. Keys are marked constant by prefixing them with the constant prefix marker
/// `KeyPrefix::Constant`.
///
/// Finally, Keys can be marked as overriding. This will cause `insert()` to drop any existing
/// value for the key instead of tracking the old values as a `Value::ValueList`.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Mapping {
    /// Holds the mapping data.
    map: IndexMap<Value, Value>,
    /// Holds the set of keys in the mapping which are marked as constant. Key constantness is
    /// propagated in [`Mapping::merge()`].
    const_keys: HashSet<Value>,
    /// Holds the set of keys in the mapping which were marked as override, but for which no
    /// previous value was overridden during insertion. We process overrides for such keys during
    /// the next call to [`Mapping::merge()`] where the contents of this map are merged into
    /// another map, i.e. a call to `merge()` where this map is `other`.
    override_keys: HashSet<Value>,
}

impl std::fmt::Display for Mapping {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{")?;
        for (i, (k, v)) in self.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{k}: {v}")?;
        }
        write!(f, "}}")
    }
}

impl Mapping {
    /// Creates a new mapping.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new mapping with the given initial capacity.
    #[inline]
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            map: IndexMap::with_capacity(capacity),
            const_keys: HashSet::default(),
            override_keys: HashSet::default(),
        }
    }

    /// Reserves capacity for at least `additional` more elements.
    ///
    /// # Panics
    ///
    /// Panics if the new allocation size overflows `usize`.
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.map.reserve(additional);
    }

    /// Shrinks the map's capacity as much as possible to fit the current contents.
    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.map.shrink_to_fit();
        self.const_keys.shrink_to_fit();
        self.override_keys.shrink_to_fit();
    }

    /// Removes all data from the mapping.
    #[inline]
    pub fn clear(&mut self) {
        self.map.clear();
        self.const_keys.clear();
        self.override_keys.clear();
    }

    /// Inserts key-value pair in the mapping.
    ///
    /// Any prefixes (`KeyPrefix` variants) are removed from the key before it's used to determine
    /// whether a value already exists, or whether the key is marked as constant in the map.
    ///
    /// If the provided key already exists in the map and is already marked as constant, the
    /// function returns an error.
    ///
    /// The function marks the key as constant in the map if it starts with the constant key prefix
    /// marker (`KeyPrefix::Constant`).
    ///
    /// If the key isn't marked as overriding (with prefix `KeyPrefix::Override`) and it already
    /// exists in the map, the new value is appended to the existing value(s) in a
    /// `Value::ValueList`. If necessary, a new `ValueList` is created from the old and the new
    /// value. If the provided value is a `ValueList` itself, it's consumed and appended to the
    /// existing `ValueList`.
    ///
    /// If the key is marked as overriding, the existing value is replaced with the new value, and
    /// the old Value is returned.
    ///
    /// Note that keys can't be marked constant and overriding. If a key has both markers, the
    /// marker which is the first character of the key will be processed, and the other marker will
    /// be treated as part of the actual key.
    #[inline]
    pub fn insert(&mut self, k: Value, v: Value) -> Result<Option<Value>> {
        self.insert_impl(k, v, false, false)
    }

    /// Inserts key-value pair in the mapping.
    ///
    /// See [`Mapping::insert()`] for the full semantics of insertion.
    ///
    /// In contrast to `Mapping::insert()` this method allows callers to force `k` to become
    /// constant or be marked as overriding through the `force_const` and `force_override` flags
    /// respectively.
    #[inline]
    fn insert_impl(
        &mut self,
        k: Value,
        v: Value,
        force_const: bool,
        force_override: bool,
    ) -> Result<Option<Value>> {
        let (k, p) = k.strip_prefix();
        if !self.map.contains_key(&k) {
            // key isn't present in the map, insert it as base value
            match p {
                Some(KeyPrefix::Constant) => {
                    // mark key as constant if it has the constant prefix
                    self.const_keys.insert(k.clone());
                }
                Some(KeyPrefix::Override) => {
                    // remember that `k` was marked as overriding if we don't have a value to
                    // override in this map.
                    self.override_keys.insert(k.clone());
                }
                None => {}
            };
            if force_const {
                self.const_keys.insert(k.clone());
            }
            if force_override {
                self.override_keys.insert(k.clone());
            }
            Ok(self.map.insert(k, v))
        } else if self.const_keys.contains(&k) {
            // k is marked constant and already set in the map, return error
            Err(anyhow!("Can't overwrite constant key {k}"))
        } else {
            // here: we know the key is present, and not yet marked constant

            // Return None if we append the new value to a ValueList
            let mut res = None;

            if force_override || matches!(p, Some(KeyPrefix::Override)) {
                // Replace the current value of `k` with the new value and remember the old value
                // to be returned by the function.
                // NOTE(sg): If we immediately process the override here, we don't need to update
                // `override_keys`.
                res = self.map.insert(k.clone(), v);
            } else {
                // Append the new value to the ValueList for k

                // Create new ValueList for `k` if necessary, and return the old value for `k` if
                // we had to create a ValueList
                let oldv = if self.map.get(&k).unwrap().is_value_list() {
                    // Store `None` in `oldv`, if k is already a ValueList.
                    None
                } else {
                    // If k isn't a ValueList yet, replace current value in map with an empty
                    // ValueList, and store the old value in `oldv`.
                    self.map.insert(k.clone(), Value::ValueList(vec![]))
                };

                // Get a mutable reference to the underlying Vec<Value> of the ValueList for `k`.
                // At this point, we know that `k`'s value must be a ValueList, since we just
                // created a ValueList, if the old value wasn't a ValueList already.
                let elems = self.map.get_mut(&k).unwrap().as_value_list_mut().unwrap();

                if let Some(oldv) = oldv {
                    // If we created a new ValueList `oldv` holds the old value of `k`. We need to
                    // insert that value into the ValueList before adding the new value. We know
                    // the old value can't be a ValueList, so we can unconditionally add it as a
                    // single element.
                    elems.push(oldv);
                }

                // Append value(s) to insert to our ValueList
                if let Value::ValueList(l) = v {
                    elems.extend(l);
                } else {
                    elems.push(v);
                }
            }

            // mark key as constant if it has the constant prefix
            if force_const || matches!(p, Some(KeyPrefix::Constant)) {
                self.const_keys.insert(k.clone());
            }

            // Return old value if we replaced it due to an override key
            Ok(res)
        }
    }

    /// Returns a double-ended iterator visiting all key-value pairs in order of
    /// insertion. Iterator element type is `(&'a Value, &'a Value)`.
    #[inline]
    #[must_use]
    pub fn iter(&self) -> Iter {
        Iter {
            iter: self.map.iter(),
        }
    }

    /// Returns a reference to the underlying `IndexMap`.
    #[inline]
    #[must_use]
    pub fn as_map(&self) -> &IndexMap<Value, Value> {
        &self.map
    }

    /// Returns `true` if the mapping contains key `k`.
    #[inline]
    #[must_use]
    pub fn contains_key(&self, k: &Value) -> bool {
        self.map.contains_key(k)
    }

    /// Returns a reference to the value for key `k` if the key is present in the mapping.
    #[inline]
    #[must_use]
    pub fn get(&self, k: &Value) -> Option<&Value> {
        self.map.get(k)
    }

    /// Returns a mutable reference to the value for key `k` if the key is present in the mapping.
    /// Returns an error if called for a key which is marked constant.
    #[inline]
    pub fn get_mut(&mut self, k: &Value) -> Result<Option<&mut Value>> {
        if self.const_keys.contains(k) {
            return Err(anyhow!("Key {k} is marked constant"));
        }
        Ok(self.map.get_mut(k))
    }

    /// Returns the given key's entry in the map for insertion and/or in-place updates.
    /// Returns an error if called for a key which is marked constant.
    #[inline]
    pub fn entry(&mut self, k: Value) -> Result<indexmap::map::Entry<Value, Value>> {
        if self.const_keys.contains(&k) {
            return Err(anyhow!("Key {k} is marked constant"));
        }
        Ok(self.map.entry(k))
    }

    /// Removes the entry for key `k` from the map and returns its value if the key was present in
    /// the map. Additionally, removes the key from the list of constant keys if it was marked
    /// constant.
    #[inline]
    pub fn remove(&mut self, k: &Value) -> Option<Value> {
        self.const_keys.remove(k);
        self.map.remove(k)
    }

    /// Removes and returns the key-value pair for `k` if the key is present in the map.
    #[inline]
    pub fn remove_entry(&mut self, k: &Value) -> Option<(Value, Value)> {
        self.const_keys.remove(k);
        self.map.remove_entry(k)
    }

    /// Returns the number of key-value pairs in the map.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Checks if the map is empty
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.map.len() == 0
    }

    /// Converts the `Mapping` into a `PyDict`.
    pub fn as_py_dict(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new(py);

        for (k, v) in self {
            let pyk = k.as_py_obj(py)?;
            let pyv = v.as_py_obj(py)?;
            dict.set_item(pyk, pyv)?;
        }

        Ok(dict.into())
    }

    /// Checks if the provided key is marked as constant.
    #[inline]
    #[must_use]
    fn is_const(&self, k: &Value) -> bool {
        self.const_keys.contains(k)
    }

    /// Checks if the provided key is marked as overriding.
    #[inline]
    #[must_use]
    fn is_override(&self, k: &Value) -> bool {
        self.override_keys.contains(k)
    }

    /// Merges Mapping `other` into this mapping.
    ///
    /// The function parses each key present in `other`
    ///
    /// This function will update the current map's constant key set with any keys that are marked
    /// as constant in the `other` map.
    pub fn merge(&mut self, other: &Self) -> Result<()> {
        for (k, v) in other {
            // ValueList merging is implemented in insert_impl
            self.insert_impl(
                k.clone(),
                v.clone(),
                other.is_const(k),
                other.is_override(k),
            )?;
        }
        Ok(())
    }

    /// Returns a new Mapping with all values flattened while preserving const and override key
    /// information.
    ///
    /// Used in `Value::flattened()` to preserve const and override key information when flattening
    /// Mapping values.
    pub(super) fn flattened(&self) -> Result<Self> {
        let mut res = Self::new();
        for (k, v) in self {
            // Propagate key properties to the resulting mapping by using `insert_impl()`.
            res.insert_impl(
                k.clone(),
                v.flattened()?,
                self.is_const(k),
                self.is_override(k),
            )?;
        }
        Ok(res)
    }

    /// Returns a new Mapping with any Reclass references in the mapping interpolated while
    /// preserving const and override key information.
    ///
    /// The method looks up reference values in parameter `root`. After interpolation of each
    /// Mapping key-value pair, the resulting value is flattened before it's inserted in the new
    /// Mapping. Mapping keys are inserted into the new mapping unchanged.
    pub(super) fn interpolate(&self, root: &Self, state: &mut ResolveState) -> Result<Self> {
        let mut res = Self::new();
        for (k, v) in self {
            // Reference loops in mappings can't be stretched across key-value pairs, so we pass a
            // copy of the resolution state we're called with to the `interpolate` call for each
            // value. Also, we don't need to update the state which we were called with, since we
            // either manage to interpolate a value (in which case it doesn't contain a loop) or we
            // don't and the whole interpolation is aborted.
            let mut st = state.clone();
            let mut v = v.interpolate(root, &mut st)?;
            v.flatten()?;
            // Propagate key properties to the resulting mapping by using `insert_impl()`.
            res.insert_impl(k.clone(), v, self.is_const(k), self.is_override(k))?;
        }
        Ok(res)
    }
}

impl From<serde_yaml::Mapping> for Mapping {
    /// Converts a `serde_yaml::Mapping` into a `Mapping`.
    fn from(m: serde_yaml::Mapping) -> Self {
        let mut new = Self::with_capacity(m.len());
        for (k, v) in m {
            // we can't have duplicate const keys when converting from a serde_yaml Mapping, so we
            // can safely unwrap the Result.
            new.insert(Value::from(k), Value::from(v)).unwrap();
        }
        new
    }
}

impl From<Mapping> for serde_yaml::Mapping {
    /// Converts a `Mapping` into a `serde_yaml::Mapping`.
    ///
    /// Note that information about constant keys is lost here.
    fn from(m: Mapping) -> Self {
        let mut new = Self::with_capacity(m.map.len());
        for (k, v) in m.map {
            new.insert(serde_yaml::Value::from(k), serde_yaml::Value::from(v));
        }
        new
    }
}

impl From<Mapping> for serde_json::Map<String, serde_json::Value> {
    fn from(m: Mapping) -> Self {
        let mut new = Self::with_capacity(m.map.len());
        for (k, v) in m.map {
            // JSON keys must be strings, we convert some Value variants to string here
            let k = match k {
                Value::String(s) | Value::Literal(s) => s,
                Value::Bool(b) => format!("{b}"),
                Value::Number(n) => format!("{n}"),
                Value::Null => "null".to_owned(),
                _ => panic!("Can't serialize {} as JSON key", v.variant()),
            };
            new.insert(k, serde_json::Value::from(v));
        }

        new
    }
}

impl std::str::FromStr for Mapping {
    type Err = anyhow::Error;

    /// Converts a `&str` into a `Mapping`.
    ///
    /// This function returns an error if the given string can't be parsed as YAML by
    /// `serde_yaml::from_str`.
    #[inline]
    fn from_str(s: &str) -> Result<Self> {
        let m = serde_yaml::from_str::<serde_yaml::Mapping>(s)?;
        Ok(Self::from(m))
    }
}

impl FromIterator<(Value, Value)> for Mapping {
    /// Creates a `Mapping` from an Iterator over `(Value, Value)`.
    ///
    /// New elements are inserted in the order in which they appear in the iterator. If the same
    /// key occurs for multiple elements, the resulting map will contain a `ValueList` fo that key.
    ///
    /// If a key is marked as overriding, any previously provided values for that key are dropped.
    ///
    /// If multiple elements in the iterator try to set the same key, and one element marks the key
    /// as constant, an elements later in the iterator which try to set that key are skipped and a
    /// diagnostic message is printed.
    #[inline]
    fn from_iter<I: IntoIterator<Item = (Value, Value)>>(iter: I) -> Self {
        let mut new = Mapping::new();
        for (k, v) in iter {
            if let Err(e) = new.insert(k, v) {
                eprintln!("Error inserting key-value pair: {e}");
            }
        }
        new
    }
}

/// Iterator over `Mapping`.
pub struct Iter<'a> {
    iter: indexmap::map::Iter<'a, Value, Value>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = (&'a Value, &'a Value);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'a> ExactSizeIterator for Iter<'a> {
    #[inline]
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<'a> IntoIterator for &'a Mapping {
    type Item = (&'a Value, &'a Value);
    type IntoIter = Iter<'a>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        Iter {
            iter: self.map.iter(),
        }
    }
}

impl Hash for Mapping {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash the kv pairs in a way that is not sensitive to their order.
        // NOTE(sg): Implementation idea copied from serde_yaml's Mapping implementation
        let mut xor = 0;
        for (k, v) in self {
            let mut hasher = DefaultHasher::new();
            k.hash(&mut hasher);
            v.hash(&mut hasher);
            xor ^= hasher.finish();
        }
        xor.hash(state);
    }
}

#[cfg(test)]
mod mapping_tests {
    use super::*;
    use std::str::FromStr;

    impl Mapping {
        // we don't care about const_keys for most of the tests, so we use this method instead of
        // insert() so we don't have to deal with the Result value.
        fn insert_raw(&mut self, k: Value, v: Value) -> Option<Value> {
            self.map.insert(k, v)
        }
    }

    fn create_map() -> Mapping {
        let mut m = Mapping::new();
        m.insert_raw("a".into(), 1.into());
        m.insert_raw("b".into(), "foo".into());
        m.insert_raw("c".into(), 3.14.into());
        m.insert_raw("d".into(), Value::Bool(true));
        m
    }

    #[test]
    fn test_from_str() {
        let input = r#"
        a: 1
        b: foo
        c: 3.14
        d: true
        e: [1,2,3]
        f:
          foo: bar
        "#;
        let m = Mapping::from_str(input).unwrap();
        let mut expected = create_map();
        expected.insert_raw("e".into(), vec![1, 2, 3].into());
        expected.insert_raw(
            "f".into(),
            Mapping::from_iter(vec![("foo".into(), "bar".into())]).into(),
        );
        assert_eq!(m, expected);
    }

    #[test]
    fn test_iter() {
        let m = create_map();

        let items = m.iter().collect::<Vec<(&Value, &Value)>>();
        assert_eq!(items.len(), 4);
        assert_eq!(items[0], (&"a".into(), &1.into()));
        assert_eq!(items[1], (&"b".into(), &"foo".into()));
        assert_eq!(items[2], (&"c".into(), &3.14.into()));
        assert_eq!(items[3], (&"d".into(), &Value::Bool(true)));
    }

    #[test]
    fn test_contains_key() {
        let mut m = create_map();
        m.insert_raw(3.14.into(), "3.14".into());

        assert!(m.contains_key(&"a".into()));
        assert!(m.contains_key(&3.14.into()));
        assert!(!m.contains_key(&"e".into()));
        assert!(!m.contains_key(&5.into()));
    }

    #[test]
    fn test_get() {
        let mut m = create_map();
        m.insert_raw(3.14.into(), "3.14".into());

        assert_eq!(m.get(&"a".into()), Some(&1.into()));
        assert_eq!(m.get(&3.14.into()), Some(&"3.14".into()));
        assert_eq!(m.get(&"e".into()), None);
    }

    #[test]
    fn test_get_mut() {
        let mut m = create_map();

        assert_eq!(m.get(&"a".into()), Some(&1.into()));
        let e = m.get_mut(&"a".into()).unwrap();
        assert!(e.is_some());
        let e = e.unwrap();
        *e = 2.into();
        assert_eq!(m.get(&"a".into()), Some(&2.into()));
    }

    #[test]
    fn test_get_mut_const_key() {
        let mut m = create_map();
        m.const_keys.insert("a".into());

        assert_eq!(m.get(&"a".into()), Some(&1.into()));
        assert!(m.get_mut(&"a".into()).is_err());
    }

    #[test]
    fn test_entry_existing() {
        let mut m = create_map();

        assert_eq!(m.get(&"a".into()), Some(&1.into()));
        m.entry("a".into())
            .unwrap()
            .and_modify(|e| *e = 3.into())
            .or_insert(2.into());

        assert_eq!(m.get(&"a".into()), Some(&3.into()));
    }

    #[test]
    fn test_entry_new() {
        let mut m = create_map();

        assert_eq!(m.get(&"e".into()), None);
        m.entry("e".into())
            .unwrap()
            .and_modify(|e| *e = 3.into())
            .or_insert(2.into());

        assert_eq!(m.get(&"e".into()), Some(&2.into()));
    }

    #[test]
    fn test_entry_error_const_key() {
        let mut m = Mapping::new();
        let _v = m.insert("=foo".into(), "foo".into());
        assert!(m.entry("foo".into()).is_err());
    }

    #[test]
    fn test_remove() {
        let mut m = create_map();

        let e = m.remove(&"a".into());
        assert_eq!(e, Some(1.into()));
        assert!(!m.contains_key(&"a".into()));

        let e = m.remove(&"a".into());
        assert_eq!(e, None);
    }

    #[test]
    fn test_remove_entry() {
        let mut m = create_map();

        let e = m.remove_entry(&"a".into());
        assert_eq!(e, Some(("a".into(), 1.into())));
        assert!(!m.contains_key(&"a".into()));
    }

    #[test]
    fn test_as_py_dict() {
        let mut m = create_map();
        m.insert_raw("e".into(), vec![1, 2, 3].into());
        m.insert_raw(
            "f".into(),
            Mapping::from_iter(vec![("foo".into(), "bar".into())]).into(),
        );
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let pym = m.as_py_dict(py).unwrap();
            let m = pym.as_ref(py);
            assert_eq!(m.len(), 6);
            assert_eq!(format!("{:?}", m.keys()), "['a', 'b', 'c', 'd', 'e', 'f']");
            let a = m.get_item(&"a").unwrap();
            assert!(a.is_instance_of::<pyo3::types::PyInt>());
            assert!(a
                .downcast_exact::<pyo3::types::PyInt>()
                .unwrap()
                .eq(1.into_py(py))
                .unwrap());
            let f = m.get_item(&"f").unwrap();
            assert!(f.is_instance_of::<PyDict>());
            let f = f.downcast_exact::<PyDict>().unwrap();
            assert_eq!(f.len(), 1);
            assert_eq!(format!("{:?}", f.keys()), "['foo']");
            assert_eq!(format!("{:?}", f.values()), "['bar']");
            // Remaining element checks omitted, since we have tests for all Value variants in
            // value.rs.
        });
    }

    #[test]
    fn test_iter_len() {
        let m = create_map();
        assert_eq!(m.iter().len(), 4);
    }

    #[test]
    fn test_into_iter() {
        let m = create_map();

        let mut items = vec![];
        for it in &m {
            items.push(it)
        }

        assert_eq!(
            items,
            vec![
                (&"a".into(), &1.into()),
                (&"b".into(), &"foo".into()),
                (&"c".into(), &3.14.into()),
                (&"d".into(), &Value::Bool(true)),
            ]
        );
    }

    #[test]
    fn test_from_str_const_keys() {
        let input = r#"
        foo: foo
        =bar: bar
        baz: baz
        "#;
        let m = Mapping::from_str(input).unwrap();
        let mut expected = Mapping::new();
        expected.insert_raw("foo".into(), "foo".into());
        // Const prefix is consumed and stored in map's list of const keys
        expected.insert_raw("bar".into(), "bar".into());
        expected.const_keys.insert("bar".into());
        // Override prefix is consumed
        expected.insert_raw("baz".into(), "baz".into());
        assert_eq!(m, expected);
    }

    #[test]
    fn test_insert_const_key() {
        let mut m = Mapping::new();
        m.insert("=foo".into(), "foo".into()).unwrap();
        assert_eq!(m.len(), 1);
        assert_eq!(m.const_keys.len(), 1);
        assert!(m.contains_key(&"foo".into()));
        assert!(!m.contains_key(&"=foo".into()));
        assert!(m.const_keys.contains(&"foo".into()));
        assert_eq!(m.get(&"foo".into()), Some(&"foo".into()));
        assert_eq!(m.get(&"=foo".into()), None);
    }

    #[test]
    fn test_overwrite_const_key() {
        let mut m = Mapping::new();
        m.insert("=foo".into(), "foo".into()).unwrap();
        let v = m.insert("=foo".into(), "bar".into());
        assert!(v.is_err());
        assert!(m.contains_key(&"foo".into()));
        assert!(m.const_keys.contains(&"foo".into()));
        assert_eq!(m.get(&"foo".into()), Some(&"foo".into()));
    }

    #[test]
    fn test_overwrite_key_make_const() {
        let mut m = Mapping::new();
        m.insert("foo".into(), "foo".into()).unwrap();
        assert!(m.const_keys.is_empty());

        let v = m.insert("=foo".into(), "bar".into());
        assert!(v.is_ok());
        assert_eq!(m.len(), 1);
        assert!(m.contains_key(&"foo".into()));
        assert!(m.const_keys.contains(&"foo".into()));
        assert_eq!(
            m.get(&"foo".into()),
            Some(&Value::ValueList(vec!["foo".into(), "bar".into()]))
        );

        let v = m.insert("foo".into(), "baz".into());
        assert!(v.is_err());
        let v = m.insert("~foo".into(), "baz".into());
        assert!(v.is_err());

        assert_eq!(m.len(), 1);
        assert!(m.contains_key(&"foo".into()));
        assert!(m.const_keys.contains(&"foo".into()));
        assert_eq!(
            m.get(&"foo".into()),
            Some(&Value::ValueList(vec!["foo".into(), "bar".into()]))
        );
    }

    #[test]
    fn test_overwrite_key_override() {
        let mut m = Mapping::new();
        m.insert("foo".into(), "foo".into()).unwrap();
        assert!(m.const_keys.is_empty());

        let v = m.insert("~foo".into(), "bar".into());
        assert!(v.is_ok());
        assert_eq!(m, Mapping::from_str("foo: bar").unwrap());
    }

    #[test]
    fn test_from_iter_duplicate_const() {
        let items = vec![("=foo".into(), "foo".into()), ("=foo".into(), "bar".into())];
        let m = Mapping::from_iter(items);
        assert_eq!(m.len(), 1);
        assert_eq!(m.get(&"foo".into()), Some(&"foo".into()));
    }

    #[test]
    fn test_from_iter_duplicate_key() {
        let items = vec![("foo".into(), "foo".into()), ("foo".into(), "bar".into())];
        let m = Mapping::from_iter(items);
        assert_eq!(m.len(), 1);
        // duplicate values for single key are stored in valuelist
        assert_eq!(
            m.get(&"foo".into()),
            Some(&Value::ValueList(vec!["foo".into(), "bar".into()]))
        );
    }

    #[test]
    fn test_from_iter_override_key() {
        let items = vec![("foo".into(), "foo".into()), ("~foo".into(), "bar".into())];
        let m = Mapping::from_iter(items);
        assert_eq!(m.len(), 1);
        assert_eq!(m, Mapping::from_str("foo: bar").unwrap());
    }

    #[test]
    fn test_insert_remove_insert_const_key() {
        let mut m = Mapping::new();
        // Initialize map with constant key foo and regular key bar
        m.insert("=foo".into(), "foo".into()).unwrap();
        m.insert("bar".into(), "bar".into()).unwrap();

        assert_eq!(m.len(), 2);
        assert_eq!(m.const_keys.len(), 1);

        // Remove constant key foo
        let v = m.remove(&"foo".into());
        assert_eq!(v, Some("foo".into()));

        assert_eq!(m.len(), 1);
        assert_eq!(m.const_keys.len(), 0);

        // Insert foo again
        m.insert("foo".into(), "baz".into()).unwrap();

        assert_eq!(m.len(), 2);
        assert_eq!(m.const_keys.len(), 0);
        assert_eq!(m.get(&"foo".into()), Some(&"baz".into()));
    }

    #[test]
    fn test_from_serde_yaml_const_keys() {
        let input = r#"
        foo: foo
        bar:
          =qux: qux
          ~foo: foo
        =baz: baz
        "#;
        let rawm: serde_yaml::Mapping = serde_yaml::from_str(input).unwrap();

        let m = Mapping::from(rawm);

        let mut bar = Mapping::new();
        bar.insert_raw("qux".into(), "qux".into());
        bar.const_keys.insert("qux".into());
        bar.insert_raw("foo".into(), "foo".into());
        bar.override_keys.insert("foo".into());

        let mut expected = Mapping::new();
        expected.insert_raw("foo".into(), "foo".into());
        expected.insert_raw("bar".into(), bar.into());
        expected.insert_raw("baz".into(), "baz".into());
        expected.const_keys.insert("baz".into());

        assert_eq!(m, expected);
    }

    #[test]
    fn test_mapping_merge_simple() {
        let base = r#"
        foo: foo
        "#;
        let mut base = Mapping::from_str(base).unwrap();
        let m = r#"
        bar: bar
        "#;
        let m = Mapping::from_str(m).unwrap();

        base.merge(&m).unwrap();

        let expected = r#"
        foo: foo
        bar: bar
        "#;
        let e = Mapping::from_str(expected).unwrap();
        assert_eq!(base, e);
    }

    #[test]
    fn test_mapping_merge() {
        let mut base = Mapping::from_str("foo: foo").unwrap();
        let m = Mapping::from_str("foo: bar").unwrap();

        base.merge(&m).unwrap();

        let mut expected = Mapping::new();
        expected.insert_raw(
            "foo".into(),
            Value::ValueList(vec!["foo".into(), "bar".into()]),
        );
        assert_eq!(base, expected);
    }

    #[test]
    fn test_mapping_merge_nested() {
        let base = r#"
        foo:
          foo: foo
        bar:
          bar: bar
        "#;
        let mut base = Mapping::from_str(base).unwrap();
        let m = r#"
        foo:
          baz: baz
        bar:
          qux: qux
        "#;
        let m = Mapping::from_str(m).unwrap();

        base.merge(&m).unwrap();

        let mut expected = Mapping::new();
        expected.insert_raw(
            "foo".into(),
            Value::ValueList(vec![
                Mapping::from_str("foo: foo").unwrap().into(),
                Mapping::from_str("baz: baz").unwrap().into(),
            ]),
        );
        expected.insert_raw(
            "bar".into(),
            Value::ValueList(vec![
                Mapping::from_str("bar: bar").unwrap().into(),
                Mapping::from_str("qux: qux").unwrap().into(),
            ]),
        );

        assert_eq!(base, expected);
    }

    #[test]
    fn test_mapping_merge_const() {
        let mut base = Mapping::from_str("foo: foo").unwrap();
        let m = Mapping::from_str("=foo: bar").unwrap();

        base.merge(&m).unwrap();

        let mut expected = Mapping::new();
        expected.insert_raw(
            "foo".into(),
            Value::ValueList(vec!["foo".into(), "bar".into()]),
        );
        expected.const_keys.insert("foo".into());
        assert_eq!(base, expected);
    }

    #[test]
    fn test_mapping_merge_override() {
        let mut base = Mapping::from_str("foo: foo").unwrap();
        let m = Mapping::from_str("~foo: bar").unwrap();

        base.merge(&m).unwrap();

        assert_eq!(base, Mapping::from_str("foo: bar").unwrap());
    }

    #[test]
    fn test_mapping_merge_override_override() {
        let mut base = Mapping::from_str("foo: foo").unwrap();
        let mut m1 = Mapping::from_str("~foo: bar").unwrap();
        // override prefixes are processed for each merge step. Keys with multiple override
        // prefixes will end up triggering an override multiple times.
        let m2 = Mapping::from_str("~~foo: baz").unwrap();

        // here we consume the first override marker of `~~foo` in m2 which results in `~foo: baz`
        // in the merged m1
        m1.merge(&m2).unwrap();
        assert_eq!(m1, Mapping::from_str("~foo: baz").unwrap());

        // here we consume the remaining override marker of `~foo` in the merged m1 which results
        // in `foo: baz` in the merged base
        base.merge(&m1).unwrap();
        assert_eq!(base, Mapping::from_str("foo: baz").unwrap());
    }

    #[test]
    fn test_mapping_merge_const_override() {
        let mut base = Mapping::from_str("foo: foo").unwrap();
        // The initial parsing sees a constant key `~foo`
        let m1 = Mapping::from_str("=~foo: bar").unwrap();

        // The merge sees overriding key `foo`, and propagates the previously stored constantness
        // of `~foo`. In the end we have mapping `{foo: bar}` where `foo` is marked constant.
        base.merge(&m1).unwrap();
        assert_eq!(base, Mapping::from_str("=foo: bar").unwrap());
    }

    #[test]
    fn test_mapping_merge_override_const() {
        let mut base = Mapping::from_str("foo: foo").unwrap();
        // The initial parsing sees an overriding key `=foo`
        let m1 = Mapping::from_str("~=foo: bar").unwrap();

        // The merge sees a constant key `foo` which has the overriding flag set. In the end we
        // have mapping `{foo: bar}` where `foo` is marked constant.
        base.merge(&m1).unwrap();
        assert_eq!(base, Mapping::from_str("=foo: bar").unwrap());
    }
}
