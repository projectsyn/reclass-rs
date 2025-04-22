use crate::types::Value;
use crate::{Config, Exports, Reclass};

#[test]
fn test_render_n1() {
    let mut c = Config::new(
        Some("./tests/inventory-class-notfound-regexp"),
        None,
        None,
        None,
    )
    .unwrap();
    c.load_from_file("reclass-config.yml", false).unwrap();
    let r = Reclass::new_from_config(c).unwrap();

    let n1 = r.render_node("n1", &Exports::default()).unwrap();
    assert_eq!(
        n1.classes,
        vec!["service.foo", "service.bar", "missing", "a", "amissing"]
    );
    assert_eq!(
        n1.parameters.get(&"a".into()),
        Some(&Value::Literal("a".into()))
    );
}

#[test]
fn test_render_n2() {
    let mut c = Config::new(
        Some("./tests/inventory-class-notfound-regexp"),
        None,
        None,
        None,
    )
    .unwrap();
    c.load_from_file("reclass-config.yml", false).unwrap();
    let r = Reclass::new_from_config(c).unwrap();

    let n2 = r.render_node("n2", &Exports::default());
    assert!(n2.is_err());
}
