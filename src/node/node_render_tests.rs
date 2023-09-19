use super::*;
use std::str::FromStr;

fn expected_params(nodename: &str, yaml: &str) -> Mapping {
    let reclass = Mapping::from_str(&format!(
        r#"
    _reclass_:
      environment: base
      name:
        short: {}
        parts: ["{}"]
        full: {}
        path: {}
        "#,
        nodename, nodename, nodename, nodename
    ))
    .unwrap();

    let mut expected = Mapping::from_str(yaml).unwrap();
    expected.merge(&reclass).unwrap();
    let mut expected = Value::Mapping(expected);
    expected.render(&Mapping::new()).unwrap();
    expected.as_mapping().unwrap().clone()
}

#[test]
fn test_render_n1() {
    let r = make_reclass();
    let mut n = Node::parse(&r, "n1").unwrap();
    assert_eq!(
        n.classes,
        UniqueList::from(vec!["cls1".to_owned(), "cls2".to_owned()])
    );
    assert_eq!(
        n.applications,
        RemovableList::from(vec!["app1".to_owned(), "app2".to_owned()])
    );

    n.render(&r).unwrap();

    let expected = expected_params(
        "n1",
        r#"
    foo:
      foo: foo
      bar: cls2
      baz: cls1
    bar:
      foo: foo
    "#,
    );

    assert_eq!(n.parameters, expected);
}

#[test]
fn test_render_n2() {
    let r = make_reclass();
    let mut n = Node::parse(&r, "n2").unwrap();
    assert_eq!(n.classes, UniqueList::from(vec!["nested.cls1".to_owned()]));
    assert_eq!(n.applications, RemovableList::from(vec![]));

    n.render(&r).unwrap();

    assert_eq!(
        n.classes,
        UniqueList::from(vec!["nested.cls2".to_owned(), "nested.cls1".to_owned()])
    );

    let expected = expected_params(
        "n2",
        r#"
    foo:
      foo: nested.cls1
      bar: n2
    bar: bar
    "#,
    );

    assert_eq!(n.parameters, expected);
}

#[test]
fn test_render_n3() {
    let r = make_reclass();
    let mut n = Node::parse(&r, "n3").unwrap();
    n.render(&r).unwrap();

    let expected = expected_params(
        "n3",
        r#"
    cluster:
      name: c-test-cluster-1234
    openshift:
      infraID: c-test-cluster-1234-xlk3f
      clusterID: 2888efd2-8a1b-4846-82ec-3a99506e2c70
      baseDomain: c-test-cluster-1234.example.org
      appsDomain: apps.c-test-cluster-1234.example.org
      apiURL: api.c-test-cluster-1234.example.org
      ssh_key: ""
    "#,
    );

    assert_eq!(n.parameters, expected);
}

#[test]
fn test_render_n4() {
    // Test case to cover class name with references
    let r = make_reclass();

    let mut n = Node::parse(&r, "n4").unwrap();
    n.render(&r).unwrap();
    assert_eq!(
        n.classes,
        UniqueList::from(vec!["cls8".into(), "${qux}".into(), "cls7".into()])
    );
    assert_eq!(n.applications, RemovableList::from(vec![]));

    let expected = expected_params(
        "n4",
        r#"
    foo:
      foo: cls1
      bar: cls1
      baz: cls1
    qux: cls1
    "#,
    );

    assert_eq!(n.parameters, expected);
}

#[test]
fn test_render_n5() {
    let r = make_reclass();
    let n = r.render_node("n5").unwrap();

    assert_eq!(n.reclass.name, "n5");
    assert_eq!(n.classes, vec!["cls9".to_string(), "cls10".to_string()]);
    let expected = expected_params(
        "n5",
        r#"
    # from cls9
    foo: bar
    =constant: foo
    foolist:
      - a
      - b
      - c
    # from cls10
    bar: bar
    target: n1
    "#,
    );
    assert_eq!(n.parameters, expected);
}

#[test]
fn test_render_n6() {
    let r = make_reclass();
    let n = r.render_node("n6").unwrap();

    assert_eq!(n.reclass.name, "n6");
    assert_eq!(n.classes, vec!["cls9".to_string(), "cls11".to_string()]);

    let expected = expected_params(
        "n6",
        r#"
    # from cls9
    foo: baz
    =constant: foo
    foolist:
      - a
      - b
      - c
    # from cls11
    baz: baz
    # interpolated in cls111
    fooer: baz
    target: n2
    bool: true
    int: 69
    float: 4.2
    "null": ~
    "#,
    );
    assert_eq!(n.parameters, expected);
}

#[test]
fn test_render_n7() {
    let r = make_reclass();
    let n = r.render_node("n7").unwrap();

    assert_eq!(n.reclass.name, "n7");
    assert_eq!(n.classes, vec!["cls9".to_string()]);

    let expected = expected_params(
        "n7",
        r#"
        # from cls9
        foo: foo
        =constant: foo
        # overwritten in n7
        foolist:
          - bar
        target: n3
        "#,
    );
    assert_eq!(n.parameters, expected);
}

#[test]
fn test_render_n8() {
    let r = make_reclass();
    let n = r.render_node("n8").unwrap();

    assert_eq!(n.reclass.name, "n8");
    assert_eq!(
        n.classes,
        vec!["nested.a_sub".to_string(), "nested.a".to_string()]
    );
    let expected = expected_params(
        "n8",
        r#"
        # from nested.a
        foo: foo
        # from nested.a_sub via nested.a
        baz: baz
        target: n4
        "#,
    );
    assert_eq!(n.parameters, expected);
}

#[test]
fn test_render_n9() {
    let r = make_reclass();
    let n = r.render_node("n9").unwrap();

    assert_eq!(n.reclass.name, "n9");
    // TODO(sg): check what the expected value here is
    assert_eq!(
        n.classes,
        vec![
            "cls9".to_string(),
            "cls10".to_string(),
            "cls12".to_string(),
            "nested.a_sub".to_string()
        ]
    );
    let expected = expected_params(
        "n9",
        r#"
        # from cls9 via cls13
        foo: foo
        =constant: foo
        foolist:
          - a
          - b
          - c
        # from cls10 via cls13
        bar: bar
        # from nested.a_sub
        baz: baz
        target: n5
        "#,
    );

    dbg!(&n.parameters);
    assert_eq!(n.parameters, expected);
}

#[test]
fn test_render_n10() {
    let r = make_reclass();
    let n = r.render_node("n10").unwrap();

    assert_eq!(n.reclass.name, "n10");
    // TODO(sg): check what the expected value here is
    assert_eq!(n.classes, vec!["cls9".to_string(), "nested.b".to_string()]);

    let expected = expected_params(
        "n10",
        r#"
    # from cls9 via nested.b
    foo: foo
    =constant: foo
    foolist:
      - a
      - b
      - c
    target: n6
    "#,
    );

    assert_eq!(n.parameters, expected);
}

#[test]
fn test_render_n11() {
    let r = make_reclass();
    let n = r.render_node("n11").unwrap();

    let expected = expected_params(
        "n11",
        r#"
        foo: foo
        bar:
          foo: "7"
          foo_int: 7
        target: n7
        target_int: n7
        "#,
    );
    assert_eq!(n.parameters, expected);
}

#[test]
fn test_render_n12() {
    let r = make_reclass();
    let n = r.render_node("n12").unwrap();

    // # Parameters
    // from cls9
    let expected = expected_params(
        "n12",
        r#"
        foo: foo
        =constant: foo
        foolist:
          - a
          - b
          - c
        target: n8
        "#,
    );
    assert_eq!(n.parameters, expected);

    // # Applications
    let apps: Vec<String> = n.applications.into();
    assert_eq!(apps, vec!["c".to_string(), "a".to_string()]);

    // # Classes
    let classes: Vec<String> = n.classes.into();
    assert_eq!(classes, vec!["cls9".to_string(), "app1".to_string()]);
}

#[test]
fn test_render_n13() {
    let r = make_reclass();
    let n = r.render_node("n13").unwrap();

    // # Parameters
    // from cls9
    let expected = expected_params(
        "n13",
        r#"
        foo: foo
        =constant: foo
        foolist:
          - a
          - b
          - c
        bar: bar
        target: n9
        "#,
    );
    assert_eq!(n.parameters, expected);

    // # Applications
    let apps: Vec<String> = n.applications.into();
    assert_eq!(
        apps,
        vec!["d".to_string(), "a".to_string(), "b".to_string()]
    );

    // # Classes
    let classes: Vec<String> = n.classes.into();
    assert_eq!(
        classes,
        vec!["cls10".to_string(), "cls9".to_string(), "app2".to_string()]
    );
}

#[test]
fn test_render_n14() {
    let r = make_reclass();
    let n = r.render_node("n14").unwrap();

    // # Parameters
    let expected = expected_params(
        "n14",
        r#"
        foo: foo
        =constant: foo
        foolist: [a, b, c]
        foodict:
          bar: bar
          baz: baz
        target: n10
        obj:
          bar: bar
          baz: baz
        list: [a, b, c]
        "#,
    );
    assert_eq!(n.parameters, expected);
}

#[test]
fn test_render_n15() {
    let r = make_reclass();
    let n = r.render_node("n15").unwrap();

    // # Parameters
    let expected = expected_params(
        "n15",
        r#"
        cls9: cls9
        foo: foo
        =constant: foo
        foolist:
          - a
          - b
          - c
        target: n11
        "#,
    );
    assert_eq!(n.parameters, expected);
}

#[test]
fn test_render_n16() {
    let r = make_reclass();
    let n = r.render_node("n16").unwrap();

    // # Parameters
    let expected = expected_params(
        "n16",
        r#"
        tenant: foo
        cluster: bar
        foobar: ishere
        target: n12
        "#,
    );
    assert_eq!(n.parameters, expected);
}

#[test]
fn test_render_n17() {
    let r = make_reclass();
    let n = r.render_node("n17").unwrap();

    // # Parameters
    // from ${baz}
    let expected = expected_params("n17", "{escaped: baz, target: n13}");
    assert_eq!(n.parameters, expected);
}

#[test]
fn test_render_n18() {
    let r = Reclass::new("./tests/inventory/nodes", "./tests/inventory/classes", true).unwrap();
    let n = r.render_node("n18").unwrap();

    // # Parameters
    let expected = expected_params(
        "n18",
        r#"
    foo: foo
    =constant: foo
    foolist:
      - a
      - b
      - c
    target: n14
    "#,
    );
    assert_eq!(n.parameters, expected);
}

#[test]
fn test_render_n19() {
    let r = make_reclass();
    let n = r.render_node("n19").unwrap();

    let expected = expected_params(
        "n19",
        r#"
    dist: foo
    instanceref: cluster
    foo: fooing
    some: syn-cluster
    target: n15
    _instance: cluster
    "#,
    );

    assert_eq!(n.parameters, expected);
}

#[test]
fn test_render_n20() {
    let r = make_reclass();
    let n = r.render_node("n20").unwrap();

    let expected = expected_params(
        "n20",
        r#"
        bardict: notadict
        bar: [a]
        foo:
          bar: barer
          baz: baz
          qux: qux
        foodict:
          bar: bar
          baz: baz
          qux: qux
        "#,
    );
    // # Parameters
    assert_eq!(n.parameters, expected);
}

#[test]
fn test_render_n21() {
    let r = make_reclass();
    let n = r.render_node("n21").unwrap();

    let expected = expected_params(
        "n21",
        r#"
        foo:
          bar: bar
          baz: baz
        foo_merge:
          bar: bar
          baz: baz
          qux: qux
        bar_merge:
          bar: bar
          baz: baz
        "#,
    );
    assert_eq!(n.parameters, expected);
}

#[test]
fn test_render_n22() {
    let r = make_reclass();
    let n = r.render_node("n22").unwrap();

    let expected = expected_params(
        "n22",
        r#"
        config_foo:
          bar: bar
          baz: [a, b]
        embedded:
          cfg1: |-
            qux: quxxing
            foo: {"bar":"bar","baz":["a","b"]}
          cfg2: |-
            qux: quxxer
            foo: {"bar":"bar","baz":["a","b"]}
        "#,
    );
    dbg!(&n.parameters);
    dbg!(&expected);
    assert_eq!(n.parameters, expected);
}

#[test]
fn test_render_n23() {
    let r = make_reclass();
    let n = r.render_node("n23").unwrap();

    let expected = expected_params(
        "n23",
        r#"
        ~baz:
          baz: cls7
          foo: cls7
          qux: n19
        foo:
          bar: n19
          baz: baz
          qux: qux
        "#,
    );
    assert_eq!(n.parameters, expected);
}

#[test]
fn test_render_n24() {
    let r = make_reclass();
    let n = r.render_node("n24").unwrap();

    let expected = expected_params(
        "n24",
        r#"
        fluentbit:
          config:
            inputs:
              systemd: {}
        "#,
    );
    assert_eq!(n.parameters, expected);
}
