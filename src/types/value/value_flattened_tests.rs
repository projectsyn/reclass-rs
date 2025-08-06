use super::*;
use paste::paste;
use std::str::FromStr;

#[test]
fn test_flattened_null() {
    assert_eq!(
        Value::Null(None)
            .flattened(&mut ResolveState::default())
            .unwrap(),
        Value::Null(None)
    );
}

macro_rules! test_flattened_simple {
    ($($variant:expr,$val:expr,$expected:expr ),*) => {
        $(
            paste! {
            #[test]
            fn [<test_flattened_simple_ $variant:snake>]() {
                let v = Value::$variant($val, None);
                let f = v.flattened(&mut ResolveState::default()).unwrap();
                assert_eq!(f, $expected);
            }
            }
        )*
    }
}

test_flattened_simple! {
    Bool,true,Value::Bool(true, None),
    Number,5.into(),Value::Number(5.into(), None),
    Literal,"foo".into(),Value::Literal("foo".into(), None),
    Sequence,vec![Value::Bool(true, None), 3.14.into()],Value::Sequence(vec![Value::Bool(true, None), 3.14.into()], None),
    Mapping,Mapping::from_str("{foo: true, bar: 3.14}").unwrap(),Value::Mapping(Mapping::from_str("{foo: true, bar: 3.14}").unwrap(), None)
}

#[test]
#[should_panic(
    expected = "In test: Can't flatten unparsed String, did you mean to call `rendered()`?"
)]
fn test_flattened_string() {
    let v = Value::String("foo".into(), None);
    let mut st = ResolveState::default();
    st.push_mapping_key(&"test".into()).unwrap();
    v.flattened(&mut st).unwrap();
}

#[test]
fn test_flattened_nested_mapping() {
    let m = Value::Mapping(
        Mapping::from_str("{foo: {foo: foo, bar: bar}, bar: bar}").unwrap(),
        None,
    );
    let f = m.rendered(&Mapping::new()).unwrap();
    let mut foo = Mapping::new();
    foo.insert("foo".into(), Value::Literal("foo".to_string(), None))
        .unwrap();
    foo.insert("bar".into(), Value::Literal("bar".to_string(), None))
        .unwrap();
    let mut expected = Mapping::new();
    expected.insert("foo".into(), foo.into()).unwrap();
    expected
        .insert("bar".into(), Value::Literal("bar".to_string(), None))
        .unwrap();
    let expected = Value::Mapping(expected, None);
    assert_eq!(f, expected);
}

#[test]
fn test_flattened_simple_value_list() {
    let v = Value::ValueList(
        vec![
            Value::Literal("foo".into(), None),
            Value::Literal("bar".into(), None),
        ],
        None,
    );
    let f = v.flattened(&mut ResolveState::default()).unwrap();
    assert!(f.is_literal());
    assert_eq!(f, Value::Literal("bar".into(), None));
}

#[test]
fn test_flattened_mixed_value_list() {
    let v = Value::ValueList(
        vec![
            Value::Number(3.14.into(), None),
            Value::Null(None),
            Value::Literal("bar".into(), None),
        ],
        None,
    );
    let f = v.flattened(&mut ResolveState::default()).unwrap();
    assert!(f.is_literal());
    assert_eq!(f, Value::Literal("bar".into(), None));
}

#[test]
fn test_flattened_sequence_value_list() {
    let v = Value::ValueList(
        vec![
            Value::Sequence(vec!["foo".into(), "bar".into()], None),
            Value::Sequence(vec!["baz".into(), "qux".into()], None),
            Value::Sequence(vec!["foo".into()], None),
        ],
        None,
    );
    let f = v.flattened(&mut ResolveState::default()).unwrap();
    assert_eq!(
        f,
        Value::Sequence(
            vec![
                "foo".into(),
                "bar".into(),
                "baz".into(),
                "qux".into(),
                "foo".into()
            ],
            None
        )
    );
}

#[test]
fn test_flattened_mapping_value_list() {
    let v = Value::ValueList(
        vec![
            Mapping::from_str("{foo: {foo: foo, bar: bar}, bar: bar}")
                .unwrap()
                .into(),
            Mapping::from_str("{baz: baz, qux: qux}").unwrap().into(),
        ],
        None,
    );
    let f = v.flattened(&mut ResolveState::default()).unwrap();
    assert!(f.is_mapping());

    let m: serde_yaml::Mapping = f.as_mapping().unwrap().clone().into();
    let expected =
        serde_yaml::from_str("{foo: {foo: foo, bar: bar}, bar: bar, baz: baz, qux: qux}").unwrap();
    assert_eq!(m, expected);
}

#[test]
fn test_flattened_null_over_mapping() {
    let v = Value::ValueList(
        vec![
            Mapping::from_str("{foo: {foo: foo, bar: bar}, bar: bar}")
                .unwrap()
                .into(),
            Value::Null(None),
        ],
        None,
    );
    let f = v.flattened(&mut ResolveState::default()).unwrap();
    assert!(f.is_null());
    assert_eq!(f, Value::Null(None));
}

#[test]
fn test_flattened_null_over_sequence() {
    let v = Value::ValueList(
        vec![
            Value::Sequence(vec!["foo".into(), "bar".into()], None),
            Value::Null(None),
        ],
        None,
    );
    let f = v.flattened(&mut ResolveState::default()).unwrap();
    assert!(f.is_null());
    assert_eq!(f, Value::Null(None));
}

#[test]
fn test_flattened_map_over_sequence_error() {
    let v = Value::ValueList(
        vec![
            Value::Sequence(vec!["foo".into(), "bar".into()], None),
            Value::Mapping(Mapping::from_str("foo: foo").unwrap(), None),
        ],
        None,
    );
    let f = v.flattened(&mut ResolveState::default());
    assert!(f.is_err());
}

#[test]
fn test_flattened_map_over_simple_value_error() {
    let v = Value::ValueList(
        vec![
            Value::Bool(true, None),
            Value::Mapping(Mapping::from_str("foo: foo").unwrap(), None),
        ],
        None,
    );
    let f = v.flattened(&mut ResolveState::default());
    assert!(f.is_err());
}

#[test]
fn test_flattened_sequence_over_map_error() {
    let v = Value::ValueList(
        vec![
            Value::Mapping(Mapping::from_str("foo: foo").unwrap(), None),
            Value::Sequence(vec!["foo".into(), "bar".into()], None),
        ],
        None,
    );
    let f = v.flattened(&mut ResolveState::default());
    assert!(f.is_err());
}

#[test]
fn test_flattened_sequence_over_simple_value_error() {
    let v = Value::ValueList(
        vec![
            Value::Bool(true, None),
            Value::Sequence(vec!["foo".into(), "bar".into()], None),
        ],
        None,
    );
    let f = v.flattened(&mut ResolveState::default());
    assert!(f.is_err());
}

#[test]
fn test_flattened_nested_mapping_value_list() {
    // preprocess the valuelist entries by calling render() on each entry to ensure we've
    // transformed all `Value::String()` to `Value::Literal()`.
    let v = Value::ValueList(
        vec![
            Mapping::from_str("foo: {foo: {foo: foo}}")
                .unwrap()
                .render(&Mapping::new())
                .unwrap()
                .into(),
            Mapping::from_str("foo: {foo: {foo: bar}}")
                .unwrap()
                .render(&Mapping::new())
                .unwrap()
                .into(),
            Mapping::from_str("foo: {foo: {bar: bar}}")
                .unwrap()
                .render(&Mapping::new())
                .unwrap()
                .into(),
            Mapping::from_str("foo: {bar: {bar: bar}}")
                .unwrap()
                .render(&Mapping::new())
                .unwrap()
                .into(),
        ],
        None,
    );
    // We use `.rendered()` instead of `.flattened()` here since we can't flatten arbitrary Values
    // anymore without interpolating them first.
    let f = v.rendered(&Mapping::new()).unwrap();
    assert!(f.is_mapping());
    let m: serde_yaml::Mapping = f.as_mapping().unwrap().clone().into();
    let expected =
        serde_yaml::from_str("foo: {foo: {foo: bar, bar: bar}, bar: {bar: bar}}").unwrap();
    assert_eq!(m, expected);
}

#[test]
fn test_flattened_nested_mapping_value_list_2() {
    // preprocess the valuelist entries by calling render() on each entry to ensure we've
    // transformed all `Value::String()` to `Value::Literal()`.
    let v = Value::ValueList(
        vec![
            Mapping::from_str("qux: {foo: {foo: {foo: foo}}}")
                .unwrap()
                .render(&Mapping::new())
                .unwrap()
                .into(),
            Mapping::from_str("qux: {foo: {foo: {foo: bar}}}")
                .unwrap()
                .render(&Mapping::new())
                .unwrap()
                .into(),
            Mapping::from_str("qux: {foo: {foo: {bar: bar}}}")
                .unwrap()
                .render(&Mapping::new())
                .unwrap()
                .into(),
            Mapping::from_str("qux: {foo: {bar: {bar: bar}}}")
                .unwrap()
                .render(&Mapping::new())
                .unwrap()
                .into(),
            Mapping::from_str("qux: {bar: {bar: {bar: bar}}}")
                .unwrap()
                .render(&Mapping::new())
                .unwrap()
                .into(),
        ],
        None,
    );
    // We use `.rendered()` instead of `.flattened()` here since we can't flatten arbitrary Values
    // anymore without interpolating them first.
    let f = v.rendered(&Mapping::new()).unwrap();
    assert!(f.is_mapping());
    let m: serde_yaml::Mapping = f.as_mapping().unwrap().clone().into();
    let expected = serde_yaml::from_str(
        "qux: {foo: {foo: {foo: bar, bar: bar}, bar: {bar: bar}}, bar: {bar: {bar: bar}}}",
    )
    .unwrap();
    assert_eq!(m, expected);
}

#[test]
fn test_flattened_nested_mapping_value_list_3() {
    let mut base = Mapping::from_str("qux: {foo: {foo: foo}}").unwrap();
    let m1 = Mapping::from_str("foo: [foo, bar, baz]").unwrap();
    let m2 = Mapping::from_str("{foo: [qux], qux: {foo: {bar: bar}}}").unwrap();
    let m3 = Mapping::from_str("qux: {foo: {foo: qux}}").unwrap();
    let m4 = Mapping::from_str("qux: {foo: {bar: baz}}").unwrap();
    base.merge(&m1).unwrap();
    base.merge(&m2).unwrap();
    base.merge(&m3).unwrap();
    base.merge(&m4).unwrap();

    // We use `.rendered()` instead of `.flattened()` here since we can't flatten arbitrary Values
    // anymore without interpolating them first.
    let f = Value::Mapping(dbg!(base), None)
        .rendered(&Mapping::new())
        .unwrap();
    assert!(f.is_mapping());
    let m: serde_yaml::Mapping = f.as_mapping().unwrap().clone().into();
    let expected =
        serde_yaml::from_str("{foo: [foo, bar, baz, qux], qux: {foo: {foo: qux, bar: baz}}}")
            .unwrap();
    assert_eq!(m, expected);
}

#[test]
fn test_flatten_value_list() {
    // smoke test for in-place flattening, see the various `test_flattened_` tests for more
    // comprehensive tests of the actual flattening logic.
    //
    // Test input is copied from test_flattened_nested_mapping_value_list_3.
    let mut base = Mapping::from_str("qux: {foo: {foo: foo}}").unwrap();
    let m1 = Mapping::from_str("foo: [foo, bar, baz]").unwrap();
    let m2 = Mapping::from_str("{foo: [qux], qux: {foo: {bar: bar}}}").unwrap();
    let m3 = Mapping::from_str("qux: {foo: {foo: qux}}").unwrap();
    let m4 = Mapping::from_str("qux: {foo: {bar: baz}}").unwrap();
    base.merge(&m1).unwrap();
    base.merge(&m2).unwrap();
    base.merge(&m3).unwrap();
    base.merge(&m4).unwrap();

    let mut v = Value::Mapping(base, None);
    v.render(&Mapping::new()).unwrap();
    assert!(v.is_mapping());

    let m: serde_yaml::Mapping = v.as_mapping().unwrap().clone().into();
    let expected =
        serde_yaml::from_str("{foo: [foo, bar, baz, qux], qux: {foo: {foo: qux, bar: baz}}}")
            .unwrap();
    assert_eq!(m, expected);
}
