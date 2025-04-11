import reclass_rs
import pytest
import sys


@pytest.mark.parametrize(
    "compose_node_name,class_mappings_match_path",
    [(False, False), (True, False), (False, True), (True, True)],
)
@pytest.mark.skipif(
    sys.platform == "win32", reason="kapicorp-reclass not supported on Windows"
)
def test_inventory_class_mappings(compose_node_name, class_mappings_match_path):
    import reclass
    import reclass.core

    config_options = {
        "nodes_uri": "nodes",
        "classes_uri": "classes",
        "compose_node_name": True,
        "class_mappings": [
            "\\*              common",
            "*                defaults",
            "test/*           cluster.test",
            "production/*     cluster.production",
            "test.*           composed.test",
            "production.*     composed.production",
            "/(test|production)\\/.*/ regex.params regex.\\\\1",
        ],
        "class_mappings_match_path": False,
    }
    c = reclass_rs.Config.from_dict("./tests/inventory-class-mapping", config_options)
    assert c is not None

    r = reclass_rs.Reclass.from_config(c)
    assert r is not None

    inv = r.inventory().as_dict()
    assert inv is not None

    # delete timestamps from resulting dict to ensure that we don't run into issues when comparing
    # two dicts that are rendered at slightly different times
    del inv["__reclass__"]["timestamp"]
    for n in inv["nodes"].keys():
        del inv["nodes"][n]["__reclass__"]["timestamp"]

    storage = reclass.get_storage(
        "yaml_fs",
        "./tests/inventory-class-mapping/nodes",
        "./tests/inventory-class-mapping/classes",
        config_options["compose_node_name"],
    )
    class_mappings = config_options.get("class_mappings")
    _reclass = reclass.core.Core(
        storage, class_mappings, reclass.settings.Settings(config_options)
    )
    py_inv = _reclass.inventory()

    # delete timestamps from resulting dict to ensure that we don't run into issues when comparing
    # two dicts that are rendered at slightly different times
    del py_inv["__reclass__"]["timestamp"]
    for n in py_inv["nodes"].keys():
        del py_inv["nodes"][n]["__reclass__"]["timestamp"]

    for nodes in py_inv["classes"].values():
        nodes.sort()

    for nodes in py_inv["applications"].values():
        nodes.sort()

    assert inv == py_inv
