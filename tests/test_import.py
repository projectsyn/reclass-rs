import reclass_rs
import pytest
import os
import pathlib


def test_import_new():
    r = reclass_rs.Reclass(
        "./tests/inventory",
        "targets",
        "classes",
        True,
    )
    invpath = pathlib.Path("./tests/inventory")
    assert r is not None
    assert r.config is not None
    assert pathlib.Path(r.config.inventory_path) == invpath
    assert pathlib.Path(r.config.nodes_path) == invpath / "targets"
    assert pathlib.Path(r.config.classes_path) == invpath / "classes"
    assert r.config.ignore_class_notfound

    assert r.nodes is not None
    assert r.classes is not None


def test_import_raises():
    with pytest.raises(ValueError) as exc:
        r = reclass_rs.Reclass("./inventory", "foo", "bar")

    assert "Error while discovering nodes" in str(exc.value)
