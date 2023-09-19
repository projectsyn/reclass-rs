import reclass_rs

expected_classes = {
    "${cls9}": ["n15"],
    "${tenant}.${cluster}": ["n16"],
    "\\${baz}": ["n17"],
    "${qux}": ["n4"],
    "app1": ["n12"],
    "app2": ["n13"],
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

expected_nodes = set([f"n{i}" for i in range(1, 25)])


def test_inventory():
    r = reclass_rs.Reclass(
        nodes_path="./tests/inventory/nodes",
        classes_path="./tests/inventory/classes",
        ignore_class_notfound=True,
    )
    inv = r.inventory()

    assert set(inv.nodes.keys()) == expected_nodes

    assert inv.classes == expected_classes

    assert inv.applications == expected_applications


def test_inventory_as_dict():
    r = reclass_rs.Reclass(
        nodes_path="./tests/inventory/nodes",
        classes_path="./tests/inventory/classes",
        ignore_class_notfound=True,
    )
    inv = r.inventory().as_dict()

    assert set(inv["nodes"].keys()) == expected_nodes

    assert inv["classes"] == expected_classes

    assert inv["applications"] == expected_applications

    assert "__reclass__" in inv
    assert set(inv["__reclass__"].keys()) == set(["timestamp"])
