use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rayon::ThreadPoolBuilder;

use reclass_rs::Reclass;

fn bench(c: &mut Criterion) {
    ThreadPoolBuilder::new()
        .num_threads(1)
        .build_global()
        .unwrap();

    c.bench_function("Reclass::inventory() single-threaded", |b| {
        let r = Reclass::new("./tests/inventory", "nodes", "classes", true).unwrap();
        b.iter(|| black_box(r.render_inventory().unwrap()))
    });
}

criterion_group!(
name = inventory_single_threaded;
config = Criterion::default().sample_size(500);
targets = bench
);
criterion_main!(inventory_single_threaded);
