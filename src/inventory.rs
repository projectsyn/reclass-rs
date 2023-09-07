use anyhow::{anyhow, Result};
use chrono::Local;
use pyo3::prelude::*;
use pyo3::types::PyDict;
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
            .keys()
            .map(|name| (name, { r.render_node(name) }))
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
            false,
        )
        .unwrap();
        let inv = Inventory::render(&r).unwrap();

        // Check that all nodes are in `inv.nodes`. We do not verify the NodeInfos here, since we
        // have individual tests for each NodeInfo in `src/node`.
        let mut nodes = inv.nodes.keys().cloned().collect::<Vec<String>>();
        nodes.sort();
        assert_eq!(
            nodes,
            (1..=4).map(|n| format!("n{n}")).collect::<Vec<String>>()
        );

        // applications should contain app[1-2]
        let mut expected_applications = HashMap::<String, Vec<String>>::new();
        expected_applications.insert("app1".into(), vec!["n1".into()]);
        expected_applications.insert("app2".into(), vec!["n1".into()]);
        assert_eq!(inv.applications, expected_applications);

        // classes should contain:
        // * cls[1-8]
        // * ${qux} -- interpolated as cls1 for n4, but both Python reclass and our implementation
        // have the uninterpolated class name in the classes list.
        // * nested.cls[1-2]
        let mut expected_classes = HashMap::<String, Vec<String>>::new();
        expected_classes.insert("cls1".into(), vec!["n1".into()]);
        expected_classes.insert("cls2".into(), vec!["n1".into()]);
        expected_classes.insert("nested.cls1".into(), vec!["n2".into()]);
        expected_classes.insert("nested.cls2".into(), vec!["n2".into()]);
        expected_classes.insert("cls3".into(), vec!["n3".into()]);
        expected_classes.insert("cls4".into(), vec!["n3".into()]);
        expected_classes.insert("cls5".into(), vec!["n3".into()]);
        expected_classes.insert("cls6".into(), vec!["n3".into()]);
        expected_classes.insert("cls7".into(), vec!["n4".into()]);
        expected_classes.insert("cls8".into(), vec!["n4".into()]);
        expected_classes.insert("${qux}".into(), vec!["n4".into()]);

        assert_eq!(inv.classes, expected_classes);
    }
}
