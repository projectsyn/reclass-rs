import reclass_rs

expected_classes = {
    "${cls9}": ["n15"],
    "${tenant}.${cluster}": ["n16"],
    "\\${baz}": ["n17"],
    "${qux}": ["n4"],
    "app1": ["n12"],
    "app2": ["n13"],
    "bar": ["n25"],
    "cls1": ["n1"],
    "cls2": ["n1"],
    "cls3": ["n3"],
    "cls4": ["n3"],
    "cls5": ["n3"],
    "cls6": ["n3"],
    "cls7": ["n4"],
    "cls8": ["n4"],
    "cls9": [
        "n10",
        "n12",
        "n13",
        "n14",
        "n18",
        "n5",
        "n6",
        "n7",
        "n9",
    ],
    "cls9_meta": ["n15"],
    "cls10": ["n13", "n5", "n9"],
    "cls11": ["n6"],
    "cls12": ["n9"],
    "cls13": ["n14"],
    "cls14": ["n23"],
    "cls15": ["n23"],
    "cluster.${dist}": ["n19"],
    "cluster.default": ["n19"],
    "cluster.facts": ["n19"],
    "cluster.global": ["n19"],
    "config": ["n16"],
    "defaults": ["n24"],
    "foo": ["n25"],
    "foo-indirect": ["n20"],
    "meta": ["n24"],
    "nested.a": ["n8"],
    "nested.a_sub": ["n8", "n9"],
    "nested.b": ["n10"],
    "nested.cls1": ["n2"],
    "nested.cls2": ["n2"],
    "nonexisting": ["n18"],
    "override": ["n24"],
    "yaml-anchor": ["n21"],
}

expected_applications = {
    "app1": ["n1"],
    "app2": ["n1"],
    "a": ["n12", "n13"],
    "b": ["n13"],
    "c": ["n12"],
    "d": ["n13"],
}

expected_nodes = {f"n{i}" for i in range(1, 26)}


def test_inventory():
    r = reclass_rs.Reclass.from_config_file("./tests/inventory", "reclass-config.yml")

    assert set(r.nodes.keys()) == expected_nodes
    included_classes = set(expected_classes.keys())
    all_classes = set(r.classes.keys())
    # discovered classes which aren't shown with their resolved names in the output:
    assert all_classes - included_classes == {
        "${baz}",  # appears as `\\${baz}`
        "cluster.foo",  # appears as `cluster.${dist}`
        "config_symlink",  # doesn't appear
        "foo.bar",  # appears as `${tenant}.${cluster}`
    }
    # class includes which aren't shown in their resolved form:
    assert included_classes - all_classes == {
        "${cls9}",  # resolved as `cls9` for n15
        "${qux}",  # resolved as `cls1` for n4
        "${tenant}.${cluster}",  # resolved as `foo.bar` for n16
        "\\${baz}",  # resolved as `${baz}` for n17
        "cluster.${dist}",  # resolved as `cluster.foo` for n19
        "nonexisting",  # skipped because ignore_class_notfound=True
    }

    inv = r.inventory()

    assert set(inv.nodes.keys()) == expected_nodes

    assert inv.classes == expected_classes

    assert inv.applications == expected_applications


def test_inventory_as_dict():
    r = reclass_rs.Reclass(
        inventory_path="./tests/inventory",
        ignore_class_notfound=True,
    )
    inv = r.inventory().as_dict()

    assert set(inv["nodes"].keys()) == expected_nodes

    assert inv["classes"] == expected_classes

    assert inv["applications"] == expected_applications

    assert "__reclass__" in inv
    assert set(inv["__reclass__"].keys()) == set(["timestamp"])


def test_reclass_from_config():
    config_options = {
        "nodes_uri": "targets",
        "classes_uri": "classes",
        "ignore_class_notfound": True,
        "compose_node_name": True,
    }
    c = reclass_rs.Config.from_dict("./tests/inventory", config_options)
    assert c is not None

    r = reclass_rs.Reclass.from_config(c)
    assert r is not None

    assert set(r.nodes.keys()) == expected_nodes
    included_classes = set(expected_classes.keys())
    all_classes = set(r.classes.keys())
    # discovered classes which aren't shown with their resolved names in the output:
    assert all_classes - included_classes == {
        "${baz}",  # appears as `\\${baz}`
        "cluster.foo",  # appears as `cluster.${dist}`
        "config_symlink",  # doesn't appear
        "foo.bar",  # appears as `${tenant}.${cluster}`
    }
    # class includes which aren't shown in their resolved form:
    assert included_classes - all_classes == {
        "${cls9}",  # resolved as `cls9` for n15
        "${qux}",  # resolved as `cls1` for n4
        "${tenant}.${cluster}",  # resolved as `foo.bar` for n16
        "\\${baz}",  # resolved as `${baz}` for n17
        "cluster.${dist}",  # resolved as `cluster.foo` for n19
        "nonexisting",  # skipped because ignore_class_notfound=True
    }
