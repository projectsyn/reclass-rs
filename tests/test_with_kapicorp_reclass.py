import reclass_rs
import pytest
import sys


def fixup_pyfmt(v: str) -> str:
    # NOTE(sg): this is not generally applicable, it only works for the `embedded` parameter for
    # node n22 in `tests/inventory`.
    r = v.replace("'", '"')
    lines = r.splitlines()
    for i in range(0, len(lines)):
        parts = lines[i].split(": ", 1)
        parts[1] = parts[1].replace(" ", "")
        lines[i] = ": ".join(parts)

    return "\n".join(lines)


def prune_timestamps(inv: dict):
    del inv["__reclass__"]["timestamp"]
    for n in inv["nodes"].values():
        del n["__reclass__"]["timestamp"]


def py_reclass_inventory(inv_path: str, config: dict) -> dict:
    import reclass
    import reclass.core

    storage = reclass.get_storage(
        "yaml_fs",
        f"{inv_path}/nodes",
        f"{inv_path}/classes",
        config["compose_node_name"],
    )
    class_mappings = config.get("class_mappings")
    _reclass = reclass.core.Core(
        storage, class_mappings, reclass.settings.Settings(config)
    )
    py_inv = _reclass.inventory()
    # remove timestamps so we can compare the whole dicts
    prune_timestamps(py_inv)

    # ensure that kapicorp-reclass top-level `classes` and `applications` values are sorted lists
    for nodes in py_inv["classes"].values():
        nodes.sort()
    for nodes in py_inv["applications"].values():
        nodes.sort()

    return py_inv


@pytest.mark.skipif(
    sys.platform == "win32", reason="kapicorp-reclass not supported on Windows"
)
def test_inventory_matches_pyreclass():
    config_options = {
        "compose_node_name": False,
        "allow_none_override": True,
        "ignore_class_notfound": True,
    }
    c = reclass_rs.Config.from_dict("./tests/inventory", config_options)
    assert c is not None
    r = reclass_rs.Reclass.from_config(c)
    assert r is not None

    inv = r.inventory().as_dict()
    assert inv is not None
    prune_timestamps(inv)

    py_inv = py_reclass_inventory("./tests/inventory", config_options)
    # kapicorp-reclass hasn't fixed the applications merge bug yet,
    # cf.https://github.com/kapicorp/reclass/pull/9
    py_inv["nodes"]["n12"]["applications"].remove("b")
    py_inv["applications"]["b"].remove("n12")

    # complex values that are embedded in a string via reclass reference are formatted differently,
    # we use proper JSON serialization, while kapicorp-reclass just uses Python object notation
    # (via string formatting for the value).
    for k, v in py_inv["nodes"]["n22"]["parameters"]["embedded"].items():
        py_inv["nodes"]["n22"]["parameters"]["embedded"][k] = fixup_pyfmt(v)

    assert inv == py_inv


@pytest.mark.parametrize("compose", [False, True])
@pytest.mark.skipif(
    sys.platform == "win32", reason="kapicorp-reclass not supported on Windows"
)
def test_inventory_nested_nodes_matches_pyreclass(compose):
    config_options = {
        "compose_node_name": compose,
        "allow_none_override": True,
        "ignore_class_notfound": True,
    }
    c = reclass_rs.Config.from_dict("./tests/inventory-nested-nodes", config_options)
    assert c is not None
    r = reclass_rs.Reclass.from_config(c)
    assert r is not None

    inv = r.inventory().as_dict()
    assert inv is not None
    prune_timestamps(inv)

    py_inv = py_reclass_inventory("./tests/inventory-nested-nodes", config_options)

    assert inv == py_inv


@pytest.mark.skipif(
    sys.platform == "win32", reason="kapicorp-reclass not supported on Windows"
)
def test_inventory_compose_node_name_compat_matches_pyreclass():
    config_options = {
        "compose_node_name": True,
        "allow_none_override": True,
        "ignore_class_notfound": True,
        # NOTE(sg): We need the compose-node-name-literal-dots Python reclass compatibility mode
        # here, since the `compose-node-name` inventory has nodes that behave differently without
        # the compatibility mode.
        "reclass_rs_compat_flags": ["compose-node-name-literal-dots"],
    }
    c = reclass_rs.Config.from_dict(
        "./tests/inventory-compose-node-name", config_options
    )
    assert c is not None
    r = reclass_rs.Reclass.from_config(c)
    assert r is not None

    inv = r.inventory().as_dict()
    assert inv is not None
    prune_timestamps(inv)

    py_inv = py_reclass_inventory("./tests/inventory-compose-node-name", config_options)

    assert inv == py_inv
