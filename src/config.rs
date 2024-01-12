use pyo3::prelude::*;

#[pyclass]
#[derive(Clone, Debug, Default)]
pub struct Config {
    /// Path to node definitions in inventory
    #[pyo3(get)]
    pub nodes_path: String,
    /// Path to class definitions in inventory
    #[pyo3(get)]
    pub classes_path: String,
    /// Whether to ignore included classes which don't exist (yet)
    #[pyo3(get)]
    pub ignore_class_notfound: bool,
}

impl Config {
    pub fn new(nodes_path: &str, classes_path: &str, ignore_class_notfound: bool) -> Self {
        Self {
            nodes_path: nodes_path.into(),
            classes_path: classes_path.into(),
            ignore_class_notfound,
        }
    }
}
