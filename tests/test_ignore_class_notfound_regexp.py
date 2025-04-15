import pytest

import reclass_rs


def test_ignore_regexp_render_n1():
    r = reclass_rs.Reclass.from_config_file(
        "./tests/inventory-class-notfound-regexp", "reclass-config.yml"
    )
    assert r.config.ignore_class_notfound_regexp == ["service\\..*", ".*missing.*"]

    n1 = r.nodeinfo("n1")

    assert n1 is not None


def test_ignore_regexp_render_n2():
    r = reclass_rs.Reclass.from_config_file(
        "./tests/inventory-class-notfound-regexp", "reclass-config.yml"
    )
    assert r.config.ignore_class_notfound_regexp == ["service\\..*", ".*missing.*"]

    with pytest.raises(
        ValueError, match="Error while rendering n2: Class foo not found"
    ):
        n2 = r.nodeinfo("n2")


def test_ignore_regexp_update_config_render_n2():
    r = reclass_rs.Reclass.from_config_file(
        "./tests/inventory-class-notfound-regexp", "reclass-config.yml"
    )
    r.set_ignore_class_notfound_regexp([".*"])
    assert r.config.ignore_class_notfound_regexp == [".*"]

    n2 = r.nodeinfo("n2")
    assert n2 is not None


def test_ignore_regexp_from_dict():
    config_options = {
        "nodes_uri": "nodes",
        "classes_uri": "classes",
        "ignore_class_notfound": True,
        "ignore_class_notfound_regexp": ["service\\..*", ".*missing.*"],
    }
    c = reclass_rs.Config.from_dict(
        "./tests/inventory-class-notfound-regexp", config_options
    )
    r = reclass_rs.Reclass.from_config(c)

    n1 = r.nodeinfo("n1")
    assert n1 is not None

    with pytest.raises(
        ValueError, match="Error while rendering n2: Class foo not found"
    ):
        n2 = r.nodeinfo("n2")
