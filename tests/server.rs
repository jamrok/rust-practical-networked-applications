use kvs::{client::KvsClient, thread_pool::SharedQueueThreadPool, KvStore};

mod common;
use crossbeam::channel::unbounded;
use std::{net::SocketAddr, str::FromStr, sync::Arc, thread, time::Duration};

use kvs::thread_pool::RayonThreadPool;
#[cfg(test)]
use tracing::{debug, error, info};

#[test]
fn start_and_shutdown_server() {
    let address = SocketAddr::from_str("127.0.0.1:9001").unwrap();
    let test_server =
        common::TestKvsServer::<KvStore, SharedQueueThreadPool>::new(address, Some(1)).spawn(1);
    thread::sleep(Duration::from_secs(3));
    test_server.shutdown();
    test_server.wait_until_shutdown();
}

#[test]
fn client_can_send_command_to_server() {
    let address = SocketAddr::from_str("127.0.0.1:9002").unwrap();
    let test_server =
        common::TestKvsServer::<KvStore, SharedQueueThreadPool>::new(address, Some(2)).spawn(1);
    test_server.wait_until_ready();
    let client = KvsClient::new(address);
    let (_, commands) = common::generate_write_commands(50, 30, common::WordLength::Random);
    for command in commands {
        client
            .send_command(&command)
            .expect("Failed to send client command");
    }
}

// fn random_command(length: usize) -> Command {
//     let index = rand::thread_rng().gen_range(0..3);
//     let key = (1..=length).fake::<String>();
//     let value = (1..=length).fake::<String>();
//     match index {
//         0 => Command::from(Get::new(key)),
//         1 => Command::from(Remove::new(key)),
//         _ => Command::from(Set::new(key, value)),
//     }
// }

#[test]
fn client_can_send_bulk_commands_to_server() -> anyhow::Result<()> {
    let server_workers = 2;
    let total_commands_to_send = 1_000;
    let client_workers = 1_000.min(total_commands_to_send);
    let address = SocketAddr::from_str("127.0.0.1:9003").unwrap();

    let client = Arc::new(KvsClient::new(address));
    let test_server =
        common::TestKvsServer::<KvStore, RayonThreadPool>::new(address, Some(server_workers))
            .spawn(1);
    test_server.wait_until_ready();
    let (_, commands) =
        common::generate_write_commands(total_commands_to_send, 20, common::WordLength::Fixed);
    let (error_tx, error_rx) = unbounded();
    let _ = crossbeam_utils::thread::scope(|scope| {
        debug!(
            "Workers: {}, Tasks for each worker: {}",
            client_workers,
            total_commands_to_send / client_workers,
        );
        for command_list in commands.chunks(total_commands_to_send / client_workers) {
            let client = client.clone();
            let command_list = command_list.to_vec();
            let error_tx = error_tx.clone();
            let error_rx = error_rx.clone();
            scope.spawn(move |_| {
                for command in command_list {
                    if let Err(e) = client.send_command(&command.clone()) {
                        // Show a handful of errors
                        if error_rx.is_empty() {
                            error!("Command send error: {} | {}", e, error_rx.len());
                        }
                        error_tx
                            .send(e) // Log all errors
                            .expect("Unable to send error to the channel."); // Log all errors
                    }
                }
            });
        }
    });

    info!("DONE sending bulk commands");
    assert!(
        error_rx.is_empty(),
        "{} Errors encountered when sending client commands",
        error_rx.len()
    );

    Ok(())
}
