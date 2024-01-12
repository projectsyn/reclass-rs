use anyhow::{anyhow, Result};
use pyo3::prelude::*;
use std::path::PathBuf;

use crate::fsutil::to_lexical_normal;

#[pyclass]
#[derive(Clone, Debug, Default)]
pub struct Config {
    /// Base path of the inventory
    #[pyo3(get)]
    pub inventory_path: String,
    /// Path to node definitions in the inventory. This should be a subdirectory of
    /// `inventory_path`.
    #[pyo3(get)]
    pub nodes_path: String,
    /// Path to class definitions in the inventory. This should be a subdirectory of
    /// `inventory_path`.
    #[pyo3(get)]
    pub classes_path: String,
    /// Whether to ignore included classes which don't exist (yet)
    #[pyo3(get)]
    pub ignore_class_notfound: bool,
}

impl Config {
    /// Creates a new `Config` from the provided parameters.
    ///
    /// If neither `inventory_path` nor `classes_path` (or `nodes_path`) is given, the method
    /// returns an error.
    ///
    /// If `inventory_path` is omitted, the component defaults to the current directory.
    /// Config options `classes_path` and `nodes_path` are expected to be relative paths to
    /// `inventory_path`. If these arguments are None, we default to `nodes` and `classes`
    /// respectively. If `ignore_class_notfound` is None, we default the option to false.
    pub fn new(
        inventory_path: Option<&str>,
        nodes_path: Option<&str>,
        classes_path: Option<&str>,
        ignore_class_notfound: Option<bool>,
    ) -> Result<Self> {
        if inventory_path.is_none() && nodes_path.is_none() {
            return Err(anyhow!(
                "One of inventory path and nodes path must be provided."
            ));
        }
        if inventory_path.is_none() && classes_path.is_none() {
            return Err(anyhow!(
                "One of inventory path and classes path must be provided."
            ));
        }
        let inventory_path = inventory_path.unwrap_or(".");
        let mut npath = PathBuf::from(inventory_path);
        if let Some(p) = nodes_path {
            npath.push(p);
        } else {
            npath.push("nodes");
        };
        let mut cpath = PathBuf::from(inventory_path);
        if let Some(p) = classes_path {
            cpath.push(p);
        } else {
            cpath.push("classes");
        };
        if npath == cpath || npath.starts_with(&cpath) || cpath.starts_with(&npath) {
            return Err(anyhow!("Nodes and classes path must be non-overlapping."));
        }
        Ok(Self {
            inventory_path: inventory_path.into(),
            nodes_path: to_lexical_normal(&npath, true).display().to_string(),
            classes_path: to_lexical_normal(&cpath, true).display().to_string(),
            ignore_class_notfound: ignore_class_notfound.unwrap_or(false),
        })
    }

    /// Construct path to node from `self.inventory_path`, `self.nodes_path` and the provided path
    /// to the node relative to the inventory nodes directory.
    pub(crate) fn node_path(&self, npath: &PathBuf) -> PathBuf {
        let mut invpath = PathBuf::from(&self.nodes_path);
        invpath.push(npath);
        invpath
    }

    /// Construct path to class from `self.inventory_path`, `self.classes_path` and the provided
    /// path to the class relative to the inventory classes directory.
    pub(crate) fn class_path(&self, cpath: &PathBuf) -> PathBuf {
        let mut invpath = PathBuf::from(&self.classes_path);
        invpath.push(cpath);
        invpath
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "One of inventory path and nodes path must be provided.")]
    fn test_config_missing_nodes() {
        let cfg = Config::new(None, None, None, None);
        assert!(cfg.is_err());
        cfg.unwrap();
    }

    #[test]
    #[should_panic(expected = "One of inventory path and classes path must be provided.")]
    fn test_config_missing_classes() {
        let cfg = Config::new(None, Some("nodes"), None, None);
        assert!(cfg.is_err());
        cfg.unwrap();
    }

    #[test]
    #[should_panic(expected = "Nodes and classes path must be non-overlapping.")]
    fn test_config_missing_non_overlapping_identical() {
        let cfg = Config::new(None, Some("nodes"), Some("nodes"), None);
        assert!(cfg.is_err());
        cfg.unwrap();
    }

    #[test]
    #[should_panic(expected = "Nodes and classes path must be non-overlapping.")]
    fn test_config_missing_non_overlapping_nodes_parent() {
        let cfg = Config::new(None, Some(""), Some("classes"), None);
        assert!(cfg.is_err());
        cfg.unwrap();
    }

    #[test]
    #[should_panic(expected = "Nodes and classes path must be non-overlapping.")]
    fn test_config_missing_non_overlapping_classes_parent() {
        let cfg = Config::new(None, Some("nodes"), Some(""), None);
        assert!(cfg.is_err());
        cfg.unwrap();
    }

    #[test]
    fn test_config_defaults() {
        let cfg = Config::new(Some("./inventory"), None, None, None).unwrap();
        assert_eq!(cfg.nodes_path, "./inventory/nodes");
        assert_eq!(cfg.classes_path, "./inventory/classes");
        assert_eq!(cfg.ignore_class_notfound, false);
    }

    #[test]
    fn test_config_concatenate() {
        let cfg =
            Config::new(Some("./inventory"), Some("targets"), Some("settings"), None).unwrap();
        assert_eq!(cfg.nodes_path, "./inventory/targets");
        assert_eq!(cfg.classes_path, "./inventory/settings");
        assert_eq!(cfg.ignore_class_notfound, false);
    }

    #[test]
    fn test_config_normalize() {
        let cfg = Config::new(
            Some("./inventory"),
            Some("targets/../targets/."),
            None,
            None,
        )
        .unwrap();
        assert_eq!(cfg.nodes_path, "./inventory/targets");
        assert_eq!(cfg.classes_path, "./inventory/classes");
        assert_eq!(cfg.ignore_class_notfound, false);
    }
}
