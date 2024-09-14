use bytes::Bytes;
use tokio::sync::mpsc as tokio_mpsc;
use std::error::Error;
use std::net::{SocketAddr, IpAddr, Ipv4Addr};
use async_trait::async_trait;
use network::{Receiver, MessageHandler, Writer};

pub mod sequencer;
mod signature;

const PORT:u16 = 13330;
const CHANNEL_CAPACITY: usize = 3;

#[tokio::main]
async fn main() {
    let (tx_recv, mut rx_recv) = tokio_mpsc::channel::<Bytes>(CHANNEL_CAPACITY);
    let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0,0,0,0)), PORT);
    println!("receiver listens on {:?}", socket);
    Receiver::spawn(socket, PeerReceiverHandler{tx_recv});
    loop {
        if let Some(bytes) = rx_recv.recv().await{
            println!("len {}", bytes.len());
        }
    }
}

#[derive(Clone)]
struct PeerReceiverHandler {
    tx_recv: tokio_mpsc::Sender<Bytes>
}
#[async_trait]
impl MessageHandler for PeerReceiverHandler {
    async fn dispatch(&self, _writer: &mut Writer, message: Bytes)
        -> Result<(), Box<dyn Error>>
    {
        self.tx_recv
            .send(message)
            .await
            .expect("failed to send received data");
        Ok(())
    }
}