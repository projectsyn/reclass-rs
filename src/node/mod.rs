use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::path::PathBuf;
// TODO(sg): Switch to serde_yaml's `apply_merge()` once it supports recursive merges, cf.
// https://github.com/dtolnay/serde-yaml/issues/362
use yaml_merge_keys::merge_keys_serde;

use crate::list::{List, RemovableList, UniqueList};
use crate::refs::Token;
use crate::types::{Mapping, Value};
use crate::{to_lexical_absolute, Reclass};

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
    params: serde_yaml::Mapping,
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

impl Node {
    /// Parse node from file with basename `name` in `r.nodes_path`.
    ///
    /// The heavy lifting is done in `Reclass.discover_nodes()` and `Node::from_str`.
    pub fn parse(r: &Reclass, name: &str) -> Result<Self> {
        let mut meta = NodeInfoMeta::new(name, name, "", "base");

        let npath = r.nodes.get(name).ok_or(anyhow!("Unknown node {name}"))?;
        let mut invpath = PathBuf::from(&r.nodes_path);
        invpath.push(npath);
        let ncontents = std::fs::read_to_string(invpath.canonicalize()?)?;

        meta.uri = format!("yaml_fs://{}", to_lexical_absolute(&invpath)?.display());

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

        // Resolve YAML merge keys in `params`
        let p = merge_keys_serde(serde_yaml::Value::from(n.params))?
            .as_mapping()
            .unwrap()
            .clone();
        n.params = p;

        // Convert serde_yaml::Mapping into our own Mapping type
        n.parameters = n.params.clone().into();

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

    /// Merges self into other, then updates self with merged values from other
    fn merge_into(&mut self, other: &mut Self) -> Result<()> {
        // We use std::mem::take() here so we can merge self.applications into other.applications
        // without having to call `clone()` twice. This doesn't destroy `self.applications` because
        // we update `self.applications` with the result of the merge immediately afterwards.
        let self_apps = std::mem::take(&mut self.applications);
        other.applications.merge(self_apps);
        self.applications = other.applications.clone();

        // We use std::mem::take() here so we can merge self.classes into other.classes without
        // having to call `clone()` twice. This doesn't destroy `self.classes` because we update
        // `self.classes` with the result of the merge immediately afterwards.
        let self_classes = std::mem::take(&mut self.classes);
        other.classes.merge(self_classes);
        self.classes = other.classes.clone();

        other.parameters.merge(&self.parameters)?;
        self.parameters = other.parameters.clone();
        Ok(())
    }

    /// Recursively loads classes and merges loaded data into self
    fn render_impl(&mut self, r: &Reclass, seen: &mut Vec<String>, root: &mut Node) -> Result<()> {
        for cls in self.classes.items_iter() {
            let cls = if cls.contains("${") {
                // Resolve any potential references if the class name contains an opening reference
                // symbol.
                let clstoken = Token::parse(&cls.clone())?;
                if let Some(clstoken) = clstoken {
                    // If we got a token, render it, and convert it into a string with
                    // `raw_string()` to ensure no spurious quotes are injected.
                    clstoken.render(&root.parameters)?.raw_string()?
                } else {
                    // If Token::parse() returns None, the class name can't contain any references,
                    // just convert cls into an owned String.
                    cls.to_string()
                }
            } else {
                // If the class name doesn't contain any opening reference symbols, it can't
                // contain any references, just convert cls into an owned String.
                cls.to_string()
            };

            // Check if we've seen the class already after resolving any references in the class
            // name.
            if seen.contains(&cls) {
                continue;
            }

            // Load class, respecting the `ignore_class_notfound` option
            let maybec = self.read_class(r, &cls);
            let Ok(Some(mut c)) = maybec else {
                if let Ok(None) = maybec {
                    eprintln!("ignore missing class {cls}");
                    continue;
                }
                return Err(maybec.unwrap_err());
            };

            // render class so we pick up further classes included in it
            c.render_impl(r, seen, root)?;
            // NOTE(sg): we don't need to merge here, since we've already mergeed into root as part
            // of the recursive call to `render_impl()`

            seen.push(cls.to_string());
        }

        // merge root into self, then update self with merged values
        self.merge_into(root)
    }

    /// Renders the Node's parameters by interpolating Reclass references and flattening
    /// ValueLists.
    fn render_parameters(&mut self) -> Result<()> {
        let p = std::mem::take(&mut self.parameters);
        let mut f = Value::Mapping(p);
        f.render_with_self()?;
        match f {
            Value::Mapping(m) => {
                self.parameters = m;
                Ok(())
            }
            _ => Err(anyhow!(
                "Rendered parameters are not a Mapping but a {}",
                f.variant()
            )),
        }
    }

    /// Load included classes (recursively), and merge parameters.
    ///
    /// Note that this method doesn't flatten overwritten parameters.
    pub fn render(&mut self, r: &Reclass) -> Result<()> {
        let mut base = Node {
            // NOTE(sg): We initialize a base node with our classes to start the class rendering
            // process.  This roughly corresponds to Python reclass's
            // `_get_class_mappings_entity()`.
            classes: self.classes.clone(),
            ..Default::default()
        };
        // NOTE(sg): We merge the `_reclass_` meta parameter into the base node before starting
        // class loading. This roughly corresponds to Python reclass's
        // `_get_automatic_parameters()`.
        base.parameters
            .insert("_reclass_".into(), self.meta.as_reclass().into())?;

        let mut seen = vec![];
        let mut root = Node::default();
        base.render_impl(r, &mut seen, &mut root)?;
        self.render_impl(r, &mut seen, &mut base)?;
        self.render_parameters()
    }
}

#[cfg(test)]
mod node_tests {
    use super::*;
    use crate::types::Value;
    use std::str::FromStr;

    fn make_reclass() -> Reclass {
        Reclass::new(
            "./tests/inventory/nodes",
            "./tests/inventory/classes",
            false,
        )
        .unwrap()
    }

    #[test]
    fn test_parse() {
        let r = make_reclass();
        let n = Node::parse(&r, "n1").unwrap();
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
        "#;
        let expected: serde_yaml::Mapping = serde_yaml::from_str(expected).unwrap();
        assert_eq!(n.parameters, expected.into());
    }

    #[test]
    #[should_panic(expected = "Unknown node n0")]
    fn test_parse_error() {
        let r = Reclass::new(
            "./tests/inventory/nodes",
            "./tests/inventory/classes",
            false,
        )
        .unwrap();
        Node::parse(&r, "n0").unwrap();
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
        assert_eq!(n.params, params);
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
        assert_eq!(n.params, expected);
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
        assert_eq!(n.params, expected);
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
        assert_eq!(n.params, expected);
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
        let r = make_reclass();
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
        let r = make_reclass();
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

    fn expected_params(nodename: &str, yaml: &str) -> Mapping {
        let reclass = Mapping::from_str(&format!(
            r#"
        _reclass_:
          environment: base
          name:
            short: {}
            parts: ["{}"]
            full: {}
            path: {}
            "#,
            nodename, nodename, nodename, nodename
        ))
        .unwrap();

        let mut expected = Mapping::from_str(yaml).unwrap();
        expected.merge(&reclass).unwrap();
        let mut expected = Value::Mapping(expected);
        expected.render(&Mapping::new()).unwrap();
        expected.as_mapping().unwrap().clone()
    }

    #[test]
    fn test_render_n1() {
        let r = make_reclass();
        let mut n = Node::parse(&r, "n1").unwrap();
        assert_eq!(
            n.classes,
            UniqueList::from(vec!["cls1".to_owned(), "cls2".to_owned()])
        );
        assert_eq!(
            n.applications,
            RemovableList::from(vec!["app1".to_owned(), "app2".to_owned()])
        );

        n.render(&r).unwrap();

        let expected = expected_params(
            "n1",
            r#"
        foo:
          foo: foo
          bar: cls2
          baz: cls1
        bar:
          foo: foo
        "#,
        );

        assert_eq!(n.parameters, expected);
    }

    #[test]
    fn test_render_n2() {
        let r = make_reclass();
        let mut n = Node::parse(&r, "n2").unwrap();
        assert_eq!(n.classes, UniqueList::from(vec!["nested.cls1".to_owned()]));
        assert_eq!(n.applications, RemovableList::from(vec![]));

        n.render(&r).unwrap();

        assert_eq!(
            n.classes,
            UniqueList::from(vec!["nested.cls2".to_owned(), "nested.cls1".to_owned()])
        );

        let expected = expected_params(
            "n2",
            r#"
        foo:
          foo: nested.cls1
          bar: n2
        bar: bar
        "#,
        );

        assert_eq!(n.parameters, expected);
    }

    #[test]
    fn test_render_n3() {
        let r = make_reclass();
        let mut n = Node::parse(&r, "n3").unwrap();
        n.render(&r).unwrap();

        let expected = expected_params(
            "n3",
            r#"
        cluster:
          name: c-test-cluster-1234
        openshift:
          infraID: c-test-cluster-1234-xlk3f
          clusterID: 2888efd2-8a1b-4846-82ec-3a99506e2c70
          baseDomain: c-test-cluster-1234.example.org
          appsDomain: apps.c-test-cluster-1234.example.org
          apiURL: api.c-test-cluster-1234.example.org
          ssh_key: ""
        "#,
        );

        assert_eq!(n.parameters, expected);
    }

    #[test]
    fn test_render_n4() {
        // Test case to cover class name with references
        let r = make_reclass();

        let mut n = Node::parse(&r, "n4").unwrap();
        n.render(&r).unwrap();
        assert_eq!(
            n.classes,
            UniqueList::from(vec!["cls8".into(), "${qux}".into(), "cls7".into()])
        );
        assert_eq!(n.applications, RemovableList::from(vec![]));

        let expected = expected_params(
            "n4",
            r#"
        foo:
          foo: cls1
          bar: cls1
          baz: cls1
        qux: cls1
        "#,
        );

        assert_eq!(n.parameters, expected);
    }

    #[test]
    fn test_render_n5() {
        let r = make_reclass();
        let n = r.render_node("n5").unwrap();

        assert_eq!(n.reclass.name, "n5");
        assert_eq!(n.classes, vec!["cls9".to_string(), "cls10".to_string()]);
        let expected = expected_params(
            "n5",
            r#"
        # from cls9
        foo: bar
        constant: foo
        foolist:
          - a
          - b
          - c
        # from cls10
        bar: bar
        target: n1
        "#,
        );
        assert_eq!(n.parameters, expected);
    }

    #[test]
    fn test_render_n6() {
        let r = make_reclass();
        let n = r.render_node("n6").unwrap();

        assert_eq!(n.reclass.name, "n6");
        assert_eq!(n.classes, vec!["cls9".to_string(), "cls11".to_string()]);

        let expected = expected_params(
            "n6",
            r#"
        # from cls9
        foo: baz
        constant: foo
        foolist:
          - a
          - b
          - c
        # from cls11
        baz: baz
        # interpolated in cls111
        fooer: baz
        target: n2
        bool: true
        int: 69
        float: 4.2
        "null": ~
        "#,
        );
        assert_eq!(n.parameters, expected);
    }

    #[test]
    fn test_render_n7() {
        let r = make_reclass();
        let n = r.render_node("n7").unwrap();

        assert_eq!(n.reclass.name, "n7");
        assert_eq!(n.classes, vec!["cls9".to_string()]);

        let expected = expected_params(
            "n7",
            r#"
            # from cls9
            foo: foo
            constant: foo
            # overwritten in n7
            foolist:
              - bar
            target: n3
            "#,
        );
        assert_eq!(n.parameters, expected);
    }

    #[test]
    fn test_render_n8() {
        let r = make_reclass();
        let n = r.render_node("n8").unwrap();

        assert_eq!(n.reclass.name, "n8");
        assert_eq!(
            n.classes,
            vec!["nested.a_sub".to_string(), "nested.a".to_string()]
        );
        let expected = expected_params(
            "n8",
            r#"
            # from nested.a
            foo: foo
            # from nested.a_sub via nested.a
            baz: baz
            target: n4
            "#,
        );
        assert_eq!(n.parameters, expected);
    }

    #[test]
    fn test_render_n9() {
        let r = make_reclass();
        let n = r.render_node("n9").unwrap();

        assert_eq!(n.reclass.name, "n9");
        // TODO(sg): check what the expected value here is
        assert_eq!(
            n.classes,
            vec![
                "cls9".to_string(),
                "cls10".to_string(),
                "cls12".to_string(),
                "nested.a_sub".to_string()
            ]
        );
        let expected = expected_params(
            "n9",
            r#"
            # from cls9 via cls13
            foo: foo
            constant: foo
            foolist:
              - a
              - b
              - c
            # from cls10 via cls13
            bar: bar
            # from nested.a_sub
            baz: baz
            target: n5
            "#,
        );

        dbg!(&n.parameters);
        assert_eq!(n.parameters, expected);
    }

    #[test]
    fn test_render_n10() {
        let r = make_reclass();
        let n = r.render_node("n10").unwrap();

        assert_eq!(n.reclass.name, "n10");
        // TODO(sg): check what the expected value here is
        assert_eq!(n.classes, vec!["cls9".to_string(), "nested.b".to_string()]);

        let expected = expected_params(
            "n10",
            r#"
        # from cls9 via nested.b
        foo: foo
        constant: foo
        foolist:
          - a
          - b
          - c
        target: n6
        "#,
        );

        assert_eq!(n.parameters, expected);
    }

    #[test]
    fn test_render_n11() {
        let r = make_reclass();
        let n = r.render_node("n11").unwrap();

        let expected = expected_params(
            "n11",
            r#"
            foo: foo
            bar:
              foo: "7"
              foo_int: 7
            target: n7
            target_int: n7
            "#,
        );
        assert_eq!(n.parameters, expected);
    }

    #[test]
    fn test_render_n12() {
        let r = make_reclass();
        let n = r.render_node("n12").unwrap();

        // # Parameters
        // from cls9
        let expected = expected_params(
            "n12",
            r#"
            foo: foo
            constant: foo
            foolist:
              - a
              - b
              - c
            target: n8
            "#,
        );
        assert_eq!(n.parameters, expected);

        // # Applications
        let apps: Vec<String> = n.applications.into();
        assert_eq!(apps, vec!["c".to_string(), "a".to_string()]);

        // # Classes
        let classes: Vec<String> = n.classes.into();
        assert_eq!(classes, vec!["cls9".to_string(), "app1".to_string()]);
    }

    #[test]
    fn test_render_n13() {
        let r = make_reclass();
        let n = r.render_node("n13").unwrap();

        // # Parameters
        // from cls9
        let expected = expected_params(
            "n13",
            r#"
            foo: foo
            constant: foo
            foolist:
              - a
              - b
              - c
            bar: bar
            target: n9
            "#,
        );
        assert_eq!(n.parameters, expected);

        // # Applications
        let apps: Vec<String> = n.applications.into();
        assert_eq!(
            apps,
            vec!["d".to_string(), "a".to_string(), "b".to_string()]
        );

        // # Classes
        let classes: Vec<String> = n.classes.into();
        assert_eq!(
            classes,
            vec!["cls10".to_string(), "cls9".to_string(), "app2".to_string()]
        );
    }

    #[test]
    fn test_render_n14() {
        let r = make_reclass();
        let n = r.render_node("n14").unwrap();

        // # Parameters
        let expected = expected_params(
            "n14",
            r#"
            foo: foo
            constant: foo
            foolist: [a, b, c]
            foodict:
              bar: bar
              baz: baz
            target: n10
            obj:
              bar: bar
              baz: baz
            list: [a, b, c]
            "#,
        );
        assert_eq!(n.parameters, expected);
    }

    #[test]
    fn test_render_n15() {
        let r = make_reclass();
        let n = r.render_node("n15").unwrap();

        // # Parameters
        let expected = expected_params(
            "n15",
            r#"
            cls9: cls9
            foo: foo
            constant: foo
            foolist:
              - a
              - b
              - c
            target: n11
            "#,
        );
        assert_eq!(n.parameters, expected);
    }

    #[test]
    fn test_render_n16() {
        let r = make_reclass();
        let n = r.render_node("n16").unwrap();

        // # Parameters
        let expected = expected_params(
            "n16",
            r#"
            tenant: foo
            cluster: bar
            foobar: ishere
            target: n12
            "#,
        );
        assert_eq!(n.parameters, expected);
    }

    #[test]
    fn test_render_n17() {
        let r = make_reclass();
        let n = r.render_node("n17").unwrap();

        // # Parameters
        // from ${baz}
        let expected = expected_params("n17", "{escaped: baz, target: n13}");
        assert_eq!(n.parameters, expected);
    }

    #[test]
    fn test_render_n18() {
        let r = Reclass::new("./tests/inventory/nodes", "./tests/inventory/classes", true).unwrap();
        let n = r.render_node("n18").unwrap();

        // # Parameters
        let expected = expected_params(
            "n18",
            r#"
        foo: foo
        constant: foo
        foolist:
          - a
          - b
          - c
        target: n14
        "#,
        );
        assert_eq!(n.parameters, expected);
    }

    #[test]
    fn test_render_n19() {
        let r = make_reclass();
        let n = r.render_node("n19").unwrap();

        let expected = expected_params(
            "n19",
            r#"
        dist: foo
        instanceref: cluster
        foo: fooing
        some: syn-cluster
        target: n15
        _instance: cluster
        "#,
        );

        assert_eq!(n.parameters, expected);
    }

    #[test]
    fn test_render_n20() {
        let r = make_reclass();
        let n = r.render_node("n20").unwrap();

        let expected = expected_params(
            "n20",
            r#"
            bardict: notadict
            bar: [a]
            foo:
              bar: barer
              baz: baz
              qux: qux
            foodict:
              bar: bar
              baz: baz
              qux: qux
            "#,
        );
        // # Parameters
        assert_eq!(n.parameters, expected);
    }

    #[test]
    fn test_render_n21() {
        let r = make_reclass();
        let n = r.render_node("n21").unwrap();

        let expected = expected_params(
            "n21",
            r#"
            foo:
              bar: bar
              baz: baz
            foo_merge:
              bar: bar
              baz: baz
              qux: qux
            bar_merge:
              bar: bar
              baz: baz
            "#,
        );
        assert_eq!(n.parameters, expected);
    }

    #[test]
    fn test_render_n22() {
        let r = make_reclass();
        let n = r.render_node("n22").unwrap();

        let expected = expected_params(
            "n22",
            r#"
            config_foo:
              bar: bar
              baz: [a, b]
            embedded:
              cfg1: |-
                qux: quxxing
                foo: {"bar":"bar","baz":["a","b"]}
              cfg2: |-
                qux: quxxer
                foo: {"bar":"bar","baz":["a","b"]}
            "#,
        );
        dbg!(&n.parameters);
        dbg!(&expected);
        assert_eq!(n.parameters, expected);
    }

    #[test]
    fn test_render_n23() {
        let r = make_reclass();
        let n = r.render_node("n23").unwrap();

        let expected = expected_params(
            "n23",
            r#"
            baz:
              baz: cls7
              foo: cls7
              qux: n19
            foo:
              bar: n19
              baz: baz
              qux: qux
            "#,
        );
        assert_eq!(n.parameters, expected);
    }
}
