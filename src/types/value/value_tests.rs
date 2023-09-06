use super::*;
use paste::paste;
use std::str::FromStr;

#[test]
fn test_is_null() {
    assert!(Value::Null.is_null());
    assert!(!Value::Bool(true).is_null());
}

#[test]
fn test_is_bool() {
    assert!(!Value::Null.is_bool());
    assert!(Value::Bool(true).is_bool());
}

#[test]
fn test_as_bool() {
    let b = Value::Bool(true);
    assert_eq!(b.as_bool(), Some(true));
    assert_eq!(Value::Null.as_bool(), None);
}

macro_rules! test_number {
    ($($ty:ident $val:expr)*) => {
        $(
            paste! {
            #[test]
            fn [<test_is_ $ty>]() {
                assert!(!Value::Null.[<is_ $ty>]());
                let n: $ty = $val;
                let n = Value::Number(n.into());
                assert!(n.[<is_ $ty>]());
            }

            #[test]
            fn [<test_as_ $ty>]() {
                assert_eq!(Value::Null.[<as_ $ty>](), None);
                let n: $ty = $val;
                let n = Value::Number(n.into());
                assert_eq!(n.[<as_ $ty>](), Some($val));
            }
            }
        )*
    }
}
test_number! { u64 5 i64 -3 f64 3.14 }

#[test]
fn test_is_string() {
    assert!(!Value::Null.is_string());
    let s = Value::from("foo");
    assert!(s.is_string());
    assert!(!s.is_literal());
}

#[test]
fn test_is_literal() {
    assert!(!Value::Null.is_literal());
    let s = Value::Literal("foo".into());
    assert!(s.is_literal());
    assert!(!s.is_string());
}

#[test]
fn test_as_str() {
    assert_eq!(Value::Null.as_str(), None);

    let s = Value::Literal("foo".into());
    assert_eq!(s.as_str(), Some("foo"));

    let s = Value::from("foo");
    assert_eq!(s.as_str(), Some("foo"));
}

#[test]
fn test_is_mapping() {
    assert!(!Value::Null.is_mapping());
    let m = Value::from(Mapping::new());
    assert!(m.is_mapping());
}

#[test]
fn test_as_mapping() {
    assert_eq!(Value::Null.as_mapping(), None);
    let m = Value::from(Mapping::new());
    assert_eq!(m.as_mapping(), Some(&Mapping::new()));
}

#[test]
fn test_as_mapping_mut() {
    assert_eq!(Value::Null.as_mapping_mut(), None);
    let mut m = Value::from(Mapping::new());
    let map = m.as_mapping_mut().unwrap();
    map.insert("foo".into(), "bar".into()).unwrap();
    assert_eq!(
        m.as_mapping(),
        Some(&Mapping::from_iter(vec![("foo".into(), "bar".into())]))
    );
}

#[test]
fn test_is_sequence() {
    assert!(!Value::Null.is_sequence());
    let s = Value::from(Sequence::new());
    assert!(s.is_sequence());
}

#[test]
fn test_as_sequence() {
    assert_eq!(Value::Null.as_sequence(), None);
    let s = Value::from(Sequence::new());
    assert_eq!(s.as_sequence(), Some(&Sequence::new()));
}

#[test]
fn test_as_sequence_mut() {
    assert_eq!(Value::Null.as_sequence_mut(), None);
    let mut s = Value::from(Sequence::new());
    let seq = s.as_sequence_mut().unwrap();
    seq.push("foo".into());
    assert_eq!(
        s.as_sequence(),
        Some(&Sequence::from_iter(vec!["foo".into()]))
    );
}

#[test]
fn test_is_value_list() {
    assert!(!Value::Null.is_value_list());
    let l = Value::ValueList(Sequence::new());
    assert!(l.is_value_list());
}

#[test]
fn test_as_value_list() {
    assert_eq!(Value::Null.as_value_list(), None);
    let l = Value::ValueList(Sequence::new());
    assert_eq!(l.as_value_list(), Some(&Sequence::new()));
}

#[test]
fn test_as_value_list_mut() {
    assert_eq!(Value::Null.as_value_list_mut(), None);
    let mut l = Value::ValueList(Sequence::new());
    let seq = l.as_value_list_mut().unwrap();
    seq.push("foo".into());
    assert_eq!(
        l.as_value_list(),
        Some(&Sequence::from_iter(vec!["foo".into()]))
    );
}

#[test]
fn test_get_mapping() {
    let m = Mapping::from_iter(vec![("a".into(), 1.into()), (2.into(), "foo".into())]);
    let m = Value::from(m);

    assert_eq!(m.get(&"a".into()), Some(&1.into()));
    assert_eq!(m.get(&2.into()), Some(&"foo".into()));
    assert_eq!(m.get(&"b".into()), None);
}

#[test]
fn test_get_mut_mapping() {
    let m = Mapping::from_iter(vec![("a".into(), 1.into())]);
    let mut m = Value::from(m);

    assert_eq!(m.get(&"a".into()), Some(&1.into()));
    let a = m.get_mut(&"a".into()).unwrap().unwrap();
    *a = "foo".into();
    assert_eq!(m.get(&"a".into()), Some(&"foo".into()));
    assert_eq!(m.get_mut(&"b".into()).unwrap(), None);
}

#[test]
fn test_get_mut_mapping_const_key() {
    let m = Mapping::from_iter(vec![("=a".into(), 1.into())]);
    let mut m = Value::from(m);

    assert_eq!(m.get(&"a".into()), Some(&1.into()));
    assert!(m.get_mut(&"a".into()).is_err());
}

#[test]
fn test_get_sequence() {
    let s = Sequence::from_iter(vec!["a".into(), 2.into(), 3.14.into()]);
    let s = Value::from(s);

    // non-u64 and out of bounds accesses return None
    assert_eq!(s.get(&(-1).into()), None);
    assert_eq!(s.get(&3.14.into()), None);
    assert_eq!(s.get(&3.into()), None);

    // non-number accesses return None
    assert_eq!(s.get(&"foo".into()), None);

    assert_eq!(s.get(&0.into()), Some(&"a".into()));
    assert_eq!(s.get(&1.into()), Some(&2.into()));
    assert_eq!(s.get(&2.into()), Some(&3.14.into()));
}

#[test]
fn test_get_mut_sequence() {
    let s = Sequence::from_iter(vec!["a".into(), 2.into(), 3.14.into()]);
    let mut s = Value::from(s);

    assert_eq!(s.get(&0.into()), Some(&"a".into()));
    let e0 = s.get_mut(&0.into()).unwrap().unwrap();
    *e0 = "foo".into();
    assert_eq!(s.get(&0.into()), Some(&"foo".into()));
    assert_eq!(s.get_mut(&3.into()).unwrap(), None);
}

#[test]
fn test_get_valuelist() {
    let s = Sequence::from_iter(vec!["a".into(), 2.into(), 3.14.into()]);
    let l = Value::ValueList(s);

    // non-u64 and out of bounds accesses return None
    assert_eq!(l.get(&(-1).into()), None);
    assert_eq!(l.get(&3.14.into()), None);
    assert_eq!(l.get(&3.into()), None);

    // non-number accesses return None
    assert_eq!(l.get(&"foo".into()), None);

    assert_eq!(l.get(&0.into()), Some(&"a".into()));
    assert_eq!(l.get(&1.into()), Some(&2.into()));
    assert_eq!(l.get(&2.into()), Some(&3.14.into()));
}

#[test]
fn test_get_mut_valuelist() {
    let s = Sequence::from_iter(vec!["a".into(), 2.into(), 3.14.into()]);
    let mut l = Value::ValueList(s);

    assert_eq!(l.get(&0.into()), Some(&"a".into()));
    let e0 = l.get_mut(&0.into()).unwrap().unwrap();
    *e0 = "foo".into();
    assert_eq!(l.get(&0.into()), Some(&"foo".into()));
    assert_eq!(l.get_mut(&3.into()).unwrap(), None);
}

#[test]
fn test_get_other_types() {
    assert_eq!(Value::Null.get(&"a".into()), None);
    assert_eq!(Value::Bool(true).get(&"a".into()), None);
    assert_eq!(Value::String("foo".into()).get(&"a".into()), None);
    // Strings can't be treated as sequences
    assert_eq!(Value::String("foo".into()).get(&0.into()), None);
    assert_eq!(Value::Literal("foo".into()).get(&"a".into()), None);
    assert_eq!(Value::Number(1.into()).get(&"a".into()), None);
}

#[test]
fn test_get_mut_other_types() {
    assert_eq!(Value::Null.get_mut(&"a".into()).unwrap(), None);
    assert_eq!(Value::Bool(true).get_mut(&"a".into()).unwrap(), None);
    assert_eq!(
        Value::String("foo".into()).get_mut(&"a".into()).unwrap(),
        None
    );
    // Strings can't be treated as sequences
    assert_eq!(
        Value::String("foo".into()).get_mut(&0.into()).unwrap(),
        None
    );
    assert_eq!(
        Value::Literal("foo".into()).get_mut(&"a".into()).unwrap(),
        None
    );
    assert_eq!(Value::Number(1.into()).get_mut(&"a".into()).unwrap(), None);
}

#[test]
fn test_strip_prefix() {
    let k1 = Value::from("=foo");
    let k2 = Value::from("~foo");
    let k3 = Value::from("foo");
    let k4 = Value::from(3);
    assert_eq!(
        k1.strip_prefix(),
        (Value::from("foo"), Some(KeyPrefix::Constant))
    );
    assert_eq!(
        k2.strip_prefix(),
        (Value::from("foo"), Some(KeyPrefix::Override))
    );
    assert_eq!(k3.strip_prefix(), (Value::from("foo"), None));
    assert_eq!(k4.strip_prefix(), (Value::from(3), None));
}

#[test]
fn test_raw_string_literal() {
    assert_eq!(
        Value::Literal("foo".into()).raw_string().unwrap(),
        "foo".to_string()
    );
}

#[test]
fn test_raw_string_null() {
    assert_eq!(Value::Null.raw_string().unwrap(), "None".to_string());
}

#[test]
fn test_raw_string_number() {
    assert_eq!(
        Value::Number(5.into()).raw_string().unwrap(),
        "5".to_string()
    );
    assert_eq!(
        Value::Number((-1).into()).raw_string().unwrap(),
        "-1".to_string()
    );
    assert_eq!(
        Value::Number(3.14.into()).raw_string().unwrap(),
        "3.14".to_string()
    );
    assert_eq!(
        Value::Number(serde_yaml::Number::from(f64::INFINITY))
            .raw_string()
            .unwrap(),
        ".inf".to_string()
    );
    assert_eq!(
        Value::Number(serde_yaml::Number::from(f64::NEG_INFINITY))
            .raw_string()
            .unwrap(),
        "-.inf".to_string()
    );
    assert_eq!(
        Value::Number(serde_yaml::Number::from(f64::NAN))
            .raw_string()
            .unwrap(),
        ".nan".to_string()
    );
}

#[test]
fn test_raw_string_mapping() {
    let mut m = Value::Mapping(Mapping::from_str("{foo: foo, bar: true, baz: 1.23}").unwrap());
    // turn string values into literals by calling flatten
    m.render(&Mapping::new()).unwrap();
    let mstr = m.raw_string().unwrap();
    // NOTE(sg): serde_json output is sorted by keys
    assert_eq!(mstr, r#"{"bar":true,"baz":1.23,"foo":"foo"}"#);
}

#[test]
fn test_raw_string_sequence() {
    let v = Value::Sequence(vec!["foo".into(), 3.14.into(), Value::Bool(true)]);
    let vstr = v.raw_string().unwrap();
    assert_eq!(vstr, r#"["foo",3.14,true]"#);
}

#[test]
fn test_raw_string_mapping_nonstring_keys() {
    // raw_string() will turn boolean, number, and null values used as keys into strings when
    // serializing the Mapping as JSON.
    let m = Mapping::from_str("{true: foo, 3.14: true, ~: 1.23}").unwrap();
    // turn string values into literals by calling interpolate
    let m = Value::Mapping(m).rendered(&Mapping::new()).unwrap();
    let mstr = m.raw_string().unwrap();
    // NOTE(sg): serde_json output is sorted by keys
    assert_eq!(mstr, r#"{"3.14":true,"null":1.23,"true":"foo"}"#);
}
