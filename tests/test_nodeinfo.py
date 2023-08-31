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
        "foo": {"foo": "foo"},
        "bar": {"foo": "foo"},
    }
