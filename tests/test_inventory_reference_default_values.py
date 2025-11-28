import reclass_rs
import pytest
import sys


def test_inventory_reference_default_values():
    config_options = {
        "nodes_uri": "nodes",
        "classes_uri": "classes",
    }
    c = reclass_rs.Config.from_dict(
        "./tests/inventory-reference-default-values", config_options
    )
    assert c is not None

    r = reclass_rs.Reclass.from_config(c)
    assert r is not None

    inv = r.inventory().as_dict()

    n1_params = inv["nodes"]["n1"]["parameters"]
    n1_expected = n1_params["expected"]
    del n1_params["expected"]
    del n1_params["_reclass_"]

    assert n1_params == n1_expected

    n2_table = inv["nodes"]["n2"]["parameters"]["table"]
    n2_expected = inv["nodes"]["n2"]["parameters"]["expected_table"]
    assert n2_table == n2_expected

    n3_table = inv["nodes"]["n3"]["parameters"]["table"]
    n3_expected = inv["nodes"]["n3"]["parameters"]["expected_table"]
    assert n3_table == n3_expected

    n4_data = inv["nodes"]["n4"]["parameters"]["data"]
    n4_expected = inv["nodes"]["n4"]["parameters"]["expected_data"]
    assert n4_data == n4_expected

    n5_data = inv["nodes"]["n5"]["parameters"]["data"]
    n5_expected = inv["nodes"]["n5"]["parameters"]["expected_data"]
    assert n5_data == n5_expected

    n6 = inv["nodes"]["n6"]["parameters"]
    expected = n6["expected"]
    assert n6["direct"] == expected
    assert n6["default"] == expected
    assert n6["defaultref"] == expected

    n7 = inv["nodes"]["n7"]["parameters"]
    assert n7["data"] == n7["expected"]

    n8 = inv["nodes"]["n8"]["parameters"]
    assert n8["data"] == n8["expected"]

    n9 = inv["nodes"]["n9"]["parameters"]
    assert n9["compile"] == n9["_compile"]["jsonnet"]

    n10 = inv["nodes"]["n10"]["parameters"]
    assert n10["compile"] == n10["_compile"]["helm"]

    n11 = inv["nodes"]["n11"]["parameters"]
    assert n11["text"] == n11["expected"]
