use serde::Deserialize;

use super::{item_pos, List};

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(from = "Vec<String>")]
pub struct RemovableList {
    items: Vec<String>,
    #[serde(skip)]
    negations: Vec<String>,
}

impl RemovableList {
    /// Handles negating the provided item
    ///
    /// Assumes that the negation prefix is already stripped from `negitem`
    fn handle_negation(&mut self, negitem: String) {
        if let Some(itpos) = item_pos(&self.items, &negitem) {
            // ...remove item from our list if it's negated in other
            self.items.remove(itpos);
        } else if item_pos(&self.negations, &negitem).is_none() {
            // ...remember negations which we haven't processed yet and
            // which aren't present in self.
            self.negations.push(negitem);
        }
    }

    /// Internal merge implementation which takes negations into account
    ///
    /// Used by List::merge/List::merge_from
    fn merge_impl(
        &mut self,
        itemiter: impl Iterator<Item = String>,
        negiter: impl Iterator<Item = String>,
    ) {
        // merge negations first...
        for n in negiter {
            self.handle_negation(n);
        }
        // take items from other and append them using append_if_new
        for it in itemiter {
            self.append_if_new(it);
        }
    }
}

impl From<Vec<String>> for RemovableList {
    #[inline]
    fn from(item: Vec<String>) -> Self {
        let mut res = RemovableList {
            items: vec![],
            negations: vec![],
        };
        for it in item {
            res.append_if_new(it);
        }
        res
    }
}

impl From<RemovableList> for Vec<String> {
    #[inline]
    fn from(l: RemovableList) -> Self {
        l.items
    }
}

impl List for RemovableList {
    #[inline]
    fn new() -> Self {
        Self::default()
    }

    #[inline]
    fn with_capacity(capacity: usize) -> Self {
        Self {
            items: Vec::with_capacity(capacity),
            negations: vec![],
        }
    }

    #[inline]
    fn len(&self) -> usize {
        self.items.len()
    }

    #[inline]
    fn shrink_to_fit(&mut self) {
        self.items.shrink_to_fit();
        self.negations.shrink_to_fit();
    }

    /// Appends or removes item from list
    ///
    /// Regular strings are inserted in the list if they're not present yet. When `item` is
    /// prefixed with ~ it's removed from the list if present.  Negated items which can't be
    /// removed immediately are stored as negations, for later processing.
    fn append_if_new(&mut self, item: String) {
        if let Some(neg) = item.strip_prefix('~') {
            // handle negation
            self.handle_negation(neg.to_string());
        } else if let Some(negpos) = item_pos(&self.negations, &item) {
            // Remove previously negated item from negations list instead of
            // inserting it into the list.
            self.negations.remove(negpos);
        } else if item_pos(&self.items, &item).is_none() {
            // Finally, insert item if neither condition applies and the item
            // isn't present in the list yet.
            self.items.push(item);
        };
    }

    /// Merges other into self, consuming other
    ///
    /// Negations from other are processed first, removing items which are already present from our
    /// list. Negations which weren't processed are kept and merged into the list's negations.
    /// Afterwards all items in other are taken and appended if they're not present in our list.
    fn merge(&mut self, other: Self) {
        self.merge_impl(other.items.into_iter(), other.negations.into_iter());
    }

    /// Merges other into self, creating a clone of other
    fn merge_from(&mut self, other: &Self) {
        self.merge_impl(other.items.iter().cloned(), other.negations.iter().cloned());
    }
}

#[cfg(test)]
mod removable_list_tests {
    use super::*;

    fn make_abc() -> RemovableList {
        vec!["a".into(), "b".into(), "c".into()].into()
    }

    fn make_def() -> RemovableList {
        vec!["d".into(), "e".into(), "f".into()].into()
    }

    #[test]
    fn test_list_to_vec() {
        let mut list = RemovableList::default();
        list.append_if_new("a".into());
        list.append_if_new("b".into());
        list.append_if_new("c".into());
        list.append_if_new("d".into());
        list.append_if_new("b".into());
        list.append_if_new("~d".into());

        let vec: Vec<String> = vec!["a".into(), "b".into(), "c".into()];

        let intoed: Vec<String> = list.clone().into();

        assert_eq!(intoed, vec);
        assert_eq!(Vec::from(list), vec);
    }

    #[test]
    fn test_vec_to_list() {
        let vec: Vec<String> = vec![
            "a".into(),
            "b".into(),
            "c".into(),
            "d".into(),
            "b".into(),
            "~d".into(),
            "~e".into(),
        ];
        let mut list = RemovableList::default();
        list.append_if_new("a".into());
        list.append_if_new("b".into());
        list.append_if_new("c".into());
        list.append_if_new("d".into());
        list.append_if_new("b".into());
        list.append_if_new("~d".into());
        list.append_if_new("~e".into());

        let intoed: RemovableList = vec.clone().into();

        assert_eq!(intoed, list);
        assert_eq!(RemovableList::from(vec), list);
    }

    #[test]
    fn test_list_add_new() {
        let mut l = make_abc();
        l.append_if_new("d".into());
        let expected: Vec<String> = vec!["a".into(), "b".into(), "c".into(), "d".into()];
        assert_eq!(l.items, expected);
        assert_eq!(l.negations, Vec::<String>::new());
    }

    #[test]
    fn test_list_add_existing() {
        let mut l = make_abc();
        l.append_if_new("c".into());
        let expected: Vec<String> = vec!["a".into(), "b".into(), "c".into()];
        assert_eq!(l.items, expected);
        assert_eq!(l.negations, Vec::<String>::new());
    }

    #[test]
    fn test_list_remove_nonexisting() {
        let mut l = make_abc();
        l.append_if_new("~d".into());
        let expected: Vec<String> = vec!["a".into(), "b".into(), "c".into()];
        assert_eq!(l.items, expected);
        assert_eq!(l.negations, vec!["d".to_string()]);
    }

    #[test]
    fn test_list_remove_existing() {
        let mut l = make_abc();
        l.append_if_new("~b".into());
        let expected: Vec<String> = vec!["a".into(), "c".into()];
        assert_eq!(l.items, expected);
        assert_eq!(l.negations, Vec::<String>::new());
    }

    #[test]
    fn test_list_negate_then_add() {
        let mut l = make_abc();
        l.append_if_new("~d".into());
        l.append_if_new("d".into());
        let expected: Vec<String> = vec!["a".into(), "b".into(), "c".into()];
        assert_eq!(l.items, expected);
        assert_eq!(l.negations, Vec::<String>::new());
    }

    #[test]
    fn test_list_negate_then_negate() {
        let mut l = make_abc();
        l.append_if_new("~d".into());
        l.append_if_new("~d".into());
        let expected: Vec<String> = vec!["a".into(), "b".into(), "c".into()];
        assert_eq!(l.items, expected);
        assert_eq!(l.negations, vec!["d".to_string()]);
    }

    #[test]
    fn test_merge() {
        let mut l = make_abc();
        let o = make_def();
        l.merge(o);

        assert_eq!(
            l.items,
            vec![
                "a".to_string(),
                "b".to_string(),
                "c".to_string(),
                "d".to_string(),
                "e".to_string(),
                "f".to_string()
            ]
        );
        assert_eq!(l.negations, Vec::<String>::new());
    }

    #[test]
    fn test_merge_same() {
        let mut l = make_abc();
        let o = make_abc();
        l.merge(o);

        assert_eq!(
            l.items,
            vec!["a".to_string(), "b".to_string(), "c".to_string(),]
        );
        assert_eq!(l.negations, Vec::<String>::new());
    }

    #[test]
    fn test_merge_add_and_remove() {
        let mut l = make_abc();
        let mut o: RemovableList = vec!["d".into()].into();
        o.append_if_new("~c".into());
        l.merge(o);

        assert_eq!(
            l.items,
            vec!["a".to_string(), "b".to_string(), "d".to_string()]
        );
        assert_eq!(l.negations, Vec::<String>::new());
    }

    #[test]
    fn test_merge_add_store_removal() {
        let mut l = make_abc();
        let mut o: RemovableList = vec!["d".into()].into();
        o.append_if_new("~c".into());
        o.append_if_new("~e".into());
        l.merge(o);

        assert_eq!(
            l.items,
            vec!["a".to_string(), "b".to_string(), "d".to_string()]
        );
        assert_eq!(l.negations, vec!["e".to_string()]);
    }

    #[test]
    fn test_merge_add_apply_removal() {
        let mut l = make_abc();
        l.append_if_new("~d".into());
        let o: RemovableList = vec!["d".into()].into();
        l.merge(o);

        assert_eq!(
            l.items,
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
        assert_eq!(l.negations, Vec::<String>::new());
    }

    #[test]
    fn test_merge_add_store_unique() {
        let mut l = make_abc();
        l.append_if_new("~d".into());
        let mut o = RemovableList::default();
        o.append_if_new("~d".into());
        l.merge(o);

        assert_eq!(
            l.items,
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
        assert_eq!(l.negations, vec!["d".to_string()]);
    }

    #[test]
    fn test_deserialize_process_negations() {
        let yaml = r#"
        - a
        - b
        - ~b
        "#;
        let l: RemovableList = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(l.items, vec!["a".to_string()]);
        assert_eq!(l.negations, Vec::<String>::new());
    }

    #[test]
    fn test_deserialize_remove_duplicates() {
        let yaml = r#"
        - a
        - a
        - b
        "#;
        let l: RemovableList = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(l.items, vec!["a".to_string(), "b".to_string()]);
        assert_eq!(l.negations, Vec::<String>::new());
    }

    #[test]
    fn test_deserialize_store_negations() {
        let yaml = r#"
        - a
        - b
        - ~c
        "#;
        let l: RemovableList = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(l.items, vec!["a".to_string(), "b".to_string()]);
        assert_eq!(l.negations, vec!["c".to_string()]);
    }
}
