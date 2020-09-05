use std::cell::RefCell;
use std::rc::Rc;

use criterion::{criterion_group, criterion_main, Criterion};
use fontdock::fs::{FsIndex, FsProvider};
use futures_executor::block_on;

use typstc::font::FontLoader;
use typstc::syntax::parse;
use typstc::typeset;

const FONT_DIR: &str = "fonts";

// 28 not too dense lines.
const COMA: &str = include_str!("../tests/coma.typ");

fn parsing_benchmark(c: &mut Criterion) {
    c.bench_function("parse-coma-28-lines", |b| b.iter(|| parse(COMA)));

    let long = COMA.repeat(100);
    c.bench_function("parse-coma-2800-lines", |b| b.iter(|| parse(&long)));
}

fn typesetting_benchmark(c: &mut Criterion) {
    let mut index = FsIndex::new();
    index.search_dir(FONT_DIR);

    let (descriptors, files) = index.clone().into_vecs();
    let provider = FsProvider::new(files.clone());
    let loader = FontLoader::new(Box::new(provider), descriptors);
    let loader = Rc::new(RefCell::new(loader));

    c.bench_function("typeset-coma-28-lines", |b| {
        b.iter(|| block_on(typeset(COMA, loader.clone(), Default::default())))
    });

    let long = COMA.repeat(100);
    c.bench_function("typeset-coma-2800-lines", |b| {
        b.iter(|| block_on(typeset(&long, loader.clone(), Default::default())))
    });
}

criterion_group!(benches, parsing_benchmark, typesetting_benchmark);
criterion_main!(benches);
