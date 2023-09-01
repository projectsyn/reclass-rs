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

/// Represents a YAML mapping in a form suitable to manage Reclass parameters.
///
/// The map provides support for managing constant keys. Constant keys can't be overwritten
/// anymore, and operations which would try to do so, or would allow users to do so (e.g. `get_mut`
/// and `insert`) will return an Error when called for a key which is marked constant.
///
/// Existing map keys can be marked constant if `insert()` is called with the existing key marked
/// as constant. Keys are marked constant by prefixing them with the constant prefix marker
/// `KeyPrefix::Constant`.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Mapping {
    /// Holds the mapping data.
    map: IndexMap<Value, Value>,
    /// Holds the set of keys in the mapping which are marked as constant.
    const_keys: HashSet<Value>,
}

impl Mapping {
    /// Creates a new mapping.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new mapping with the given initial capacity.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            map: IndexMap::with_capacity(capacity),
            const_keys: HashSet::default(),
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
    }

    /// Removes all data from the mapping.
    #[inline]
    pub fn clear(&mut self) {
        self.map.clear();
        self.const_keys.clear();
    }

    /// Inserts key-value pair in the mapping. If the key already existed, the old value is
    /// returned. If the key didn't exist, None is returned. If the key existed and was marked
    /// constant, an error is returned.
    ///
    /// The function also processes the provided key and marks it as constant if it starts with the
    /// constant key prefix marker (`KeyPrefix::Constant`).
    #[inline]
    pub fn insert(&mut self, k: Value, v: Value) -> Result<Option<Value>> {
        let (n, p) = k.strip_prefix();
        // check if the key (stripped from any prefixes) is marked constant
        if !self.const_keys.contains(&n) || !self.map.contains_key(&n) {
            // either the key isn't marked constant, or isn't present in the map yet.
            match p {
                Some(KeyPrefix::Constant) => {
                    self.const_keys.insert(n.clone());
                    Ok(self.map.insert(n, v))
                }
                // if the key isn't marked constant, insert the original key.
                _ => Ok(self.map.insert(k, v)),
            }
        } else {
            // k is marked constant and already set in the map, return error
            Err(anyhow!(format!(
                "Inserting {:?}={:?}, key already in map and marked constant",
                n, v
            )))
        }
    }

    /// Returns a double-ended iterator visiting all key-value pairs in order of
    /// insertion. Iterator element type is `(&'a Value, &'a Value)`.
    #[inline]
    pub fn iter(&self) -> Iter {
        Iter {
            iter: self.map.iter(),
        }
    }

    /// Returns a reference to the underlying `IndexMap`.
    #[inline]
    pub fn as_map(&self) -> &IndexMap<Value, Value> {
        &self.map
    }

    /// Returns `true` if the mapping contains key `k`.
    #[inline]
    pub fn contains_key(&self, k: &Value) -> bool {
        self.map.contains_key(k)
    }

    /// Returns a reference to the value for key `k` if the key is present in the mapping.
    #[inline]
    pub fn get(&self, k: &Value) -> Option<&Value> {
        self.map.get(k)
    }

    /// Returns a mutable reference to the value for key `k` if the key is present in the mapping.
    /// Returns an error if called for a key which is marked constant.
    #[inline]
    pub fn get_mut(&mut self, k: &Value) -> Result<Option<&mut Value>> {
        if !self.const_keys.contains(k) {
            Ok(self.map.get_mut(k))
        } else {
            Err(anyhow!(format!("Key {:?} is marked constant", k)))
        }
    }

    /// Returns the given key's entry in the map for insertion and/or in-place updates.
    /// Returns an error if called for a key which is marked constant.
    #[inline]
    pub fn entry(&mut self, k: Value) -> Result<indexmap::map::Entry<Value, Value>> {
        if !self.const_keys.contains(&k) {
            Ok(self.map.entry(k))
        } else {
            Err(anyhow!(format!("Key {:?} is marked constant", k)))
        }
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
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Converts the `Mapping` into a `PyDict`.
    pub fn as_py_dict(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new(py);

        for (k, v) in self.iter() {
            let pyk = k.as_py_obj(py)?;
            let pyv = v.as_py_obj(py)?;
            dict.set_item(pyk, pyv)?;
        }

        Ok(dict.into())
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
    /// key occurs for multiple elements, the last value associated with the key wins.
    ///
    /// Note that this function will discard elements in the iterator if the element's key is
    /// already in the map and marked as constant.
    #[inline]
    fn from_iter<I: IntoIterator<Item = (Value, Value)>>(iter: I) -> Self {
        let mut new = Mapping::new();
        for (k, v) in iter {
            if let Err(e) = new.insert(k, v) {
                eprintln!("Error inserting key-value pair: {}", e);
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
        ~baz: baz
        "#;
        let m = Mapping::from_str(input).unwrap();
        let mut expected = Mapping::new();
        expected.insert_raw("foo".into(), "foo".into());
        // Const prefix is consumed and stored in map's list of const keys
        expected.insert_raw("bar".into(), "bar".into());
        expected.const_keys.insert("bar".into());
        // Override prefix is left alone
        expected.insert_raw("~baz".into(), "baz".into());
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
        assert_eq!(m.get(&"foo".into()), Some(&"bar".into()));

        let v = m.insert("foo".into(), "baz".into());
        assert!(v.is_err());
        let v = m.insert("~foo".into(), "baz".into());
        assert!(v.is_err());

        assert_eq!(m.len(), 1);
        assert!(m.contains_key(&"foo".into()));
        assert!(m.const_keys.contains(&"foo".into()));
        assert_eq!(m.get(&"foo".into()), Some(&"bar".into()));
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
        // last foo key wins
        assert_eq!(m.get(&"foo".into()), Some(&"bar".into()));
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
        bar.insert_raw("~foo".into(), "foo".into());

        let mut expected = Mapping::new();
        expected.insert_raw("foo".into(), "foo".into());
        expected.insert_raw("bar".into(), bar.into());
        expected.insert_raw("baz".into(), "baz".into());
        expected.const_keys.insert("baz".into());

        assert_eq!(m, expected);
    }
}
