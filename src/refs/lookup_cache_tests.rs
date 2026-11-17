use super::*;
use std::str::FromStr;

/// Builds params where `foo` is a ValueList with one layer per input document.
fn layered_params(base: &str, layers: &[&str]) -> Mapping {
    let mut p = Mapping::from_str(base).unwrap();
    for l in layers {
        p.merge(
            &Mapping::from_str(l).unwrap(),
            &ResolveState::default(),
            &RenderOpts::default(),
        )
        .unwrap();
    }
    assert!(p.get(&"foo".into()).unwrap().is_value_list());
    p
}

fn resolve(r: &str, p: &Mapping, state: &mut ResolveState) -> Result<Value> {
    parse_ref(r)
        .unwrap()
        .resolve(p, state, &RenderOpts::default())
}

#[test]
fn test_lookup_cache_hit() {
    let p = layered_params("foo: {a: x}", &["foo: {b: y}"]);
    let state = ResolveState::default();

    let mut st = state.clone();
    let a = resolve("${foo:a}", &p, &mut st).unwrap();
    assert_eq!(a, Value::Literal("x".into()));
    let cached = Rc::clone(state.lookup_cache.borrow().get("foo").unwrap());

    let mut st = state.clone();
    let b = resolve("${foo:b}", &p, &mut st).unwrap();
    assert_eq!(b, Value::Literal("y".into()));

    // The second lookup reused the flattened `foo` instead of flattening it again.
    let cache = state.lookup_cache.borrow();
    assert_eq!(cache.len(), 1);
    assert!(Rc::ptr_eq(&cached, cache.get("foo").unwrap()));
}

#[test]
fn test_lookup_cache_not_poisoned_by_loop_error() {
    // The `${bar}` layer loops back to `${foo:a}` while `foo` is flattened for `${foo:a}`.
    let p = layered_params("bar: {c: '${foo:a}'}\nfoo: {a: x}", &["foo: '${bar}'"]);
    let state = ResolveState::default();

    let mut st = state.clone();
    let a = resolve("${foo:a}", &p, &mut st).unwrap();
    assert_eq!(a, Value::Literal("x".into()));
    // That flattened `foo` holds a loop error for `c`, so it must not be cached.
    assert!(!state.lookup_cache.borrow().contains_key("foo"));

    // A fresh state sharing the cache must report the error for its own lookup path. A cached
    // copy of the first flattening would report paths ["bar", "foo:a"] instead.
    let mut st = ResolveState {
        lookup_cache: Rc::clone(&state.lookup_cache),
        ..Default::default()
    };
    let c = resolve("${foo:c}", &p, &mut st).unwrap();
    assert_eq!(
        c,
        Value::ResolveError(
            "Detected reference loop with reference paths [\"bar\", \"foo:a\", \"foo:c\"].".into()
        )
    );
    // Plain siblings still resolve from a fresh state.
    let mut st = ResolveState {
        lookup_cache: Rc::clone(&state.lookup_cache),
        ..Default::default()
    };
    let a = resolve("${foo:a}", &p, &mut st).unwrap();
    assert_eq!(a, Value::Literal("x".into()));
}

#[test]
fn test_lookup_cache_loop_still_detected() {
    // Loop between two keys of the flattened ValueList.
    let p = layered_params("foo: {a: '${foo:b}'}", &["foo: {b: '${foo:a}'}"]);
    let mut state = ResolveState::default();
    let err = resolve("${foo:a}", &p, &mut state).unwrap_err();
    assert_eq!(
        err.to_string(),
        "Detected reference loop with reference paths [\"foo:a\", \"foo:b\"]."
    );

    // Loop through a ValueList layer which is itself a reference.
    let p = layered_params("foo: {a: x}", &["foo: '${foo:a}'"]);
    let mut state = ResolveState::default();
    let err = resolve("${foo:a}", &p, &mut state).unwrap_err();
    assert_eq!(
        err.to_string(),
        "Detected reference loop with reference paths [\"foo:a\"]."
    );
}

#[test]
fn test_resolve_state_debug_summarizes_cache() {
    let p = layered_params("foo: {a: x}", &["foo: {b: y}"]);
    let mut state = ResolveState::default();
    resolve("${foo:a}", &p, &mut state).unwrap();
    let dbg = format!("{state:?}");
    assert!(dbg.contains("lookup_cache_len: 1"), "{dbg}");
    assert!(!dbg.contains("Mapping"), "{dbg}");
}
