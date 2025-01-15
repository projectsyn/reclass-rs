import pytest
import reclass_rs


def test_no_compose_node_name_compat_collision():
    config = reclass_rs.Config.from_dict(
        inventory_path="./tests/inventory-compose-node-name",
        config={"reclass_rs_compat_flags": ["compose-node-name-literal-dots"]},
    )
    assert config.compatflags == {reclass_rs.CompatFlag.ComposeNodeNameLiteralDots}
    with pytest.raises(ValueError) as exc:
        r = reclass_rs.Reclass.from_config(config)

    assert "Error while discovering nodes: Definition of node '1'" in str(exc)


def test_no_compose_node_name_compat():
    r = reclass_rs.Reclass(inventory_path="./tests/inventory-nested-nodes")
    r.set_compat_flag(reclass_rs.CompatFlag.ComposeNodeNameLiteralDots)
    assert not r.config.compose_node_name
    assert r.config.compatflags == {reclass_rs.CompatFlag.ComposeNodeNameLiteralDots}

    inv = r.inventory()

    assert set(inv.nodes.keys()) == {"a1", "b1", "c1", "d1"}

    a1 = inv.nodes["a1"].parameters["_reclass_"]["name"]
    assert a1["full"] == "a1"
    assert a1["parts"] == ["a1"]
    assert a1["path"] == "a1"
    assert a1["short"] == "a1"


def test_no_compose_node_name():
    r = reclass_rs.Reclass(inventory_path="./tests/inventory-nested-nodes")
    assert not r.config.compose_node_name
    assert r.config.compatflags == set()

    inv = r.inventory()

    assert set(inv.nodes.keys()) == {"a1", "b1", "c1", "d1"}

    a1 = inv.nodes["a1"].parameters["_reclass_"]["name"]
    assert a1["full"] == "a1"
    assert a1["parts"] == ["a1"]
    assert a1["path"] == "a1"
    assert a1["short"] == "a1"


def test_compose_node_name_compat():
    r = reclass_rs.Reclass.from_config_file(
        "./tests/inventory-compose-node-name", "reclass-config.yml"
    )
    r.set_compat_flag(reclass_rs.CompatFlag.ComposeNodeNameLiteralDots)
    assert r.config.compose_node_name
    assert r.config.compatflags == {reclass_rs.CompatFlag.ComposeNodeNameLiteralDots}

    inv = r.inventory()

    assert set(inv.nodes.keys()) == {
        "a.1",
        "a",
        "b.1",
        "c.1",
        "c._c.1",
        "d",
        "d1",
        "d2",
    }

    a1 = inv.nodes["a.1"].parameters["_reclass_"]["name"]
    assert a1["full"] == "a.1"
    assert a1["parts"] == ["a", "1"]
    assert a1["path"] == "a/1"
    assert a1["short"] == "1"


def test_compose_node_name():
    r = reclass_rs.Reclass.from_config_file(
        "./tests/inventory-compose-node-name", "reclass-config.yml"
    )
    assert r.config.compose_node_name
    assert r.config.compatflags == set()

    inv = r.inventory()

    assert set(inv.nodes.keys()) == {
        "a.1",
        "a",
        "b.1",
        "c.1",
        "c._c.1",
        "d",
        "d1",
        "d2",
    }

    a1 = inv.nodes["a.1"].parameters["_reclass_"]["name"]
    assert a1["full"] == "a.1"
    assert a1["parts"] == ["a.1"]
    assert a1["path"] == "a.1"
    assert a1["short"] == "a.1"
