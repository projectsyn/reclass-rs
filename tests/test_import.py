import reclass_rs
import pytest
import os


def test_import_new():
    r = reclass_rs.Reclass(
        "./tests/inventory/targets", "./tests/inventory/classes", True
    )
    assert r is not None
    assert r.nodes_path == "./tests/inventory/targets"
    assert r.classes_path == "./tests/inventory/classes"
    assert r.ignore_class_notfound


def test_import_raises():
    with pytest.raises(ValueError) as exc:
        r = reclass_rs.Reclass("./foo", "./bar")

    assert "Error while discovering nodes" in str(exc.value)
