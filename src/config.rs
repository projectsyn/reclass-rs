use anyhow::{anyhow, Result};
use fancy_regex::Regex;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyType};
use regex::RegexSet;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::LazyLock;

use crate::fsutil::to_lexical_normal;
use crate::list::{List, UniqueList};
use crate::NodeInfoMeta;

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

#[derive(Clone, Debug)]
enum Pattern {
    Glob(glob::Pattern),
    Regex(Regex),
}

#[derive(Debug, Clone)]
struct ClassMapping {
    pat: String,
    classes: Vec<String>,
    pattern: Pattern,
}

impl std::fmt::Display for ClassMapping {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.pat, self.classes.join(" "))
    }
}

impl Default for ClassMapping {
    fn default() -> Self {
        Self {
            pat: "*".to_owned(),
            pattern: Pattern::Glob(glob::Pattern::new("*").unwrap()),
            classes: Vec::new(),
        }
    }
}

fn parse_class_mapping(cmspec: &str) -> Result<(&str, Vec<&str>)> {
    let mut parts = cmspec.split_whitespace();
    let pat = parts
        .next()
        .ok_or(anyhow!("Expected '<Pattern> <classes>'"))?;
    // unescape leading '*' for glob patterns. Leading '*' needs to be escaped to avoid YAML
    // parsing issues unless strings are explicitly wrapped in quotes.
    let pat = if pat.starts_with("\\*") {
        pat.strip_prefix('\\').unwrap()
    } else {
        pat
    };
    let classes = parts.collect::<Vec<&str>>();
    if classes.is_empty() {
        return Err(anyhow!("No classes mapped for {pat}"));
    }

    Ok((pat, classes))
}

fn replace_regex_backrefs(s: &str) -> String {
    static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\\\\(\d+)").unwrap());
    RE.replace_all(s, r"$${$1}").into_owned()
}

impl ClassMapping {
    fn new(cmspec: &str) -> Result<Self> {
        let (pat, classes) = parse_class_mapping(cmspec)?;
        let pattern = if pat.starts_with('/') {
            if pat
                .chars()
                .next_back()
                .ok_or(anyhow!("Regex pattern is empty?"))?
                != '/'
            {
                return Err(anyhow!("Expected regex pattern to be enclosed in `/`"));
            }
            // Strip enclosing '/' from pattern if pattern starts and ends with '/'
            let p = &pat[1..pat.len() - 1];
            if p.is_empty() {
                return Err(anyhow!("empty regex patterns are not supported"));
            }
            Pattern::Regex(
                Regex::new(p).map_err(|e| anyhow!("While compiling regex pattern {pat}: {e}"))?,
            )
        } else {
            Pattern::Glob(
                glob::Pattern::new(pat)
                    .map_err(|e| anyhow!("While compiling glob pattern {pat}: {e}"))?,
            )
        };
        Ok(Self {
            pat: pat.to_owned(),
            pattern,
            classes: classes
                .iter()
                .map(|&s| replace_regex_backrefs(s))
                .collect::<Vec<String>>(),
        })
    }

    /// This function appends the classes defined for the current `ClassMapping` to the passed
    /// `UniqueList` if the passed `node` str matches the glob or regex pattern.
    ///
    /// NOTE: this function expects that `node` is the node name or path depending on
    /// `class_mappings_match_path` and won't modify the passed string.
    fn append_if_matches(&self, node: &str, mapped_cls: &mut UniqueList) -> Result<()> {
        match &self.pattern {
            Pattern::Regex(re) => {
                if let Some(cap) = re.captures(node)? {
                    for c in &self.classes {
                        let mut cls = String::new();
                        cap.expand(c, &mut cls);
                        mapped_cls.append_if_new(cls);
                    }
                }
            }
            Pattern::Glob(glob) => {
                if glob.matches(node) {
                    for c in &self.classes {
                        mapped_cls.append_if_new(c.clone());
                    }
                }
            }
        }
        Ok(())
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
    /// List of regex patterns for which to ignore included classes which don't exist (yet)
    ///
    /// If no values are provided in this parameter, classes matching `.*` are ignored if
    /// `ignore_class_notfound=true`.
    #[pyo3(get)]
    ignore_class_notfound_regexp: Vec<String>,
    ignore_class_notfound_regexset: RegexSet,
    /// Python Reclass compatibility flags. See `CompatFlag` for available flags.
    #[pyo3(get)]
    pub compatflags: HashSet<CompatFlag>,
    /// Whether to match the nodes' paths in the inventory when applying entries in
    /// `class_mappings`.
    ///
    /// Defaults to `false`.
    #[pyo3(get)]
    pub class_mappings_match_path: bool,
    /// Class mappings provides a mechanism to include one or more classes by default in all nodes
    /// which match a glob or regex pattern.
    #[pyo3(get)]
    pub class_mappings: Vec<String>,
    // NOTE(sg): we need to preserve the order of class mappings since the order in the config
    // determines the order in which classes are included and class include order can be
    // semantically relevant depending on the contents of each included class.
    class_mappings_patterns: Vec<ClassMapping>,
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
            class_mappings: Vec::new(),
            class_mappings_patterns: Vec::new(),
            class_mappings_match_path: false,
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
            "class_mappings_match_path" => {
                self.class_mappings_match_path = v.as_bool().ok_or(anyhow!(
                    "Expected value of config key 'class_mappings_match_path' to be a boolean"
                ))?;
            }
            "class_mappings" => {
                let cmlist = v.as_sequence().ok_or(anyhow!(
                    "Expected value of config key 'class_mappings' to be a list"
                ))?;
                self.class_mappings = cmlist
                    .iter()
                    .map(|v| {
                        v.as_str().map(ToOwned::to_owned).ok_or(anyhow!(
                            "Expected entry of config key 'class_mappings' to be a string"
                        ))
                    })
                    .collect::<Result<Vec<String>>>()?;
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
        self.compile_class_mapping_patterns()?;
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

    /// This function returns a list of classes to include based on the passed NodeInfo.
    ///
    /// The function uses `NodeInfo::class_mappings_match_name()` to determine the node name for
    /// matching class_mappings patterns. That function takes into account
    /// `class_mappings_match_path`.
    pub(crate) fn get_class_mappings(&self, node: &NodeInfoMeta) -> Result<UniqueList> {
        let mut mapped_cls = UniqueList::new();
        let matchname = node.class_mappings_match_name(self)?;
        for cm in &self.class_mappings_patterns {
            cm.append_if_matches(matchname, &mut mapped_cls)?;
        }
        Ok(mapped_cls)
    }

    fn compile_class_mapping_patterns(&mut self) -> Result<()> {
        self.class_mappings_patterns = self
            .class_mappings
            .iter()
            .map(|s| ClassMapping::new(&s[..]))
            .collect::<Result<Vec<_>>>()?;
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
        cfg.compile_ignore_class_notfound_patterns().map_err(|e| {
            PyValueError::new_err(format!(
                "Error while compiling class_notfound_regexp patterns: {e}"
            ))
        })?;
        cfg.compile_class_mapping_patterns().map_err(|e| {
            PyValueError::new_err(format!(
                "Error while compiling class_mappings patterns: {e}"
            ))
        })?;

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

    #[test]
    fn test_config_parse_class_mappings() {
        let mut cfg =
            Config::new(Some("./tests/inventory-class-mapping"), None, None, None).unwrap();
        cfg.load_from_file("reclass-config.yml", false).unwrap();
        assert!(cfg.class_mappings_match_path);
        let expected_mappings = vec![
            ("*", vec!["common"]),
            ("*", vec!["defaults"]),
            ("test/*", vec!["cluster.test"]),
            ("production/*", vec!["cluster.production"]),
            ("test.*", vec!["composed.test"]),
            ("production.*", vec!["composed.production"]),
            (
                "/(test|production)\\/.*/",
                vec!["regex.params", "regex.\\\\1"],
            ),
            ("/(test)\\/.*/", vec!["regex.rust-${1}"]),
            ("/^test(?!.*-stg-test).*/", vec!["cluster.test"]),
            ("/^test.*-stg-test.*/", vec!["cluster.staging"]),
            ("/.*c$/", vec!["class1", "class2"]),
        ];
        let mappings = cfg
            .class_mappings
            .iter()
            .map(|s| parse_class_mapping(&s[..]))
            .collect::<Result<Vec<_>>>()
            .unwrap();
        dbg!(&mappings);
        assert_eq!(mappings, expected_mappings);
    }

    #[test]
    fn test_replace_regex_backrefs_none_single() {
        let classes = vec![
            ("foo", "foo"),
            ("foo.bar", "foo.bar"),
            ("foo-\\\\1", "foo-${1}"),
            ("foo-\\\\1234.bar", "foo-${1234}.bar"),
            ("foo-\\\\12f", "foo-${12}f"),
        ];
        for (c, e) in &classes {
            let r = replace_regex_backrefs(c);
            assert_eq!(&r, e);
        }
    }
    #[test]
    fn test_replace_regex_backrefs_multiple() {
        let classes = vec![
            ("foo-\\\\1\\\\2", "foo-${1}${2}"),
            ("foo-\\\\1234.\\\\1", "foo-${1234}.${1}"),
        ];
        for (c, e) in &classes {
            let r = replace_regex_backrefs(c);
            assert_eq!(&r, e);
        }
    }
}
