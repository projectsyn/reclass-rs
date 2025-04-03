use anyhow::{anyhow, Result};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyType};
use regex::RegexSet;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

use crate::fsutil::to_lexical_normal;

/// Flags to change reclass-rs behavior to be compaible with Python reclass
#[pyclass(eq, eq_int)]
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum CompatFlag {
    /// This flag enables Python Reclass-compatible rendering of fields `path` and `parts` in
    /// `NodeInfoMeta` when Reclass option `compose-node-name` is enabled.
    ///
    /// By default, if this flag isn't enabled, reclass-rs will preserve literal dots in the node's
    /// file path when rendering fields `path` and `parts` in `NodeInfoMeta` when
    /// `compose-node-name` is enabled.
    ComposeNodeNameLiteralDots,
}

#[pymethods]
impl CompatFlag {
    fn __hash__(&self) -> u64 {
        let mut h = DefaultHasher::new();
        self.hash(&mut h);
        h.finish()
    }
}

impl TryFrom<&str> for CompatFlag {
    type Error = anyhow::Error;
    fn try_from(value: &str) -> Result<Self> {
        match value {
            "compose-node-name-literal-dots"
            | "compose_node_name_literal_dots"
            | "ComposeNodeNameLiteralDots" => Ok(Self::ComposeNodeNameLiteralDots),
            _ => Err(anyhow!("Unknown compatibility flag '{value}'")),
        }
    }
}

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
    /// Whether to treat nested files in `nodes_path` as node definitions
    #[pyo3(get)]
    pub compose_node_name: bool,
    /// Python Reclass compatibility flags. See `CompatFlag` for available flags.
    #[pyo3(get)]
    ignore_class_notfound_regexp: Vec<String>,
    ignore_class_notfound_regexset: RegexSet,
    #[pyo3(get)]
    pub compatflags: HashSet<CompatFlag>,
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
        }
        let mut cpath = PathBuf::from(inventory_path);
        if let Some(p) = classes_path {
            cpath.push(p);
        } else {
            cpath.push("classes");
        }
        if npath == cpath || npath.starts_with(&cpath) || cpath.starts_with(&npath) {
            return Err(anyhow!("Nodes and classes path must be non-overlapping."));
        }
        Ok(Self {
            inventory_path: inventory_path.into(),
            nodes_path: to_lexical_normal(&npath, true).display().to_string(),
            classes_path: to_lexical_normal(&cpath, true).display().to_string(),
            ignore_class_notfound: ignore_class_notfound.unwrap_or(false),
            compose_node_name: false,
            ignore_class_notfound_regexp: vec![".*".to_string()],
            ignore_class_notfound_regexset: RegexSet::new([".*"])?,
            compatflags: HashSet::new(),
        })
    }

    fn set_option(
        &mut self,
        cfg_path: &std::path::Path,
        k: &str,
        v: &serde_yaml::Value,
        verbose: bool,
    ) -> Result<()> {
        let vstr = serde_yaml::to_string(v)?;
        let vstr = vstr.trim();
        match k {
            "nodes_uri" => {
                cfg_path
                    .with_file_name(vstr)
                    .to_str()
                    .ok_or(anyhow!("Can't create nodes path from config file"))?
                    .clone_into(&mut self.nodes_path);
            }
            "classes_uri" => {
                cfg_path
                    .with_file_name(vstr)
                    .to_str()
                    .ok_or(anyhow!("Can't create nodes path from config file"))?
                    .clone_into(&mut self.classes_path);
            }
            "ignore_class_notfound" => {
                self.ignore_class_notfound = v.as_bool().ok_or(anyhow!(
                    "Expected value of config key 'ignore_class_notfound' to be a boolean"
                ))?;
            }
            "ignore_class_notfound_regexp" => {
                let list = v.as_sequence().ok_or(anyhow!(
                    "Expected value of config key 'ignore_class_notfound_regexp' to be a list"
                ))?;
                self.ignore_class_notfound_regexp.clear();
                for val in list {
                    self.ignore_class_notfound_regexp.push(
                        val.as_str()
                            .ok_or(anyhow!(
                                "Expected entry of 'ignore_class_notfound_regexp' to be a string"
                            ))?
                            .to_string(),
                    );
                }
                self.ignore_class_notfound_regexp.shrink_to_fit();
            }
            "compose_node_name" => {
                self.compose_node_name = v.as_bool().ok_or(anyhow!(
                    "Expected value of config key 'compose_node_name' to be a boolean"
                ))?;
            }
            "reclass_rs_compat_flags" => {
                let flags = v.as_sequence().ok_or(anyhow!(
                    "Expected value of config key 'reclass_rs_compat_flags' to be a list"
                ))?;
                for f in flags {
                    let f = f
                        .as_str()
                        .ok_or(anyhow!("Expected compatibility flag to be a string"))?;
                    if let Ok(flag) = CompatFlag::try_from(f) {
                        self.compatflags.insert(flag);
                    } else {
                        eprintln!("Unknown compatibility flag '{f}', ignoring...");
                    }
                }
            }
            _ => {
                if verbose {
                    eprintln!(
                        "reclass-config.yml entry '{k}={vstr}' not implemented yet, ignoring..."
                    );
                }
            }
        }

        Ok(())
    }

    /// Load additional config options from the file at `<self.inventory_path>/<config_file>`.
    ///
    /// This method assumes that you've created a Config object with a suitable `inventory_path`.
    ///
    /// If `verbose` is true, the method will print diagnostic messages for config options which
    /// aren't implemented yet.
    pub fn load_from_file(&mut self, config_file: &str, verbose: bool) -> Result<()> {
        let mut cfg_path = PathBuf::from(&self.inventory_path);
        cfg_path.push(config_file);

        let cfg_file = std::fs::read_to_string(&cfg_path)?;
        let cfg: serde_yaml::Value = serde_yaml::from_str(&cfg_file)?;
        for (k, v) in cfg
            .as_mapping()
            .ok_or(anyhow!("Expected reclass config to be a Mapping"))?
        {
            let kstr = serde_yaml::to_string(k)?;
            let kstr = kstr.trim();
            self.set_option(&cfg_path, kstr, v, verbose)?;
        }
        self.compile_ignore_class_notfound_patterns()?;
        Ok(())
    }

    /// Returns the currently configured `ignore_class_notfound_regexp` pattern list.
    pub fn get_ignore_class_notfound_regexp(&self) -> &Vec<String> {
        &self.ignore_class_notfound_regexp
    }

    /// Updates the saved ignore_class_notfound_regexp pattern list with the provided list and
    /// ensures that the precompiled RegexSet is updated to match the new pattern list.
    pub fn set_ignore_class_notfound_regexp(&mut self, patterns: Vec<String>) -> Result<()> {
        self.ignore_class_notfound_regexp = patterns;
        self.compile_ignore_class_notfound_patterns()
    }

    pub(crate) fn is_class_ignored(&self, cls: &str) -> bool {
        self.ignore_class_notfound && self.ignore_class_notfound_regexset.is_match(cls)
    }

    fn compile_ignore_class_notfound_patterns(&mut self) -> Result<()> {
        self.ignore_class_notfound_regexset = RegexSet::new(&self.ignore_class_notfound_regexp)
            .map_err(|e| anyhow!("while compiling ignore_class_notfound regex patterns: {e}"))?;
        Ok(())
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

#[pymethods]
impl Config {
    fn __repr__(&self) -> String {
        format!("{self:#?}")
    }

    /// Creates a Config object based on the provided `inventory_path` and the config options
    /// passed in the `config` Python dict. If `verbose` is set to `true`, reclass-rs will print
    /// diagnostic messages for unknown config options.
    ///
    /// Returns a `Config` object or raises a `ValueError`.
    #[classmethod]
    #[pyo3(signature = (inventory_path, config, verbose=false))]
    fn from_dict(
        _cls: &Bound<'_, PyType>,
        inventory_path: &str,
        config: &Bound<'_, PyDict>,
        verbose: bool,
    ) -> PyResult<Self> {
        let mut cfg = Config::new(Some(inventory_path), None, None, None).map_err(|e| {
            PyValueError::new_err(format!(
                "Failed to initialize reclass-rs config object: {e}"
            ))
        })?;

        // `set_option()` expects `cfg_path` to be the path to the reclass config file. Since we're
        // not actually reading from the file here, we need to push an arbitrary path segment so
        // that `set_option()` will configure the `nodes_path` and `classes_path` fields correctly.
        let mut cfg_path = PathBuf::from(inventory_path);
        cfg_path.push("dummy");

        for (k, v) in config {
            let kstr = k.extract::<&str>()?;
            let val: crate::types::Value = TryInto::try_into(v)?;
            cfg.set_option(&cfg_path, kstr, &val.into(), verbose)
                .map_err(|e| {
                    PyValueError::new_err(format!("Error while setting option {kstr}: {e}"))
                })?;
        }

        Ok(cfg)
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

    #[test]
    fn test_config_update_ignore_class_notfound_patterns() {
        let mut cfg = Config::new(Some("./inventory"), None, None, None).unwrap();
        assert_eq!(cfg.ignore_class_notfound_regexp, vec![".*"]);

        cfg.set_ignore_class_notfound_regexp(vec![".*foo".into(), "bar.*".into()])
            .unwrap();

        assert!(cfg.ignore_class_notfound_regexset.is_match("thefooer"));
        assert!(cfg.ignore_class_notfound_regexset.is_match("baring"));
        assert!(!cfg.ignore_class_notfound_regexset.is_match("bazzer"));
    }
}
