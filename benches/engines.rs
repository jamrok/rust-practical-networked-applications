use criterion::{black_box, criterion_group, criterion_main, Criterion};
use fake::Fake;
use kvs::{KvStore, KvsEngine, SledKvsEngine};

use rand::prelude::*;
use std::collections::HashMap;
use tempfile::TempDir;

pub type SampleData = HashMap<String, String>;
pub type SampleDataVec = Vec<String>;

pub struct KvEngine<Engine: KvsEngine> {
    engine: Engine,
    _temp_dir: TempDir,
}

impl<Engine: KvsEngine> KvEngine<Engine> {
    pub fn new() -> Self {
        let _temp_dir = TempDir::new().unwrap();
        let engine = Engine::open(_temp_dir.path()).unwrap();
        Self { engine, _temp_dir }
    }
}

impl<Engine: KvsEngine> Default for KvEngine<Engine> {
    fn default() -> Self {
        Self::new()
    }
}

pub fn write(c: &mut Criterion) {
    let kvs = KvEngine::<KvStore>::new();
    let sled = KvEngine::<SledKvsEngine>::new();
    let (list, _) = generate_write_list();
    let mut group = c.benchmark_group("engines/write");
    group.bench_function("kvs", |b| {
        b.iter(|| {
            load_data(&kvs, &list);
        })
    });
    group.bench_function("sled", |b| {
        b.iter(|| {
            load_data(&sled, &list);
        })
    });

    group.finish();
}

pub fn read(c: &mut Criterion) {
    let kvs = KvEngine::<KvStore>::new();
    let sled = KvEngine::<SledKvsEngine>::new();
    let (list, list_keys) = generate_write_list();
    let mut group = c.benchmark_group("engines/read");
    load_data(&kvs, &list);
    load_data(&sled, &list);
    let read_list = generate_random_read_list(list_keys);
    group.bench_function("kvs", |b| {
        b.iter(|| {
            get_data(&kvs, &read_list);
        })
    });
    group.bench_function("sled", |b| {
        b.iter(|| {
            get_data(&sled, &read_list);
        })
    });

    group.finish();
}

fn generate_write_list() -> (SampleData, SampleDataVec) {
    let mut list = SampleData::new();
    let mut list_vec = SampleDataVec::new();

    for _ in 1..=100 {
        let key = (1..=100_000).fake::<String>();
        let value = (1..=100_000).fake::<String>();
        list_vec.push(key.to_owned());
        list.insert(key, value);
    }
    (list, list_vec)
}

fn generate_random_read_list(list: SampleDataVec) -> SampleDataVec {
    let mut new_list = Vec::new();
    let mut rng = thread_rng();
    for _ in 1..=list.len() {
        let index = rng.gen_range(0..list.len());
        let item = list.get(index).unwrap();
        new_list.push(item.to_owned());
    }
    new_list
}

fn load_data<T: KvsEngine>(kvs: &KvEngine<T>, list: &SampleData) {
    for (key, value) in list {
        kvs.engine.set(key.to_owned(), value.to_owned()).unwrap();
    }
}

fn get_data<T: KvsEngine>(kvs: &KvEngine<T>, list: &SampleDataVec) {
    for key in list {
        black_box(kvs.engine.get(key.to_owned()).unwrap());
    }
}

criterion_group!(engines, read, write);
criterion_main!(engines);
