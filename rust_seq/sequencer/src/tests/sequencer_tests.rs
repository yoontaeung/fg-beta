// sequencer_tests.rs
use std::net::SocketAddr;
use tokio::sync::mpsc as tokio_mpsc;
use bytes::Bytes;
use message::Message;
use crate::Sequencer;
use std::str::FromStr;
use tokio::time::{Duration, timeout};
use network::{Receiver, MessageHandler, Writer, SimpleSender};
use crate::sequencer::CastType;

// Helper function to create a Sequencer with test data
fn setup_sequencer(node_id: u32) -> Sequencer {
    let node_ind = node_id;
    let num_nodes = 4;
    let address_book = vec![
        "127.0.0.1:8080",
        "127.0.0.1:8081",
        "127.0.0.1:8082",
        "127.0.0.1:8083",
    ]
    .iter()
    .map(|&addr| SocketAddr::from_str(addr).unwrap())
    .collect();

    Sequencer::new(node_ind, num_nodes, address_book)
}

// async fn setup_sequencer_with_task(seq: Sequencer) -> Sequencer {
//     const CHANNEL_CAPACITY: usize = 1_000;
//     let node_id = seq.node_ind;
//     // Spawning receiver to handle incoming messages from peer nodes
//     let (tx_recv, rx_recv) = tokio_mpsc::channel::<Bytes>(CHANNEL_CAPACITY);
//     let (tx_send, rx_send) = tokio_mpsc::channel::<CastType>(CHANNEL_CAPACITY);
//     let (tx_main, rx_main) = tokio_mpsc::channel::<Vec<u8>>(CHANNEL_CAPACITY);
    
//     seq.spawn_receiver(tx_recv);
//     seq.spawn_sender(rx_send);
//     seq.spawn_periodic_sender(tx_send.clone(), tx_main);
//     seq.run_main_loop(node_id, rx_recv, rx_main, tx_send).await
// }

#[tokio::test]
async fn test_new_sequencer() {
    let sequencer = setup_sequencer(0);

    assert_eq!(sequencer.node_ind, 0);
    assert_eq!(sequencer.num_nodes, 4);
    assert_eq!(sequencer.f_cnt, 1); 
    assert!(!sequencer.sent_fin.contains(&true)); // sent_fin should be empty
}

#[tokio::test]
async fn test_append_echo() {
    let mut sequencer = setup_sequencer(0);
    let sender = 1;
    let round = 0;
    let sign = vec![1, 2, 3, 4]; // Mock signature

    sequencer.append_echo(round, sender, sign.clone());

    assert_eq!(sequencer.signs.len(), 1); // Ensure signs has one entry now
    assert_eq!(sequencer.signs[0], vec![(sender, sign)]); // Check if the entry is correct
}

#[tokio::test]
async fn test_sequencer_msg_communication() {
    let mut sequencer = setup_sequencer(0);
    let (tx, mut rx) = tokio_mpsc::channel(32);
    
    // Mock a receiver for the sequencer
    sequencer.spawn_receiver(tx.clone());

    // Simulate sending a message
    let test_message = Message::Syn {
        sender: sequencer.node_ind,
        pub_key: vec![0; 32], // Mock public key
    };

    let test_message_clone = Message::Syn {
        sender: sequencer.node_ind,
        pub_key: vec![0; 32], // Mock public key
    };

    // Serialize the message to bytes
    let serialized_message = test_message.to_bytes().unwrap();

    tx.send(serialized_message).await.expect("failed to send");

    // Wait for the message to be received or timeout
    match timeout(Duration::from_secs(1), rx.recv()).await {
        Ok(Some(received_bytes)) => {
            // Deserialize the bytes back to a message
            let received_message = Message::from_bytes(received_bytes).unwrap();
            assert_eq!(received_message, test_message_clone); // Assert that the messages are equal
        }
        _ => panic!("Did not receive the message within the expected time"),
    }
}

