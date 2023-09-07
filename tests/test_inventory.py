import reclass_rs


def test_inventory():
    r = reclass_rs.Reclass(
        nodes_path="./tests/inventory/nodes", classes_path="./tests/inventory/classes"
    )
    inv = r.inventory()

    assert set(inv.nodes.keys()) == set(["n1", "n2", "n3", "n4"])

    expected_classes = {
        "cls1": ["n1"],
        "cls2": ["n1"],
        "cls3": ["n3"],
        "cls4": ["n3"],
        "cls5": ["n3"],
        "cls6": ["n3"],
        "cls7": ["n4"],
        "cls8": ["n4"],
        "${qux}": ["n4"],
        "nested.cls1": ["n2"],
        "nested.cls2": ["n2"],
    }

    assert inv.classes == expected_classes

    expected_applications = {
        "app1": ["n1"],
        "app2": ["n1"],
    }

    assert inv.applications == expected_applications


def test_inventory_as_dict():
    r = reclass_rs.Reclass(
        nodes_path="./tests/inventory/nodes", classes_path="./tests/inventory/classes"
    )
    inv = r.inventory().as_dict()

    assert set(inv["nodes"].keys()) == set(["n1", "n2", "n3", "n4"])

    expected_classes = {
        "cls1": ["n1"],
        "cls2": ["n1"],
        "cls3": ["n3"],
        "cls4": ["n3"],
        "cls5": ["n3"],
        "cls6": ["n3"],
        "cls7": ["n4"],
        "cls8": ["n4"],
        "${qux}": ["n4"],
        "nested.cls1": ["n2"],
        "nested.cls2": ["n2"],
    }

    assert inv["classes"] == expected_classes

    expected_applications = {
        "app1": ["n1"],
        "app2": ["n1"],
    }

    assert inv["applications"] == expected_applications

    assert "__reclass__" in inv
    assert set(inv["__reclass__"].keys()) == set(["timestamp"])
