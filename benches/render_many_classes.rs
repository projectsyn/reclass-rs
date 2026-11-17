use criterion::{Criterion, criterion_group, criterion_main};
use std::fmt::Write;
use std::hint::black_box;

use reclass_rs::Reclass;

const BIG_KEYS: usize = 20_000;
const CLASSES: usize = 300;

/// Writes an inventory where one big class and many small classes all contribute to the
/// top-level key `data`, and each small class references a value in the big class.
fn write_inventory() -> std::path::PathBuf {
    let base = std::env::temp_dir().join(format!(
        "reclass-rs-bench-many-classes-{}",
        std::process::id()
    ));
    std::fs::remove_dir_all(&base).ok();
    std::fs::create_dir_all(base.join("classes")).unwrap();
    std::fs::create_dir_all(base.join("nodes")).unwrap();

    let mut big = String::from("parameters:\n  data:\n    big:\n");
    for i in 0..BIG_KEYS {
        writeln!(big, "      k{i}: v{i}").unwrap();
    }
    std::fs::write(base.join("classes/big.yml"), big).unwrap();

    let mut node = String::from("classes:\n  - big\n");
    for i in 0..CLASSES {
        let cls = format!("parameters:\n  data:\n    c{i}: '${{data:big:k{i}}}'\n");
        std::fs::write(base.join(format!("classes/c{i}.yml")), cls).unwrap();
        writeln!(node, "  - c{i}").unwrap();
    }
    std::fs::write(base.join("nodes/n.yml"), node).unwrap();
    base
}

fn bench(c: &mut Criterion) {
    let base = write_inventory();
    let r = Reclass::new(base.to_str().unwrap(), "nodes", "classes", false).unwrap();
    c.bench_function("Reclass::render_node() many classes on one key", |b| {
        b.iter(|| black_box(r.render_node("n").unwrap()));
    });
    std::fs::remove_dir_all(&base).unwrap();
}

criterion_group!(
name = render_many_classes;
config = Criterion::default().sample_size(10);
targets = bench
);
criterion_main!(render_many_classes);
