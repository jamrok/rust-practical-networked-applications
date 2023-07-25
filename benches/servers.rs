use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use crossbeam::channel::{unbounded, Receiver, Sender};
use kvs::{KvStore, KvsEngine, KvsError, SledKvsEngine};

#[allow(clippy::duplicate_mod)]
#[path = "../tests/common/mod.rs"]
mod tests;

use crate::tests::SampleWriteCommandsVec;
use kvs::{
    client::KvsClient,
    server::initialize_event_logging,
    shared::{Command, Get},
    thread_pool::{RayonThreadPool, SharedQueueThreadPool, ThreadPool},
    KvsError::GeneralError,
};
use std::{any::type_name, net::SocketAddr, str::FromStr, sync::Arc};
use tracing::{debug, error};

pub type SampleReadCommandsVec = Vec<(Command, String)>;

pub fn bench_servers(c: &mut Criterion) {
    kvstore_bench::<SledKvsEngine, RayonThreadPool>(c);
    kvstore_bench::<KvStore, RayonThreadPool>(c);
    kvstore_bench::<KvStore, SharedQueueThreadPool>(c);
}

fn get_type_name<T>() -> String {
    let type_name = type_name::<T>();
    type_name
        .split("::")
        .nth(2)
        .unwrap_or_else(|| panic!("Unable to parse name from type '{}'", type_name))
        .to_string()
}

pub fn kvstore_bench<Engine: KvsEngine, Pool: ThreadPool>(c: &mut Criterion) {
    initialize_event_logging();

    let total_commands_to_send = 200;
    let max_cpus = num_cpus::get();
    let client_workers = max_cpus;
    let write_client_workers = client_workers.min(total_commands_to_send);
    let read_client_workers = client_workers.min(total_commands_to_send);

    let address = SocketAddr::from_str("127.0.0.1:10003").unwrap();
    let (sample_data, commands) =
        tests::generate_write_commands(total_commands_to_send, 20, tests::WordLength::Fixed);
    let write_batch_size = total_commands_to_send / write_client_workers.max(1);
    let read_batch_size = total_commands_to_send / read_client_workers.max(1);
    let (error_tx, error_rx) = unbounded();
    let client = Arc::new(KvsClient::new(address));

    // Create a list of read commands from sample_data
    let mut read_commands = Vec::new();
    for (key, value) in sample_data {
        let command = Command::from(Get::new(key));
        read_commands.push((command, value));
    }

    let pool_name = get_type_name::<Pool>();
    let engine_name = get_type_name::<Engine>();
    let mut group = c.benchmark_group(format!("servers/{}/{}", engine_name, pool_name));
    for cpu in 0..=max_cpus {
        // Test from 1, then 2 to 2x the number of CPUs in even increments
        let cpus = (cpu << 1).max(1);
        // Setup
        let server_workers = cpus;
        let test_server =
            tests::TestKvsServer::<Engine, Pool>::new(address, Some(server_workers)).spawn(1);
        test_server.wait_until_ready();

        let write_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(write_client_workers)
            .build()
            .expect("Unable to create write pool.");

        let read_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(read_client_workers)
            .build()
            .expect("Unable to create read pool.");

        debug!(
            "Write Workers: {}, Tasks for each worker: {}",
            write_client_workers, write_batch_size,
        );

        let benchmark_id = BenchmarkId::new("write/cpus", cpus);
        group.bench_function(benchmark_id, |b| {
            b.iter(|| {
                do_write_heavy_work(
                    &commands,
                    &write_pool,
                    error_tx.clone(),
                    error_rx.clone(),
                    client.clone(),
                    write_batch_size,
                );
            });
        });

        debug!(
            "Read Workers: {}, Tasks for each worker: {}",
            read_client_workers, read_batch_size
        );
        let benchmark_id = BenchmarkId::new("read/cpus", cpus);
        group.bench_function(benchmark_id, |b| {
            b.iter(|| {
                do_read_heavy_work(
                    &read_commands,
                    &read_pool,
                    error_tx.clone(),
                    error_rx.clone(),
                    client.clone(),
                    read_batch_size,
                );
            });
        });
        test_server.shutdown();
        test_server.wait_until_shutdown();
    }
    group.finish();
}

fn do_write_heavy_work(
    commands: &SampleWriteCommandsVec,
    pool: &rayon::ThreadPool,
    error_tx: Sender<KvsError>,
    error_rx: Receiver<KvsError>,
    client: Arc<KvsClient>,
    batch_size: usize,
) {
    pool.scope(|scope| {
        for command_list in commands.chunks(batch_size) {
            let client = client.clone();
            let error_tx = error_tx.clone();
            let error_rx = error_rx.clone();
            scope.spawn(move |_| {
                for command in command_list {
                    if let Err(error) = client.send_command(command) {
                        // Show a sample of errors (if any)
                        if error_rx.is_empty() {
                            error!("Command send error: {:?}", error);
                        }
                        // Log all errors
                        error_tx
                            .send(error)
                            .expect("Unable to send error to the channel.");
                    }
                }
            });
        }
    });
    assert!(
        error_rx.is_empty(),
        "{} Errors encountered when sending client commands",
        error_rx.len()
    );
}

fn do_read_heavy_work(
    commands: &SampleReadCommandsVec,
    pool: &rayon::ThreadPool,
    error_tx: Sender<KvsError>,
    error_rx: Receiver<KvsError>,
    client: Arc<KvsClient>,
    batch_size: usize,
) {
    pool.scope(|scope| {
        for command_list in commands.chunks(batch_size) {
            let client = client.clone();
            let error_tx = error_tx.clone();
            let error_rx = error_rx.clone();
            scope.spawn(move |_| {
                for (command, expected_value) in command_list {
                    let possible_error = client
                        .send_command(command)
                        .map(|actual_value| {
                            let actual_value = actual_value.trim_end();
                            (actual_value != expected_value).then_some(GeneralError(format!(
                                "Data doesn't match! Expected: {}, Actual: {}",
                                expected_value, actual_value
                            )))
                        })
                        .unwrap_or_else(Some);

                    if let Some(error) = possible_error {
                        // Show a sample of errors (if any)
                        if error_rx.is_empty() {
                            error!("Command get error: {:?}", error);
                        }
                        // Log all errors
                        error_tx
                            .send(error)
                            .expect("Unable to send error to the channel.");
                    };
                }
            });
        }
    });
    assert!(
        error_rx.is_empty(),
        "{} Errors encountered when sending client commands",
        error_rx.len()
    );
}

criterion_group!(servers, bench_servers);
criterion_main!(servers);
