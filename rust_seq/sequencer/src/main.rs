use std::net::SocketAddr;
use tokio::sync::mpsc as tokio_mpsc;
use bytes::Bytes;
use tokio::signal;
use std::sync::Arc;
use std::fs;

pub mod sequencer;
mod signature;
use sequencer::*;

const CHANNEL_CAPACITY: usize = 1_000_000;

#[tokio::main]
async fn main() {
    let fp_str = fs::read_to_string("ip.config").unwrap();
    let mut iter = fp_str.lines()
        .next()
        .unwrap()
        .split_whitespace();
    let num_nodes: u32 = iter.next().unwrap().parse().unwrap();
    let payload_size:usize = iter.next().unwrap().parse().unwrap();
    let node_ind: u32 = std::env::args()
        .nth(1)
        .expect("usage: cargo r --bin seq -- <NODE_INDEX>")
        .parse()
        .unwrap();
    let address_book: Vec<SocketAddr> = fp_str.lines()
        .skip(1)
        .map(|s| s.parse().expect("failed to parse SocketAddr"))
        .collect();

    println!("# of node {}, node ind {}, payload {}\naddress_book: {:?}", 
        num_nodes, 
        node_ind, 
        payload_size, 
        address_book);
    assert!(node_ind < num_nodes);

    let (tx_recv, rx_recv) = tokio_mpsc::channel::<Bytes>(CHANNEL_CAPACITY);
    let (tx_send, rx_send) = tokio_mpsc::channel::<CastType>(CHANNEL_CAPACITY);
    let measurement = Arc::new(MeasureDs::new());

    let curr_node = Sequencer::new(
        node_ind, 
        num_nodes, 
        address_book, 
        payload_size,
        measurement.clone()
    );

    curr_node.spawn_receiver(tx_recv);
    curr_node.spawn_sender(rx_send);
    curr_node.spawn_periodic_sender(tx_send.clone()); //, tx_main);
    tokio::spawn(async move {
        curr_node.run_main_loop(rx_recv, tx_send).await;
    });

    match signal::ctrl_c().await {
        Ok(()) => { println!("terminating..."); },
        Err(e) => {
            eprintln!("unable to listen for shutdown signal: {}", e);
        },
    }

    // wait for one second. we can implement cancel token to inform other tasks 
    // to shut down, but we don't know its ipact on performance. so.. putting 
    // it off for now 
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

    measurement.write_measurements(
        format!("./eval/node_{}.eval", node_ind), 
        node_ind,
        num_nodes, 
        payload_size
    ).await;
}