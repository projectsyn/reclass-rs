import reclass_rs

from pathlib import Path


def test_nodeinfo_n1():
    r = reclass_rs.Reclass(
        nodes_path="./tests/inventory/nodes", classes_path="./tests/inventory/classes"
    )
    n = r.nodeinfo("n1")
    npath = Path("./tests/inventory/nodes/n1.yml").resolve()
    assert n.__reclass__.uri == f"yaml_fs://{npath}"
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


def test_nodeinfo_n1_meta_symlink():
    r = reclass_rs.Reclass(
        nodes_path="./tests/inventory/targets", classes_path="./tests/inventory/classes"
    )
    n = r.nodeinfo("n1")
    npath = Path("./tests/inventory/targets/n1.yml").absolute()
    assert n.__reclass__.uri == f"yaml_fs://{npath}"


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


def test_nodeinfo_n3():
    r = reclass_rs.Reclass(
        nodes_path="./tests/inventory/nodes", classes_path="./tests/inventory/classes"
    )
    n = r.nodeinfo("n3")
    assert n.applications == []
    assert n.classes == ["cls4", "cls5", "cls6", "cls3"]
    assert n.parameters == {
        "_reclass_": {
            "environment": "base",
            "name": {
                "full": "n3",
                "parts": ["n3"],
                "path": "n3",
                "short": "n3",
            },
        },
        "cluster": {"name": "c-test-cluster-1234"},
        "openshift": {
            "infraID": "c-test-cluster-1234-xlk3f",
            "clusterID": "2888efd2-8a1b-4846-82ec-3a99506e2c70",
            "baseDomain": "c-test-cluster-1234.example.org",
            "appsDomain": "apps.c-test-cluster-1234.example.org",
            "apiURL": "api.c-test-cluster-1234.example.org",
            "ssh_key": "",
        },
    }


def test_nodeinfo_n4():
    r = reclass_rs.Reclass(
        nodes_path="./tests/inventory/nodes", classes_path="./tests/inventory/classes"
    )
    n = r.nodeinfo("n4")
    assert n.applications == []
    assert n.classes == ["cls8", "${qux}", "cls7"]
    assert n.parameters == {
        "_reclass_": {
            "environment": "base",
            "name": {
                "full": "n4",
                "parts": ["n4"],
                "path": "n4",
                "short": "n4",
            },
        },
        "qux": "cls1",
        "foo": {"foo": "cls1", "bar": "cls1", "baz": "cls1"},
    }
