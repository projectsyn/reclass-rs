use super::*;

use std::str::FromStr;

impl Mapping {
    pub(super) fn render(&self, root: &Self) -> Result<Self> {
        let mut state = ResolveState::default();
        self.interpolate(root, &mut state)
    }
}

fn sequence_literal(v: Vec<Value>) -> Value {
    let mut state = ResolveState::default();
    Value::Sequence(v)
        .interpolate(&Mapping::new(), &mut state)
        .unwrap()
}

fn mapping_literal(m: Mapping) -> Value {
    Value::Mapping(m.render(&Mapping::new()).unwrap())
}

#[test]
fn test_extend_sequence() {
    let mut p = Mapping::new();
    p.insert(
        Value::String("l".into()),
        Value::Sequence(vec!["a".into(), "b".into(), "c".into()]),
    )
    .unwrap();
    let mut o = Mapping::new();
    o.insert(Value::String("l".into()), Value::Sequence(vec!["d".into()]))
        .unwrap();

    p.merge(&o).unwrap();
    p = p.render(&p).unwrap();

    assert_eq!(
        p.get(&"l".into()).unwrap(),
        &sequence_literal(vec!["a".into(), "b".into(), "c".into(), "d".into()])
    );
}

#[test]
fn test_override_sequence() {
    let mut p = Mapping::new();
    p.insert(
        Value::String("l".into()),
        Value::Sequence(vec!["a".into(), "b".into(), "c".into()]),
    )
    .unwrap();
    let mut o = Mapping::new();
    o.insert(
        Value::String("~l".into()),
        Value::Sequence(vec!["d".into()]),
    )
    .unwrap();

    p.merge(&o).unwrap();
    p = p.render(&p).unwrap();

    assert_eq!(
        p.get(&"l".into()).unwrap(),
        &sequence_literal(vec!["d".into()])
    );
}

#[test]
fn test_extend_mapping() {
    let mut p = Mapping::new();
    let mut m = Mapping::new();
    m.insert(Value::String("a".into()), Value::Bool(true))
        .unwrap();
    p.insert(Value::String("m".into()), Value::Mapping(m))
        .unwrap();

    let mut o = Mapping::new();
    let mut n = Mapping::new();
    n.insert(Value::String("b".into()), Value::Bool(true))
        .unwrap();
    o.insert(Value::String("m".into()), Value::Mapping(n))
        .unwrap();

    let mut r = Mapping::new();
    r.insert(Value::String("a".into()), Value::Bool(true))
        .unwrap();
    r.insert(Value::String("b".into()), Value::Bool(true))
        .unwrap();

    p.merge(&o).unwrap();
    p = p.render(&p).unwrap();

    assert_eq!(p.get(&"m".into()).unwrap(), &Value::Mapping(r));
}

#[test]
fn test_override_mapping() {
    let mut p = Mapping::new();
    let mut m = Mapping::new();
    m.insert(Value::String("a".into()), Value::Bool(true))
        .unwrap();
    p.insert(Value::String("m".into()), Value::Mapping(m))
        .unwrap();

    let mut o = Mapping::new();
    let mut n = Mapping::new();
    n.insert(Value::String("b".into()), Value::Bool(true))
        .unwrap();
    o.insert(Value::String("~m".into()), Value::Mapping(n.clone()))
        .unwrap();

    p.merge(&o).unwrap();
    p = p.render(&p).unwrap();

    assert_eq!(p.get(&"m".into()).unwrap(), &Value::Mapping(n));
}

#[test]
#[should_panic(expected = "Can't overwrite constant key \"c\"")]
fn test_constant_param_overwrite_panics() {
    let mut p = Mapping::new();

    let mut n = Mapping::new();
    n.insert(Value::String("=c".into()), Value::String("p".into()))
        .unwrap();

    p.merge(&n).unwrap();

    let mut o = Mapping::new();
    o.insert(Value::String("c".into()), Value::String("o".into()))
        .unwrap();

    p.merge(&o).unwrap();
}

#[test]
fn test_embedded_ref() {
    let mut p = Mapping::new();
    let mut m = Mapping::new();
    m.insert(Value::String("foo".into()), Value::String("foo".into()))
        .unwrap();
    m.insert(Value::String("bar".into()), Value::String("bar".into()))
        .unwrap();
    m.insert(
        Value::String("foobar1".into()),
        Value::String("${foo}bar".into()),
    )
    .unwrap();
    m.insert(
        Value::String("foobar2".into()),
        Value::String("foo${bar}".into()),
    )
    .unwrap();
    m.insert(
        Value::String("foobar3".into()),
        Value::String("${foo}${bar}".into()),
    )
    .unwrap();
    m.insert(
        Value::String("baz".into()),
        Value::String("${foo}-${bar}-baz".into()),
    )
    .unwrap();

    p.merge(&m).unwrap();
    p = p.render(&p).unwrap();

    assert_eq!(p.get(&"foo".into()).unwrap(), &Value::Literal("foo".into()));
    assert_eq!(
        p.get(&"foobar1".into()).unwrap(),
        &Value::Literal("foobar".into())
    );
    assert_eq!(
        p.get(&"foobar2".into()).unwrap(),
        &Value::Literal("foobar".into())
    );
    assert_eq!(
        p.get(&"foobar3".into()).unwrap(),
        &Value::Literal("foobar".into())
    );
    assert_eq!(
        p.get(&"baz".into()).unwrap(),
        &Value::Literal("foo-bar-baz".into())
    );
}

#[test]
fn test_ref_in_sequence() {
    let mut p = Mapping::new();
    let mut m = Mapping::new();
    m.insert(Value::String("foo".into()), Value::String("foo".into()))
        .unwrap();
    m.insert(Value::String("bar".into()), Value::String("bar".into()))
        .unwrap();
    m.insert(
        Value::String("list".into()),
        Value::Sequence(vec!["${foo}".into(), "${bar}".into()]),
    )
    .unwrap();

    p.merge(&m).unwrap();
    p = p.render(&p).unwrap();

    assert_eq!(p.get(&"foo".into()).unwrap(), &Value::Literal("foo".into()));
    assert_eq!(p.get(&"bar".into()).unwrap(), &Value::Literal("bar".into()));
    assert_eq!(
        p.get(&"list".into()).unwrap(),
        &sequence_literal(vec!["foo".into(), "bar".into()])
    );
}

#[test]
fn test_nested_ref() {
    let mut p = Mapping::new();
    let mut m = Mapping::new();
    let mut mm = Mapping::new();
    mm.insert(
        Value::String("bar".into()),
        Value::String("nested-bar".into()),
    )
    .unwrap();
    m.insert(Value::String("foo".into()), Value::Mapping(mm))
        .unwrap();
    m.insert(Value::String("bar".into()), Value::String("bar".into()))
        .unwrap();
    m.insert(
        Value::String("ref".into()),
        Value::String("${foo:${bar}}".into()),
    )
    .unwrap();

    p.merge(&m).unwrap();
    p = p.render(&p).unwrap();

    assert_eq!(
        p.get(&"ref".into()).unwrap(),
        &Value::Literal("nested-bar".into())
    );
}

#[test]
fn test_merge_over_ref() {
    let mut p = Mapping::new();
    let base = r#"
    foodict:
      bar: bar
      baz: baz
      qux: qux
    foo: ${foodict}"#;
    let base = Mapping::from_str(base).unwrap();
    p.merge(&base).unwrap();

    let overlay = r#"
    foo:
      bar: barer"#;
    let overlay = Mapping::from_str(overlay).unwrap();
    p.merge(&overlay).unwrap();

    p = p.render(&p).unwrap();
    dbg!(&p);

    let merged_foo = r#"
    bar: barer
    baz: baz
    qux: qux"#;
    let merged_foo = Mapping::from_str(merged_foo).unwrap();
    dbg!(&merged_foo);

    assert_eq!(p.get(&"foo".into()).unwrap(), &mapping_literal(merged_foo));
}

#[test]
fn test_merge_over_ref_nested() {
    let mut p = Mapping::new();
    let base = r#"
    foodict:
      bar: bar
      baz: baz
      qux: qux
    some:
      foo: ${foodict}"#;
    let base = Mapping::from_str(base).unwrap();
    p.merge(&base).unwrap();

    let overlay = r#"
    some:
      foo:
        bar: barer"#;
    let overlay = Mapping::from_str(overlay).unwrap();

    p.merge(&overlay).unwrap();
    p = p.render(&p).unwrap();

    let merged_some = r#"
    foo:
      bar: barer
      baz: baz
      qux: qux"#;
    let merged_some = Mapping::from_str(merged_some).unwrap();

    assert_eq!(
        p.get(&"some".into()).unwrap(),
        &mapping_literal(merged_some)
    );
}

#[test]
fn test_merge_over_null() {
    let mut p = Mapping::new();
    let base = r#"
    foodict:
      bar: bar
      baz: baz
      qux: qux
    some:
      foo: null"#;
    let base = Mapping::from_str(base).unwrap();
    p.merge(&base).unwrap();

    let overlay = r#"
    some:
      foo:
        bar: barer"#;
    let overlay = Mapping::from_str(overlay).unwrap();

    p.merge(&overlay).unwrap();
    p = p.render(&p).unwrap();

    let merged_some = r#"
    foo:
      bar: barer"#;
    let merged_some = Mapping::from_str(merged_some).unwrap();

    assert_eq!(
        p.get(&"some".into()).unwrap(),
        &mapping_literal(merged_some)
    );
}

#[test]
fn test_merge_null() {
    let mut p = Mapping::new();
    let base = r#"
    some:
      foo:
        bar: bar
        baz: baz
        qux: qux"#;
    let base = Mapping::from_str(base).unwrap();
    p.merge(&base).unwrap();

    let overlay = r#"
    some:
      foo: null"#;
    let overlay = Mapping::from_str(overlay).unwrap();

    p.merge(&overlay).unwrap();
    p = p.render(&p).unwrap();

    let merged_some = r#"
    foo: null"#;
    let merged_some = Mapping::from_str(merged_some).unwrap();

    assert_eq!(
        p.get(&"some".into()).unwrap(),
        &mapping_literal(merged_some)
    );
}

#[test]
fn test_merge_interpolate_embedded_nested_ref() {
    let mut p = Mapping::new();
    let base = r#"
    foo:
      bar:
        baz: baz
        qux: qux
    bar:
      foo:
        release-1.21: foo-1.22
        release-1.22: foo-1.22
        release-1.23: foo-1.22
    "#;
    let base = Mapping::from_str(base).unwrap();
    p.merge(&base).unwrap();

    let config1 = r#"
    version: release-1.21
    foo:
      bar:
        baz: baz-${bar:foo:${version}}
    "#;
    let config1 = Mapping::from_str(config1).unwrap();
    p.merge(&config1).unwrap();

    let config2 = r#"
    version: release-${dynamic:major}.${dynamic:minor}
    dynamic:
      major: "1"
      minor: "22"
    "#;
    let config2 = Mapping::from_str(config2).unwrap();
    p.merge(&config2).unwrap();
    p = p.render(&p).unwrap();

    let val = p
        .get(&"foo".into())
        .unwrap()
        .get(&"bar".into())
        .unwrap()
        .get(&"baz".into())
        .unwrap();
    assert_eq!(val, &Value::Literal("baz-foo-1.22".into()));
}

#[test]
fn test_interpolate_duplicate_ref_no_loop() {
    let base = r#"
    foo:
      bar: ${baz}-${baz}
    baz: baz
    "#;
    let base = Mapping::from_str(base).unwrap();

    let p = base.render(&base).unwrap();

    let expected = Mapping::from_str("{foo: {bar: baz-baz}, baz: baz}").unwrap();
    let expected = expected.render(&Mapping::new()).unwrap();
    assert_eq!(p, expected);
}

#[test]
fn test_interpolate_sequence_duplicate_ref_no_loop() {
    let base = r#"
    foo:
      bar:
      - ${baz}
      - ${baz}
    baz: baz
    "#;
    let base = Mapping::from_str(base).unwrap();

    let p = base.render(&base).unwrap();

    let expected = Mapping::from_str("{foo: {bar: [baz, baz]}, baz: baz}").unwrap();
    let expected = expected.render(&Mapping::new()).unwrap();
    assert_eq!(p, expected);
}

#[test]
fn test_interpolate_nested_mapping_no_loop() {
    let base = r#"
    foo:
      bar:
        baz: ${foo:baz:bar}
        qux: foo
      baz:
        bar: qux
        qux: ${foo:bar:qux}
    "#;
    let base = Mapping::from_str(base).unwrap();

    let p = base.render(&base).unwrap();

    let expected =
        Mapping::from_str("{foo: {bar: {baz: qux, qux: foo}, baz: {bar: qux, qux: foo}}}").unwrap();
    let expected = expected.render(&Mapping::new()).unwrap();
    assert_eq!(p, expected);
}

#[test]
#[should_panic(expected = "While resolving references in \
    {\"foo\": {\"bar\": \"${bar}\"}, \"bar\": [{\"baz\": \"baz\", \"qux\": \"qux\"}, \
    {\"baz\": \"${foo}\"}]}: Reference loop with reference paths [\"bar\", \"foo\"].")]
fn test_merge_interpolate_loop() {
    let base = r#"
    foo:
      bar: ${bar}
    bar:
      baz: baz
      qux: qux
    "#;
    let base = Mapping::from_str(base).unwrap();
    let config1 = r#"
    bar:
      baz: ${foo}
    "#;
    let config1 = Mapping::from_str(config1).unwrap();

    let mut p = Mapping::new();
    p.merge(&base).unwrap();
    p.merge(&config1).unwrap();

    let mut v = Value::from(p);
    v.render_with_self().unwrap();
}

#[test]
#[should_panic(expected = "While resolving references in \
     {\"foo\": {\"bar\": [\"${bar}\", \"${baz}\"]}, \"bar\": \"${qux}\", \
     \"baz\": {\"bar\": \"${foo}\"}, \"qux\": 3.14}: \
    Reference loop with reference paths [\"baz\", \"foo\"].")]
fn test_interpolate_sequence_loop() {
    let base = r#"
    foo:
      bar:
      - ${bar}
      - ${baz}
    bar: ${qux}
    baz:
      bar: ${foo}
    qux: 3.14
    "#;
    let base = Mapping::from_str(base).unwrap();

    let mut v = Value::from(base);
    v.render_with_self().unwrap();
}

#[test]
#[should_panic(expected = "While resolving references in \
    {\"foo\": {\"bar\": {\"baz\": \"${foo:baz:bar}\", \"qux\": \"${foo:qux:foo}\"}, \
    \"baz\": {\"bar\": \"qux\", \"qux\": \"${foo:bar:qux}\"}, \"qux\": \
    {\"foo\": \"${foo:baz:qux}\"}}}: \
    Reference loop with reference paths [\"foo:bar:qux\", \"foo:baz:qux\", \"foo:qux:foo\"].")]
fn test_interpolate_nested_mapping_loop() {
    let m = r#"
    foo:
      bar:
        baz: ${foo:baz:bar}
        qux: ${foo:qux:foo}
      baz:
        bar: qux
        qux: ${foo:bar:qux}
      qux:
        foo: ${foo:baz:qux}
    "#;
    let m = Mapping::from_str(m).unwrap();

    let mut v = Value::from(m);
    v.render_with_self().unwrap();
}

#[test]
#[should_panic(
    expected = "While resolving references in \"${foo:${foo:${foo:${foo:${foo:\
        ${foo:${foo:${foo:${foo:${foo:${foo:${foo:${foo:${foo:${foo:${foo:${foo:\
        ${foo:${foo:${foo:${foo:${foo:${foo:${foo:${foo:${foo:${foo:${foo:${foo:\
        ${foo:${foo:${foo:${foo:${foo:${foo:${foo:${foo:${foo:${foo:${foo:${foo:\
        ${foo:${foo:${foo:${foo:${foo:${foo:${foo:${foo:${foo:${foo:${foo:${foo:\
        ${foo:${foo:${foo:${foo:${foo:${foo:${foo:${foo:${foo:${foo:${foo:${foo:\
        ${foo:${foo:${foo:${foo:${foo:${foo}}}}}}}}}}}}}}}}}}}}}}}}}}}}}}}}}}}}}\
        }}}}}}}}}}}}}}}}}}}}}}}}}}}}}}}}}}\": \
        Token resolution exceeded recursion depth of 64. \
        We've seen the following reference paths: []."
)]
fn test_interpolate_depth_exceeded() {
    // construct a reference string which is a nested sequence of ${foo:....${foo}} with 70 nesting
    // levels. Note that the expected error has an empty list of reference paths because we hit the
    // recursion limit before we even manage to construct the initial ref path in
    // `Token::resolve()`.
    let refstr = (0..70).fold("${foo}".to_string(), |s, _| format!("${{foo:{s}}}"));
    let map = (0..70).fold(Mapping::from_str("foo: bar").unwrap(), |m, _| {
        let mut n = Mapping::new();
        n.insert("foo".into(), Value::Mapping(m)).unwrap();
        n
    });
    let v = Value::from(refstr);
    v.rendered(&map).unwrap();
}
