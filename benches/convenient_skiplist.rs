use convenient_skiplist::{iter::RangeHint, SkipList};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn iter_all_bench(c: &mut Criterion) {
    let mut sk = SkipList::<u32>::new();
    let upper = 50000;
    for i in 0..upper {
        sk.insert(i);
    }
    c.bench_function("iter_all(50000)", |b| {
        b.iter(|| {
            for i in sk.iter_all() {
                black_box(i);
            }
        })
    });
}

fn iter_range_bench(c: &mut Criterion) {
    let mut sk = SkipList::<u32>::new();
    let upper = 50000;
    for i in 0..upper {
        sk.insert(i);
    }
    c.bench_function("iter_all(50000)", |b| {
        b.iter(|| {
            for i in sk.range(&(upper / 2), &(upper / 2 + upper / 5)) {
                black_box(i);
            }
        })
    });
}

fn iter_range_with_bench(c: &mut Criterion) {
    let mut sk = SkipList::<u32>::new();
    let upper = 50000;
    for i in 0..upper {
        sk.insert(i);
    }
    c.bench_function("iter_bench_all(50000)", |b| {
        b.iter(|| {
            let f = sk.range_with(|&i| {
                if i < (upper / 2) {
                    RangeHint::SmallerThanRange
                } else if i > (upper / 2 + upper / 5) {
                    RangeHint::LargerThanRange
                } else {
                    RangeHint::InRange
                }
            });

            for i in f {
                black_box(i);
            }
        })
    });
}

fn bench_insert_linear_500(c: &mut Criterion) {
    c.bench_function("insert_500", |b| {
        b.iter(|| {
            let mut sk = SkipList::<u32>::new();
            let upper = 500;
            for i in 0..upper {
                black_box(sk.insert(i));
            }
        })
    });
}

fn bench_insert_linear_5000(c: &mut Criterion) {
    c.bench_function("insert_5000", |b| {
        b.iter(|| {
            let mut sk = SkipList::<u32>::new();
            let upper = 5000;
            for i in 0..upper {
                black_box(sk.insert(i));
            }
        })
    });
}

fn bench_insert_linear_50000(c: &mut Criterion) {
    c.bench_function("insert_50000", |b| {
        b.iter(|| {
            let mut sk = SkipList::<u32>::new();
            let upper = 50000;
            for i in 0..upper {
                black_box(sk.insert(i));
            }
        })
    });
}

fn bench_contains_500(c: &mut Criterion) {
    let mut sk = SkipList::<u32>::new();
    let upper = 500;
    for i in 0..upper {
        black_box(sk.insert(i));
    }
    c.bench_function("contains_500", |b| {
        b.iter(|| {
            black_box(sk.contains(&500));
        })
    });
}

fn bench_contains_5000(c: &mut Criterion) {
    let mut sk = SkipList::<u32>::new();
    let upper = 5000;
    for i in 0..upper {
        black_box(sk.insert(i));
    }
    c.bench_function("contains_5000", |b| {
        b.iter(|| {
            black_box(sk.contains(&4001));
        })
    });
}

fn bench_contains_50000(c: &mut Criterion) {
    let mut sk = SkipList::<u32>::new();
    let upper = 50000;
    for i in 0..upper {
        black_box(sk.insert(i));
    }
    c.bench_function("contains_50000", |b| {
        b.iter(|| {
            black_box(sk.contains(&33333));
        })
    });
}

fn bench_contains_500000(c: &mut Criterion) {
    let mut sk = SkipList::<u32>::new();
    let upper = 500000;
    for i in 0..upper {
        black_box(sk.insert(i));
    }
    c.bench_function("contains_500000", |b| {
        b.iter(|| {
            black_box(sk.contains(&333033));
        })
    });
}

criterion_group!(
    benches,
    iter_all_bench,
    iter_range_bench,
    iter_range_with_bench,
    bench_insert_linear_500,
    bench_insert_linear_5000,
    bench_insert_linear_50000,
    bench_contains_500,
    bench_contains_5000,
    bench_contains_50000,
    bench_contains_500000,
);
criterion_main!(benches);
