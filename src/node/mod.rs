use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};
// TODO(sg): Switch to serde_yaml's `apply_merge()` once it supports recursive merges, cf.
// https://github.com/dtolnay/serde-yaml/issues/362
use yaml_merge_keys::merge_keys_serde;

use crate::list::{List, RemovableList, UniqueList};
use crate::types::Mapping;
use crate::{Reclass, SUPPORTED_YAML_EXTS};

mod nodeinfo;

pub(crate) use nodeinfo::*;

/// Represents a Reclass node or class
#[derive(Debug, Default, Deserialize)]
pub struct Node {
    /// List of Reclass applications for this node
    #[serde(default)]
    pub applications: RemovableList,
    /// List of Reclass classes included by this node
    #[serde(default)]
    pub classes: UniqueList,
    /// Reclass parameters for this node as parsed from YAML
    #[serde(default, rename = "parameters")]
    _params: serde_yaml::Mapping,
    /// Reclass parameters for this node converted into our own mapping type
    #[serde(skip)]
    parameters: Mapping,
    /// Location of this node relative to `classes_path`. `None` for nodes.
    #[serde(skip)]
    own_loc: Option<PathBuf>,
    /// Information about the node, empty (default value) for Node objects parsed from classes.
    #[serde(skip)]
    meta: NodeInfoMeta,
}

/// Loads data from `<npath>.yml` or `<npath>.yaml`.
fn load_file(npath: &Path) -> Result<(String, PathBuf)> {
    let mut ncontents: Result<(String, PathBuf)> =
        Err(anyhow!("Node `{}.ya?ml` not found", npath.display()));
    // Try both `.yml` and `.yaml` for both nodes and classes. Prefer `.yml` if both exist.
    for ext in SUPPORTED_YAML_EXTS {
        let np = npath.with_extension(ext);
        if let Ok(contents) = std::fs::read_to_string(&np) {
            ncontents = Ok((contents, np));
            break;
        }
    }
    ncontents
}

impl Node {
    /// Parse node from file with basename `name` in `r.nodes_path`.
    ///
    /// The heavy lifting is done in `load_file` and `Node::from_str`.
    pub fn parse(r: &Reclass, name: &str) -> Result<Self> {
        let mut meta = NodeInfoMeta::new(name, name, "", "base");

        let mut npath = PathBuf::from(&r.nodes_path);
        npath.push(name);
        let (ncontents, fname) = load_file(&npath)?;

        meta.uri = format!("yaml_fs://{}", std::fs::canonicalize(fname)?.display());

        Node::from_str(meta, None, &ncontents)
    }

    /// Initializes a `Node` struct from a string.
    ///
    /// The given string is parsed as YAML. Parameter `npath` is interpreted as the node's location
    /// in the class hierarchy. If the parameter is `None`, relative includes are treated as
    /// relative to `classes_path`.
    pub fn from_str(meta: NodeInfoMeta, npath: Option<PathBuf>, ncontents: &str) -> Result<Self> {
        let mut n: Node = serde_yaml::from_str(ncontents)?;
        n.own_loc = npath;
        n.meta = meta;

        // Transform any relative class names to absolute class names, based on the new node's
        // `own_loc`.
        let mut classes = UniqueList::with_capacity(n.classes.len());
        for cls in n.classes.items_iter() {
            classes.append_if_new(n.abs_class_name(cls)?);
        }
        classes.shrink_to_fit();
        n.classes = classes;

        // Resolve YAML merge keys in `_params`
        let p = merge_keys_serde(serde_yaml::Value::from(n._params))?
            .as_mapping()
            .unwrap()
            .to_owned();
        n._params = p;

        // Convert serde_yaml::Mapping into our own Mapping type
        let mut params: Mapping = n._params.clone().into();
        // Inject `_reclass_` meta parameter into parameters, return an error if someone has marked
        // `_reclass_` or a subkey of it as constant.
        params.insert("_reclass_".into(), n.meta.as_reclass().into())?;
        n.parameters = params;

        Ok(n)
    }

    /// Turns a relative class name (prefixed with one or more `.`) into an absolute class name
    /// based on the current `Node`'s location (field `own_loc`).
    ///
    /// Note that an arbitrary number of leading dots will be consumed, but the top-most directory
    /// which can anchor the class is the directory given as `classes_path`.
    fn abs_class_name(&self, class: &str) -> Result<String> {
        if !class.starts_with('.') {
            // bail early for absolute classes
            return Ok(class.to_string());
        }

        let mut cls = class;
        // Parent starts out as the directory of our own class, or '.' for nodes
        let mut parent = if let Some(loc) = &self.own_loc {
            PathBuf::from(loc)
        } else {
            PathBuf::from(".")
        };
        if cls.starts_with('.') {
            // push placeholder, so the popping in the loop correctly places .foo in the same
            // directory as ourselves.
            parent.push("<placeholder>");
        }

        // Process any number of prefixed `.`, moving up the hierarchy, stopping at the root.
        while let Some(next) = cls.strip_prefix('.') {
            parent.pop();
            cls = next;
        }

        // Render the absolute path of the class (relative to `self.classes_dir`)
        let mut absclass = String::new();
        // If we have a relative reference past the lookup root, `components()` will be empty, and
        // the resulting absolute path will be based in the lookup root.
        for d in parent.components() {
            match d {
                std::path::Component::Normal(p) => {
                    absclass.push_str(p.to_str().unwrap());
                    absclass.push('.');
                }
                // if we've reached CurDir, we've reached the lookup root exactly
                std::path::Component::CurDir => {}
                _ => {
                    return Err(anyhow!(
                        "Unexpected non-normal path segment in class lookup: {:?}",
                        d
                    ))
                }
            }
        }
        absclass.push_str(cls);

        Ok(absclass)
    }

    /// Looks up and parses `Node` from provided `class` string relative to own location.
    ///
    /// If the current Node's location is empty, relative class references inherently turn into
    /// absolute references, since relative references can't escape the `r.classes_path` base
    /// directory.
    ///
    /// If the class is not prefixed with one or more dots, it's looked up relative to
    /// `r.classes_path`.
    ///
    /// The method extracts the the relative file path for the class in `r.classes_dir` from
    /// `r.classes`.
    fn read_class(&self, r: &Reclass, class: &str) -> Result<Option<Self>> {
        let cls = self.abs_class_name(class)?;

        // Lookup path for provided class in r.classes, handling ignore_class_notfound
        let Some(cpath) = r.classes.get(&cls) else {
            if r.ignore_class_notfound {
                return Ok(None);
            }
            return Err(anyhow!("Class {cls} not found"));
        };

        // Extract the directory in which the new class is stored to use for the new class's
        // `own_loc`.
        let class_loc = if let Some(parent) = cpath.parent() {
            PathBuf::from(parent)
        } else {
            PathBuf::new()
        };

        // Render inventory path of class based from `r.classes_path`.
        let mut invpath = PathBuf::from(&r.classes_path);
        invpath.push(cpath);

        // Load file contents and create Node
        let mut meta = NodeInfoMeta::default();
        let ccontents = std::fs::read_to_string(invpath.canonicalize()?)?;
        meta.uri = format!("yaml_fs://{}", invpath.canonicalize()?.display());
        Ok(Some(
            Node::from_str(meta, Some(class_loc), &ccontents)
                .with_context(|| format!("Deserializing {cls}"))?,
        ))
    }
}

#[cfg(test)]
mod node_tests {
    use super::*;

    #[test]
    fn test_parse() {
        let r = Reclass::new(
            "./tests/inventory/nodes",
            "./tests/inventory/classes",
            false,
        )
        .unwrap();
        let n = Node::parse(&r, "n1").unwrap();
        println!("{:#?}", n.meta);
        assert_eq!(
            n.classes,
            UniqueList::from(vec!["cls1".to_owned(), "cls2".to_owned()])
        );
        assert_eq!(
            n.applications,
            RemovableList::from(vec!["app1".to_owned(), "app2".to_owned()])
        );
        let expected = r#"
        foo:
          foo: foo
        bar:
          foo: foo
        _reclass_:
          environment: base
          name:
            short: n1
            parts: ["n1"]
            full: n1
            path: n1
        "#;
        let expected: serde_yaml::Mapping = serde_yaml::from_str(expected).unwrap();
        assert_eq!(n.parameters, expected.into());
    }

    #[test]
    fn test_from_str() {
        let node = r#"
        classes:
          - foo
          - bar
        applications:
          - foo
          - bar
        parameters:
          foo:
            bar: bar
        "#;

        let n = Node::from_str(NodeInfoMeta::default(), None, node).unwrap();
        assert_eq!(
            n.classes,
            UniqueList::from(vec!["foo".to_owned(), "bar".to_owned()])
        );
        assert_eq!(
            n.applications,
            RemovableList::from(vec!["foo".to_owned(), "bar".to_owned()])
        );
        let mut foo = serde_yaml::Mapping::new();
        foo.insert(
            serde_yaml::Value::from("bar"),
            serde_yaml::Value::from("bar"),
        );
        let mut params = serde_yaml::Mapping::new();
        params.insert(serde_yaml::Value::from("foo"), serde_yaml::Value::from(foo));
        assert_eq!(n._params, params);
    }

    #[test]
    fn test_from_str_merge_keys() {
        let node = r#"
        parameters:
          foo: &foo
            bar: bar
          fooer:
            <<: *foo
        "#;
        let n = Node::from_str(NodeInfoMeta::default(), None, node).unwrap();
        let expected = r#"
        foo:
          bar: bar
        fooer:
          bar: bar
        "#;
        let expected: serde_yaml::Mapping = serde_yaml::from_str(expected).unwrap();
        assert_eq!(n._params, expected);
    }

    #[test]
    fn test_from_str_merge_keys_nested() {
        let node = r#"
        parameters:
          foo: &foo
            bar: bar
          fooer:
            bar:
              <<: *foo
        "#;
        let n = Node::from_str(NodeInfoMeta::default(), None, node).unwrap();
        let expected = r#"
        foo:
          bar: bar
        fooer:
          bar:
            bar: bar
        "#;
        let expected: serde_yaml::Mapping = serde_yaml::from_str(expected).unwrap();
        assert_eq!(n._params, expected);
    }

    #[test]
    fn test_from_str_merge_keys_recursive() {
        // NOTE(sg): This test fails when using serde_yaml's `apply_merge` instead of the
        // yaml-merge-keys crate. Example input taken from the serde_yaml issue linked at the top.
        let node = r#"
        parameters:
          a: &a
            a: a
          b: &b
            <<: *a
          c:
            - <<: *a
            - <<: *b
        "#;
        let n = Node::from_str(NodeInfoMeta::default(), None, node).unwrap();
        let expected = r#"
        a:
          a: a
        b:
          a: a
        c:
          - a: a
          - a: a
        "#;
        let expected: serde_yaml::Mapping = serde_yaml::from_str(expected).unwrap();
        assert_eq!(n._params, expected);
    }

    #[test]
    fn abs_class_name_already_abs() {
        let n = Node::default();
        let p = n.abs_class_name("foo").unwrap();
        assert_eq!(p, "foo");
    }

    #[test]
    fn abs_class_name_already_abs_in_subclass() {
        let mut c = Node::default();
        let cpath = PathBuf::from("foo/bar/baz");
        c.own_loc = Some(cpath);
        let p = c.abs_class_name("foo.bar").unwrap();
        assert_eq!(p, "foo.bar");
    }

    #[test]
    fn abs_class_name_same_dir() {
        let mut c = Node::default();
        let cpath = PathBuf::from("foo/bar/baz");
        c.own_loc = Some(cpath);
        let p = c.abs_class_name(".foo").unwrap();
        assert_eq!(p, "foo.bar.baz.foo");
    }

    #[test]
    fn abs_class_name_same_dir_subclass() {
        let mut c = Node::default();
        let cpath = PathBuf::from("foo/bar/baz");
        c.own_loc = Some(cpath);
        let p = c.abs_class_name(".foo.bar").unwrap();
        assert_eq!(p, "foo.bar.baz.foo.bar");
    }

    #[test]
    fn abs_class_name_parent_dir() {
        let mut c = Node::default();
        let cpath = PathBuf::from("foo/bar/baz");
        c.own_loc = Some(cpath);
        let p = c.abs_class_name("..foo").unwrap();
        assert_eq!(p, "foo.bar.foo");
    }

    #[test]
    fn abs_class_name_multi_parent_dir() {
        let mut c = Node::default();
        let cpath = PathBuf::from("foo/bar/baz");
        c.own_loc = Some(cpath);
        let p = c.abs_class_name("...foo").unwrap();
        assert_eq!(p, "foo.foo");
    }

    #[test]
    fn abs_class_name_exact_root_dir() {
        let mut c = Node::default();
        let cpath = PathBuf::from("foo/bar/baz");
        c.own_loc = Some(cpath);
        let p = c.abs_class_name("....foo").unwrap();
        assert_eq!(p, "foo");
    }

    #[test]
    fn abs_class_name_past_root_dir() {
        let mut c = Node::default();
        let cpath = PathBuf::from("foo/bar/baz");
        c.own_loc = Some(cpath);
        let p = c.abs_class_name(".....foo").unwrap();
        assert_eq!(p, "foo");
    }

    #[test]
    fn abs_class_name_past_root_dir_subclass() {
        let mut c = Node::default();
        let cpath = PathBuf::from("foo/bar/baz");
        c.own_loc = Some(cpath);
        let p = c.abs_class_name(".....foo.bar").unwrap();
        assert_eq!(p, "foo.bar");
    }

    #[test]
    fn test_read_class() {
        let r = Reclass::new(
            "./tests/inventory/nodes",
            "./tests/inventory/classes",
            false,
        );
        let n = Node::parse(&r, "n1").unwrap();
        let c = n.read_class(&r, "cls1").unwrap().unwrap();
        let expected = r#"
        foo:
          foo: cls1
          bar: cls1
          baz: cls1
        "#;
        let expected = Mapping::from_str(expected).unwrap();
        assert_eq!(c.parameters, expected);
    }

    #[test]
    fn test_read_class_relative() {
        let r = Reclass::new(
            "./tests/inventory/nodes",
            "./tests/inventory/classes",
            false,
        );
        let n = Node::parse(&r, "n1").unwrap();
        let c1 = n.read_class(&r, "nested.cls1").unwrap().unwrap();
        let c2 = c1.read_class(&r, ".cls2").unwrap().unwrap();
        let expected = r#"
        foo:
          foo: nested.cls2
        "#;
        let expected = Mapping::from_str(expected).unwrap();
        assert_eq!(c2.parameters, expected);
    }
}
