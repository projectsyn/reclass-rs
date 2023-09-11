import reclass_rs


def test_nodeinfo_n1():
    r = reclass_rs.Reclass(
        nodes_path="./tests/inventory/nodes", classes_path="./tests/inventory/classes"
    )
    n = r.nodeinfo("n1")
    assert n.applications == ["app1", "app2"]
    assert n.classes == ["cls1", "cls2"]
    assert n.parameters == {
        "_reclass_": {
            "environment": "base",
            "name": {
                "full": "n1",
                "parts": ["n1"],
                "path": "n1",
                "short": "n1",
            },
        },
        "foo": {"foo": "foo", "bar": "cls2", "baz": "cls1"},
        "bar": {"foo": "foo"},
    }


def test_nodeinfo_n2():
    r = reclass_rs.Reclass(
        nodes_path="./tests/inventory/nodes", classes_path="./tests/inventory/classes"
    )
    n = r.nodeinfo("n2")
    assert n.applications == []
    assert n.classes == ["nested.cls2", "nested.cls1"]
    assert n.parameters == {
        "_reclass_": {
            "environment": "base",
            "name": {
                "full": "n2",
                "parts": ["n2"],
                "path": "n2",
                "short": "n2",
            },
        },
        "foo": {"foo": "nested.cls1", "bar": "n2"},
        "bar": "bar",
    }
