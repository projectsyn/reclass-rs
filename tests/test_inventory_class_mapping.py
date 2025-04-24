import reclass_rs
import pytest
import sys

from test_with_kapicorp_reclass import prune_timestamps, py_reclass_inventory


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
            "/^test(?!.*-stg-test).*/  cluster.test",
            "/^test.*-stg-test.*/      cluster.staging",
            "/.*c$/                    class1 class2",
        ],
        "class_mappings_match_path": False,
    }
    c = reclass_rs.Config.from_dict("./tests/inventory-class-mapping", config_options)
    assert c is not None

    r = reclass_rs.Reclass.from_config(c)
    assert r is not None

    inv = r.inventory().as_dict()
    assert inv is not None
    prune_timestamps(inv)

    py_inv = py_reclass_inventory("./tests/inventory-class-mapping", config_options)

    assert inv == py_inv
