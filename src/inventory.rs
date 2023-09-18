use anyhow::{anyhow, Result};
use chrono::Local;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use rayon::prelude::*;
use std::collections::HashMap;

use super::{NodeInfo, Reclass};

#[pyclass]
#[derive(Debug, Default)]
pub struct Inventory {
    /// Maps each application which is included by at least one node to the list of nodes which
    /// include it.
    #[pyo3(get)]
    applications: HashMap<String, Vec<String>>,
    /// Maps each class which is included by at least one node to the list of nodes which include
    /// it.
    #[pyo3(get)]
    classes: HashMap<String, Vec<String>>,
    /// Maps each node name discovered by `Reclass::discover_nodes()` to its `NodeInfo`.
    #[pyo3(get)]
    nodes: HashMap<String, NodeInfo>,
}

impl Inventory {
    /// Renders the full inventory for the given Reclass config.
    pub fn render(r: &Reclass) -> Result<Self> {
        // Render all nodes
        let infos: Vec<_> = r
            .nodes
            .par_iter()
            .map(|(name, _)| (name, { r.render_node(name) }))
            .collect();

        // Generate `Inventory` from the rendered nodes
        let mut inv = Self::default();
        for (name, info) in infos {
            let info = info.map_err(|e| anyhow!("Error rendering node {name}: {e}"))?;
            for cls in &info.classes {
                inv.classes
                    .entry(cls.clone())
                    .and_modify(|nodes: &mut Vec<String>| nodes.push(name.clone()))
                    .or_insert(vec![name.clone()]);
            }
            for app in &info.applications {
                inv.applications
                    .entry(app.clone())
                    .and_modify(|nodes: &mut Vec<String>| nodes.push(name.clone()))
                    .or_insert(vec![name.clone()]);
            }
            // Ensure application and classes values are sorted. We need to consume the iterator,
            // but we don't care about the vec of unit types which results from calling sort on the
            // values_mut() elements, so we directly drop the resulting Vec.
            drop(
                inv.classes
                    .values_mut()
                    .map(|v| v.sort())
                    .collect::<Vec<()>>(),
            );
            drop(
                inv.applications
                    .values_mut()
                    .map(|v| v.sort())
                    .collect::<Vec<()>>(),
            );
            inv.nodes.insert(name.clone(), info);
        }
        Ok(inv)
    }
}

#[pymethods]
impl Inventory {
    /// Returns the Inventory as a Python dict.
    ///
    /// The structure of the returned dict should match Python reclass the structure of the dict
    /// returned by Python reclass's `inventory()` method.
    fn as_dict(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new(py);
        dict.set_item("applications", self.applications.clone().into_py(py))?;
        dict.set_item("classes", self.classes.clone().into_py(py))?;
        let nodes_dict = PyDict::new(py);
        for (name, info) in &self.nodes {
            nodes_dict.set_item(name, info.as_dict(py)?)?;
        }
        dict.set_item("nodes", nodes_dict)?;

        let reclass_dict = PyDict::new(py);
        let ts = Local::now();
        reclass_dict.set_item("timestamp", ts.format("%c").to_string())?;
        dict.set_item("__reclass__", reclass_dict)?;
        Ok(dict.into())
    }
}

#[cfg(test)]
mod inventory_tests {
    use super::*;

    #[test]
    fn test_render() {
        let r = Reclass::new(
            "./tests/inventory/nodes",
            "./tests/inventory/classes",
            // n18 includes a nonexistent class
            true,
        )
        .unwrap();
        let inv = Inventory::render(&r).unwrap();

        // Check that all nodes are in `inv.nodes`. We do not verify the NodeInfos here, since we
        // have individual tests for each NodeInfo in `src/node`.
        let mut nodes = inv.nodes.keys().cloned().collect::<Vec<String>>();
        nodes.sort();

        let mut expected_nodes = (1..=24).map(|n| format!("n{n}")).collect::<Vec<String>>();
        expected_nodes.sort();

        assert_eq!(nodes, expected_nodes);

        // applications should contain app[1-2], [a-d]
        let mut expected_applications = HashMap::<String, Vec<String>>::new();
        expected_applications.insert("app1".into(), vec!["n1".into()]);
        expected_applications.insert("app2".into(), vec!["n1".into()]);
        expected_applications.insert("a".into(), vec!["n12".into(), "n13".into()]);
        expected_applications.insert("b".into(), vec!["n13".into()]);
        expected_applications.insert("c".into(), vec!["n12".into()]);
        expected_applications.insert("d".into(), vec!["n13".into()]);

        assert_eq!(inv.applications, expected_applications);

        // classes should match the hash map defined below.
        // Note that classes with parameter references are tracked unrendered and the rendered
        // variants aren't added to the classes list for the node. Here's the expected
        // rendered values:
        // * ${cls9} -- rendered as cls9 for n15
        // * ${qux} -- rendered as cls1 for n4
        // * ${tenant}.${cluster} -- rendered as foo.bar for n16
        // * \${baz} -- rendered as `${baz}` for n17
        // * cluster.${dist} -- rendered as cluster.foo for n19

        let mut expected_classes = HashMap::<String, Vec<String>>::new();
        expected_classes.insert("${cls9}".into(), vec!["n15".into()]);
        expected_classes.insert("${qux}".into(), vec!["n4".into()]);
        expected_classes.insert("${tenant}.${cluster}".into(), vec!["n16".into()]);
        expected_classes.insert("\\${baz}".into(), vec!["n17".into()]);
        expected_classes.insert("app1".into(), vec!["n12".into()]);
        expected_classes.insert("app2".into(), vec!["n13".into()]);
        expected_classes.insert("cls1".into(), vec!["n1".into()]);
        expected_classes.insert("cls2".into(), vec!["n1".into()]);
        expected_classes.insert("cls3".into(), vec!["n3".into()]);
        expected_classes.insert("cls4".into(), vec!["n3".into()]);
        expected_classes.insert("cls5".into(), vec!["n3".into()]);
        expected_classes.insert("cls6".into(), vec!["n3".into()]);
        expected_classes.insert("cls7".into(), vec!["n4".into()]);
        expected_classes.insert("cls8".into(), vec!["n4".into()]);
        expected_classes.insert(
            "cls9".into(),
            vec![
                "n10".into(),
                "n12".into(),
                "n13".into(),
                "n14".into(),
                "n18".into(),
                "n5".into(),
                "n6".into(),
                "n7".into(),
                "n9".into(),
            ],
        );
        expected_classes.insert("cls9_meta".into(), vec!["n15".into()]);
        expected_classes.insert("cls10".into(), vec!["n13".into(), "n5".into(), "n9".into()]);
        expected_classes.insert("cls11".into(), vec!["n6".into()]);
        expected_classes.insert("cls12".into(), vec!["n9".into()]);
        expected_classes.insert("cls13".into(), vec!["n14".into()]);
        expected_classes.insert("cls14".into(), vec!["n23".into()]);
        expected_classes.insert("cls15".into(), vec!["n23".into()]);
        expected_classes.insert("cluster.${dist}".into(), vec!["n19".into()]);
        expected_classes.insert("cluster.default".into(), vec!["n19".into()]);
        expected_classes.insert("cluster.facts".into(), vec!["n19".into()]);
        expected_classes.insert("cluster.global".into(), vec!["n19".into()]);
        expected_classes.insert("config".into(), vec!["n16".into()]);
        expected_classes.insert("defaults".into(), vec!["n24".into()]);
        expected_classes.insert("foo-indirect".into(), vec!["n20".into()]);
        expected_classes.insert("meta".into(), vec!["n24".into()]);
        expected_classes.insert("nested.a".into(), vec!["n8".into()]);
        expected_classes.insert("nested.a_sub".into(), vec!["n8".into(), "n9".into()]);
        expected_classes.insert("nested.b".into(), vec!["n10".into()]);
        expected_classes.insert("nested.cls1".into(), vec!["n2".into()]);
        expected_classes.insert("nested.cls2".into(), vec!["n2".into()]);
        expected_classes.insert("nonexisting".into(), vec!["n18".into()]);
        expected_classes.insert("override".into(), vec!["n24".into()]);
        expected_classes.insert("yaml-anchor".into(), vec!["n21".into()]);

        assert_eq!(inv.classes, expected_classes);
    }
}
