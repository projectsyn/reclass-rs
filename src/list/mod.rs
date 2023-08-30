mod removable;
mod unique;

/// Defines the shared interface between the unique list (which is effectively an insert-ordered
/// Set) and the unique list which supports removals.
pub trait List {
    fn new() -> Self;
    fn with_capacity(capacity: usize) -> Self;
    fn len(&self) -> usize;
    fn shrink_to_fit(&mut self);
    fn append_if_new(&mut self, item: String);
    fn merge(&mut self, other: Self);
    fn merge_from(&mut self, other: &Self);
}

/// Returns the 0-indexed position of the item in the list, if it's found
fn item_pos(items: &[String], item: &String) -> Option<usize> {
    items.iter().position(|v| v == item)
}

pub use removable::*;
pub use unique::*;
