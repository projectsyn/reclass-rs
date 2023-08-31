// This implementation is inspired by `serde_yaml::Mapping`

use anyhow::Result;
use indexmap::IndexMap;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use super::value::Value;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Mapping {
    map: IndexMap<Value, Value>,
    const_keys: Vec<Value>,
}

impl Mapping {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            map: IndexMap::with_capacity(capacity),
            const_keys: vec![],
        }
    }

    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.map.reserve(additional);
    }

    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.map.shrink_to_fit();
        self.const_keys.shrink_to_fit();
    }

    #[inline]
    pub fn clear(&mut self) {
        self.map.clear();
        self.const_keys.clear();
    }

    #[inline]
    pub fn insert(&mut self, k: Value, v: Value) -> Option<Value> {
        self.map.insert(k, v)
    }

    /// Returns a double-ended iterator visiting all key-value pairs in order of
    /// insertion. Iterator element type is `(&'a Value, &'a Value)`.
    #[inline]
    pub fn iter(&self) -> Iter {
        Iter {
            iter: self.map.iter(),
        }
    }

    #[inline]
    pub fn as_map(&self) -> &IndexMap<Value, Value> {
        &self.map
    }

    #[inline]
    pub fn as_map_mut(&mut self) -> &mut IndexMap<Value, Value> {
        &mut self.map
    }

    #[inline]
    pub fn contains_key(&self, k: &Value) -> bool {
        self.map.contains_key(k)
    }

    #[inline]
    pub fn get(&self, k: &Value) -> Option<&Value> {
        self.map.get(k)
    }

    #[inline]
    pub fn get_mut(&mut self, k: &Value) -> Option<&mut Value> {
        self.map.get_mut(k)
    }

    #[inline]
    pub fn entry(&mut self, k: Value) -> indexmap::map::Entry<Value, Value> {
        self.map.entry(k)
    }

    #[inline]
    pub fn remove(&mut self, k: &Value) -> Option<Value> {
        self.map.remove(k)
    }

    #[inline]
    pub fn remove_entry(&mut self, k: &Value) -> Option<(Value, Value)> {
        self.map.remove_entry(k)
    }

    /// Returns the number of key-value pairs in the map
    #[inline]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Converts the `Mapping` into a `PyDict`
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
    fn from(m: serde_yaml::Mapping) -> Self {
        let mut new = Self::with_capacity(m.len());
        for (k, v) in m {
            new.insert(Value::from(k), Value::from(v));
        }
        new
    }
}

impl From<Mapping> for serde_yaml::Mapping {
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

    #[inline]
    fn from_str(s: &str) -> Result<Self> {
        let m = serde_yaml::from_str::<serde_yaml::Mapping>(s)?;
        Ok(Self::from(m))
    }
}

impl FromIterator<(Value, Value)> for Mapping {
    // TODO(sg): handle const keys here
    #[inline]
    fn from_iter<I: IntoIterator<Item = (Value, Value)>>(iter: I) -> Self {
        Mapping {
            map: IndexMap::from_iter(iter),
            const_keys: vec![],
        }
    }
}

/// Iterator over `&reclass_rs::types::mapping::Mapping`.
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

    fn create_map() -> Mapping {
        let mut m = Mapping::new();
        m.insert("a".into(), 1.into());
        m.insert("b".into(), "foo".into());
        m.insert("c".into(), 3.14.into());
        m.insert("d".into(), Value::Bool(true));
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
        expected.insert("e".into(), vec![1, 2, 3].into());
        expected.insert(
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
        m.insert(3.14.into(), "3.14".into());

        assert!(m.contains_key(&"a".into()));
        assert!(m.contains_key(&3.14.into()));
        assert!(!m.contains_key(&"e".into()));
        assert!(!m.contains_key(&5.into()));
    }

    #[test]
    fn test_get() {
        let mut m = create_map();
        m.insert(3.14.into(), "3.14".into());

        assert_eq!(m.get(&"a".into()), Some(&1.into()));
        assert_eq!(m.get(&3.14.into()), Some(&"3.14".into()));
        assert_eq!(m.get(&"e".into()), None);
    }

    #[test]
    fn test_get_mut() {
        let mut m = create_map();

        assert_eq!(m.get(&"a".into()), Some(&1.into()));
        let e = m.get_mut(&"a".into());
        assert!(e.is_some());
        let e = e.unwrap();
        *e = 2.into();
        assert_eq!(m.get(&"a".into()), Some(&2.into()));
    }

    #[test]
    fn test_entry_existing() {
        let mut m = create_map();

        assert_eq!(m.get(&"a".into()), Some(&1.into()));
        m.entry("a".into())
            .and_modify(|e| *e = 3.into())
            .or_insert(2.into());

        assert_eq!(m.get(&"a".into()), Some(&3.into()));
    }

    #[test]
    fn test_entry_new() {
        let mut m = create_map();

        assert_eq!(m.get(&"e".into()), None);
        m.entry("e".into())
            .and_modify(|e| *e = 3.into())
            .or_insert(2.into());

        assert_eq!(m.get(&"e".into()), Some(&2.into()));
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
        m.insert("e".into(), vec![1, 2, 3].into());
        m.insert(
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
}
