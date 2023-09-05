use super::*;
use paste::paste;
use std::str::FromStr;

#[test]
fn test_flattened_null() {
    assert_eq!(Value::Null.flattened().unwrap(), Value::Null);
}

macro_rules! test_flattened_simple {
    ($($variant:expr,$val:expr,$expected:expr ),*) => {
        $(
            paste! {
            #[test]
            fn [<test_flattened_simple_ $variant:snake>]() {
                let v = Value::$variant($val);
                let f = v.flattened().unwrap();
                assert_eq!(f, $expected);
            }
            }
        )*
    }
}

test_flattened_simple! {
    Bool,true,Value::Bool(true),
    Number,5.into(),Value::Number(5.into()),
    // String is flattened into Literal
    String,"foo".into(),Value::Literal("foo".into()),
    Literal,"foo".into(),Value::Literal("foo".into()),
    Sequence,vec![Value::Bool(true), 3.14.into()],Value::Sequence(vec![Value::Bool(true), 3.14.into()]),
    Mapping,Mapping::from_str("{foo: true, bar: 3.14}").unwrap(),Value::Mapping(Mapping::from_str("{foo: true, bar: 3.14}").unwrap())
}

#[test]
fn test_flattened_nested_mapping() {
    let m = Value::Mapping(Mapping::from_str("{foo: {foo: foo, bar: bar}, bar: bar}").unwrap());
    let f = m.flattened().unwrap();
    let mut foo = Mapping::new();
    foo.insert("foo".into(), Value::Literal("foo".to_string()))
        .unwrap();
    foo.insert("bar".into(), Value::Literal("bar".to_string()))
        .unwrap();
    let mut expected = Mapping::new();
    expected.insert("foo".into(), foo.into()).unwrap();
    expected
        .insert("bar".into(), Value::Literal("bar".to_string()))
        .unwrap();
    let expected = Value::Mapping(expected);
    assert_eq!(f, expected);
}

#[test]
fn test_flattened_simple_value_list() {
    let v = Value::ValueList(vec![
        Value::Literal("foo".into()),
        Value::Literal("bar".into()),
    ]);
    let f = v.flattened().unwrap();
    assert!(f.is_literal());
    assert_eq!(f, Value::Literal("bar".into()));
}

#[test]
fn test_flattened_mixed_value_list() {
    let v = Value::ValueList(vec![
        Value::Number(3.14.into()),
        Value::Null,
        Value::Literal("bar".into()),
    ]);
    let f = v.flattened().unwrap();
    assert!(f.is_literal());
    assert_eq!(f, Value::Literal("bar".into()));
}

#[test]
fn test_flattened_sequence_value_list() {
    let v = Value::ValueList(vec![
        Value::Sequence(vec!["foo".into(), "bar".into()]),
        Value::Sequence(vec!["baz".into(), "qux".into()]),
        Value::Sequence(vec!["foo".into()]),
    ]);
    let f = v.flattened().unwrap();
    assert_eq!(
        f,
        Value::Sequence(vec![
            "foo".into(),
            "bar".into(),
            "baz".into(),
            "qux".into(),
            "foo".into()
        ])
    );
}

#[test]
fn test_flattened_mapping_value_list() {
    let v = Value::ValueList(vec![
        Mapping::from_str("{foo: {foo: foo, bar: bar}, bar: bar}")
            .unwrap()
            .into(),
        Mapping::from_str("{baz: baz, qux: qux}").unwrap().into(),
    ]);
    let f = v.flattened().unwrap();
    assert!(f.is_mapping());

    let m: serde_yaml::Mapping = f.as_mapping().unwrap().clone().into();
    let expected =
        serde_yaml::from_str("{foo: {foo: foo, bar: bar}, bar: bar, baz: baz, qux: qux}").unwrap();
    assert_eq!(m, expected);
}

#[test]
fn test_flattened_null_over_mapping() {
    let v = Value::ValueList(vec![
        Mapping::from_str("{foo: {foo: foo, bar: bar}, bar: bar}")
            .unwrap()
            .into(),
        Value::Null,
    ]);
    let f = v.flattened().unwrap();
    assert!(f.is_null());
    assert_eq!(f, Value::Null);
}

#[test]
fn test_flattened_null_over_sequence() {
    let v = Value::ValueList(vec![
        Value::Sequence(vec!["foo".into(), "bar".into()]),
        Value::Null,
    ]);
    let f = v.flattened().unwrap();
    assert!(f.is_null());
    assert_eq!(f, Value::Null);
}

#[test]
fn test_flattened_map_over_sequence_error() {
    let v = Value::ValueList(vec![
        Value::Sequence(vec!["foo".into(), "bar".into()]),
        Value::Mapping(Mapping::from_str("foo: foo").unwrap()),
    ]);
    let f = v.flattened();
    assert!(f.is_err());
}

#[test]
fn test_flattened_map_over_simple_value_error() {
    let v = Value::ValueList(vec![
        Value::Bool(true),
        Value::Mapping(Mapping::from_str("foo: foo").unwrap()),
    ]);
    let f = v.flattened();
    assert!(f.is_err());
}

#[test]
fn test_flattened_sequence_over_map_error() {
    let v = Value::ValueList(vec![
        Value::Mapping(Mapping::from_str("foo: foo").unwrap()),
        Value::Sequence(vec!["foo".into(), "bar".into()]),
    ]);
    let f = v.flattened();
    assert!(f.is_err());
}

#[test]
fn test_flattened_sequence_over_simple_value_error() {
    let v = Value::ValueList(vec![
        Value::Bool(true),
        Value::Sequence(vec!["foo".into(), "bar".into()]),
    ]);
    let f = v.flattened();
    assert!(f.is_err());
}

#[test]
fn test_flattened_nested_mapping_value_list() {
    let v = Value::ValueList(vec![
        Mapping::from_str("foo: {foo: {foo: foo}}").unwrap().into(),
        Mapping::from_str("foo: {foo: {foo: bar}}").unwrap().into(),
        Mapping::from_str("foo: {foo: {bar: bar}}").unwrap().into(),
        Mapping::from_str("foo: {bar: {bar: bar}}").unwrap().into(),
    ]);
    let f = v.flattened().unwrap();
    assert!(f.is_mapping());
    let m: serde_yaml::Mapping = f.as_mapping().unwrap().clone().into();
    let expected =
        serde_yaml::from_str("foo: {foo: {foo: bar, bar: bar}, bar: {bar: bar}}").unwrap();
    assert_eq!(m, expected);
}

#[test]
fn test_flattened_nested_mapping_value_list_2() {
    let v = Value::ValueList(vec![
        Mapping::from_str("qux: {foo: {foo: {foo: foo}}}")
            .unwrap()
            .into(),
        Mapping::from_str("qux: {foo: {foo: {foo: bar}}}")
            .unwrap()
            .into(),
        Mapping::from_str("qux: {foo: {foo: {bar: bar}}}")
            .unwrap()
            .into(),
        Mapping::from_str("qux: {foo: {bar: {bar: bar}}}")
            .unwrap()
            .into(),
        Mapping::from_str("qux: {bar: {bar: {bar: bar}}}")
            .unwrap()
            .into(),
    ]);
    let f = v.flattened().unwrap();
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

    let f = Value::Mapping(dbg!(base)).flattened().unwrap();
    assert!(f.is_mapping());
    let m: serde_yaml::Mapping = f.as_mapping().unwrap().clone().into();
    let expected =
        serde_yaml::from_str("{foo: [foo, bar, baz, qux], qux: {foo: {foo: qux, bar: baz}}}")
            .unwrap();
    assert_eq!(m, expected);
}
