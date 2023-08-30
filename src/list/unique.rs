use serde::Deserialize;

use super::{item_pos, List};

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(from = "Vec<String>")]
pub struct UniqueList {
    items: Vec<String>,
}

impl UniqueList {
    pub fn items_iter(&self) -> impl Iterator<Item = &String> {
        self.items.iter()
    }

    #[cfg(test)]
    pub fn get_items(&self) -> &Vec<String> {
        &self.items
    }

    fn merge_impl(&mut self, itemiter: impl Iterator<Item = String>) {
        for it in itemiter {
            self.append_if_new(it);
        }
    }
}

impl From<Vec<String>> for UniqueList {
    #[inline]
    fn from(item: Vec<String>) -> Self {
        let mut res = Self { items: vec![] };
        for it in item {
            res.append_if_new(it);
        }
        res
    }
}

impl From<UniqueList> for Vec<String> {
    #[inline]
    fn from(l: UniqueList) -> Self {
        l.items
    }
}

impl List for UniqueList {
    #[inline]
    fn new() -> Self {
        Self::default()
    }

    #[inline]
    fn with_capacity(capacity: usize) -> Self {
        Self {
            items: Vec::with_capacity(capacity),
        }
    }

    #[inline]
    fn len(&self) -> usize {
        self.items.len()
    }

    #[inline]
    fn shrink_to_fit(&mut self) {
        self.items.shrink_to_fit();
    }

    /// Appends item to list if it's not present yet
    fn append_if_new(&mut self, item: String) {
        if item_pos(&self.items, &item).is_none() {
            self.items.push(item);
        }
    }

    /// Merges other into self, consuming other
    fn merge(&mut self, other: Self) {
        self.merge_impl(other.items.into_iter());
    }

    /// Merges other into self, creating a clone of other
    fn merge_from(&mut self, other: &Self) {
        self.merge_impl(other.items.iter().cloned());
    }
}

#[cfg(test)]
mod unique_list_tests {
    use super::*;

    #[test]
    fn test_list_to_vec() {
        let mut list = UniqueList::default();
        list.append_if_new("a".into());
        list.append_if_new("b".into());
        list.append_if_new("c".into());
        list.append_if_new("b".into());

        let vec: Vec<String> = vec!["a".into(), "b".into(), "c".into()];

        let intoed: Vec<String> = list.clone().into();

        assert_eq!(intoed, vec);
        assert_eq!(Vec::from(list), vec);
    }

    #[test]
    fn test_vec_to_list() {
        let vec: Vec<String> = vec!["a".into(), "b".into(), "c".into(), "d".into(), "b".into()];
        let mut list = UniqueList::default();
        list.append_if_new("a".into());
        list.append_if_new("b".into());
        list.append_if_new("c".into());
        list.append_if_new("d".into());
        list.append_if_new("b".into());

        let intoed: UniqueList = vec.clone().into();

        assert_eq!(intoed, list);
        assert_eq!(UniqueList::from(vec), list);
    }

    #[test]
    fn test_add_new() {
        let mut l = UniqueList::default();
        l.append_if_new("a".into());
        let r: Vec<String> = l.into();
        assert_eq!(r, vec!["a".to_string()]);
    }

    #[test]
    fn test_add_unique() {
        let mut l = UniqueList::default();
        l.append_if_new("a".into());
        l.append_if_new("a".into());
        let r: Vec<String> = l.into();
        assert_eq!(r, vec!["a".to_string()]);
    }

    #[test]
    fn test_merge() {
        let mut a: UniqueList = vec!["a".into()].into();
        let b: UniqueList = vec!["b".into()].into();

        a.merge(b);

        let r: Vec<String> = a.into();
        assert_eq!(r, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn test_merge_unique_append() {
        let mut a: UniqueList = vec!["b".into(), "a".into()].into();
        let b: UniqueList = vec!["b".into()].into();

        a.merge(b);

        let r: Vec<String> = a.into();
        assert_eq!(r, vec!["b".to_string(), "a".to_string()]);
    }

    #[test]
    fn test_deserialize() {
        let yaml = r#"
        - a
        - b
        "#;
        let l: UniqueList = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(l.items, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn test_deserialize_unique() {
        let yaml = r#"
        - a
        - b
        - a
        "#;
        let l: UniqueList = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(l.items, vec!["a".to_string(), "b".to_string()]);
    }
}
