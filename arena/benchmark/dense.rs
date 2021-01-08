use criterion::{black_box, criterion_group, criterion_main, Criterion};

use pui_arena::base::dense::Arena;

#[allow(clippy::many_single_char_names)]
pub fn dense(c: &mut Criterion) {
    c.bench_function("dense insertion", |b| {
        b.iter(|| {
            let mut arena = Arena::new();
            let arena = black_box(&mut arena);
            let _: usize = arena.insert(());
        })
    });
    c.bench_function("dense re-insertion", |b| {
        let mut arena = Arena::new();
        b.iter(|| {
            let arena = black_box(&mut arena);
            let a: usize = arena.insert(());
            let b: usize = arena.insert(());
            let c: usize = arena.insert(());
            let d: usize = arena.insert(());
            let e: usize = arena.insert(());

            let (a, b, c, d, e) = black_box((a, b, c, d, e));

            arena.remove(b);
            arena.remove(d);
            arena.remove(a);
            arena.remove(c);
            arena.remove(e);

            let _ = black_box((a, b, c, d, e));

            let a: usize = arena.insert(());
            let b: usize = arena.insert(());
            let c: usize = arena.insert(());
            let d: usize = arena.insert(());
            let e: usize = arena.insert(());

            let (a, b, c, d, e) = black_box((a, b, c, d, e));

            arena.remove(b);
            arena.remove(d);
            arena.remove(a);
            arena.remove(c);
            arena.remove(e);

            let _ = black_box((a, b, c, d, e));
        })
    });
    c.bench_function("dense iteration non-contigious", |b| {
        let mut arena = Arena::new();

        for i in 0..1000 {
            let _: usize = arena.insert(i);
        }

        for i in (0..1000).filter(|&i| i % 2 == 0 || i % 11 == 0) {
            arena.remove(i);
        }

        b.iter(|| black_box(&arena).iter().sum::<i32>())
    });
    c.bench_function("dense iteration contigious", |b| {
        let mut arena = Arena::new();

        for i in 0..1000 {
            let _: usize = arena.insert(i);
        }

        for i in (1..=1000).filter(|&i| i % 11 < 6) {
            arena.remove(i);
        }

        b.iter(|| black_box(&arena).iter().sum::<i32>())
    });
    c.bench_function("dense iteration packed", |b| {
        let mut arena = Arena::new();

        for i in 0..545 {
            let _: usize = arena.insert(i);
        }

        b.iter(|| black_box(&arena).iter().sum::<i32>())
    });
}

criterion_group!(benches, dense);
criterion_main!(benches);
