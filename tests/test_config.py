import reclass_rs
import pytest
import pathlib


def test_config_from_dict():
    config_options = {
        "nodes_uri": "targets",
        "classes_uri": "classes",
        "ignore_class_notfound": False,
        "compose_node_name": False,
    }
    c = reclass_rs.Config.from_dict("./tests/inventory", config_options)
    assert c is not None

    assert not c.ignore_class_notfound
    assert not c.compose_node_name

    expected_nodes_path = pathlib.Path("./tests/inventory/targets")
    expected_classes_path = pathlib.Path("./tests/inventory/classes")
    assert pathlib.Path(c.nodes_path) == expected_nodes_path
    assert pathlib.Path(c.classes_path) == expected_classes_path


def test_config_from_dict_non_default():
    config_options = {
        "nodes_uri": "targets",
        "classes_uri": "classes",
        "ignore_class_notfound": True,
        "ignore_class_notfound_regexp": ["foo", "bar"],
        "compose_node_name": True,
    }
    c = reclass_rs.Config.from_dict("./tests/inventory", config_options)
    assert c is not None

    assert c.ignore_class_notfound
    assert c.compose_node_name

    expected_nodes_path = pathlib.Path("./tests/inventory/targets")
    expected_classes_path = pathlib.Path("./tests/inventory/classes")
    assert pathlib.Path(c.nodes_path) == expected_nodes_path
    assert pathlib.Path(c.classes_path) == expected_classes_path

    assert c.ignore_class_notfound_regexp == ["foo", "bar"]
