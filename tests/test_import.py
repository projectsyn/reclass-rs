import reclass_rs


def test_import_new():
    r = reclass_rs.Reclass()
    assert r is not None
    assert r.nodes_path == "./inventory/nodes"
    assert r.classes_path == "./inventory/classes"
    assert not r.ignore_class_notfound


def test_import_new_args():
    r = reclass_rs.Reclass("./test/targets", "./test/classes", True)
    assert r is not None
    assert r.nodes_path == "./test/targets"
    assert r.classes_path == "./test/classes"
    assert r.ignore_class_notfound
