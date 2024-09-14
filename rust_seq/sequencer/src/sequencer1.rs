use std::net::{SocketAddr, IpAddr, Ipv4Addr};
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc as tokio_mpsc;
use tokio::time as tokio_time;
use bytes::Bytes;
use async_trait::async_trait;
use network::{Receiver, MessageHandler, Writer, SimpleSender};
use message::Message;
use crate::signature::KeyPair;
use std::collections::HashSet;
use ring::digest;

#[cfg(test)]
#[path = "tests/sequencer_tests.rs"]
pub mod sequencer_tests;

const PAYLOAD_SIZE:usize = 10_000_000;
type U8Arr = Vec<u8>;

pub struct Sequencer {
    node_ind: u32,
    num_nodes: u32,
    f_cnt: usize,

    /* address related */
    self_addr: SocketAddr,
    address_book: Vec<SocketAddr>,

    /* key related */
    keypair: Arc<KeyPair>,
    peer_pkeys: Vec<Option<U8Arr>>,

    /* transactions and data related */
    txs: Arc<Vec<Mutex<Vec<U8Arr>>>>, // txs[0][1][2] -> peer 0's msg of round 1, the third u8
    tx_hashes: Arc<Vec<Mutex<Vec<U8Arr>>>>,
    signs: Arc<Vec<Mutex<Vec<(u32, U8Arr)>>>>, // signs[0][1] -> second (peer index, sign) in round 0

    /* checks if a node has sent message to peers */
    sent_echo: Vec<Vec<bool>>,  // sent_echo[0][1] -> sent echo to sender 0 in round 1
    sent_fin: Vec<bool>,   // sent_fin[0] -> round 0, sent finals to peers
    sent_sup: Vec<Vec<bool>>,   // sent_sup[0][1] -> sent sup to sender 0 in round 1

    /* deliver related */
    delivered: Vec<Vec<bool>>,   // delivered[0][1]  -> peer 0's msg in round 1 is delivered. 
    delivered_cnt: Vec<U8Arr>, // to count up the number of sup messages

    /* for throughput measurement */
    total_sent: Arc<Mutex<u32>>,
    sent_vec: Arc<Mutex<Vec<u32>>>,
    send_sent_vec: Arc<Mutex<Vec<u32>>>,
    total_recv: Arc<Mutex<u32>>,
    recv_vec: Arc<Mutex<Vec<u32>>>,
    send_recv: Arc<Mutex<u32>>,
    send_recv_vec: Arc<Mutex<Vec<u32>>>,
    round_latency_vec:Arc<Mutex<Vec<u128>>>,
}

impl Sequencer {
    pub fn new(
        node_ind:u32, 
        num_nodes:u32, 
        address_book:Vec<SocketAddr>,
        sent_vec:Arc<Mutex<Vec<u32>>>,
        send_sent_vec:Arc<Mutex<Vec<u32>>>,
        recv_vec:Arc<Mutex<Vec<u32>>>,
        send_recv_vec:Arc<Mutex<Vec<u32>>>,
        round_latency_vec:Arc<Mutex<Vec<u128>>>
    ) -> Self {
        let mut peer_pkeys = Vec::with_capacity(num_nodes as usize);
        let txs = Arc::new(Vec::with_capacity(num_nodes as usize));
        let tx_hashes = Arc::new(Vec::with_capacity(num_nodes as usize));
        let mut sent_echo = Vec::with_capacity(num_nodes as usize);
        let sent_fin = Vec::new();
        let mut sent_sup = Vec::with_capacity(num_nodes as usize);
        let mut delivered = Vec::with_capacity(num_nodes as usize);
        let mut delivered_cnt = Vec::with_capacity(num_nodes as usize);

        for _i in 0..num_nodes {
            peer_pkeys.push(None);
            txs.push(Mutex::new(Vec::new()));
            tx_hashes.push(Mutex::new(Vec::new()));
            sent_echo.push(Vec::new());
            sent_sup.push(Vec::new());
            delivered.push(Vec::new());
            delivered_cnt.push(Vec::new());
        }

        Sequencer {
            /* basic info */
            node_ind,
            num_nodes,
            f_cnt: (num_nodes as usize - 1) / 3,
            /* address */
            self_addr: address_book[node_ind as usize],
            address_book,
            /* keys */
            keypair: Arc::new(KeyPair::new()),
            peer_pkeys,
            /* transactions */
            txs,
            tx_hashes,
            signs: Arc::new(Vec::new()),
            /* checking flags */
            sent_echo,
            sent_fin,
            sent_sup,
            /* deliver */
            delivered,
            delivered_cnt,
            /* measurement */
            total_sent:Arc::new(Mutex::new(0)),
            sent_vec,
            send_sent_vec,
            total_recv:Arc::new(Mutex::new(0)),
            recv_vec,
            send_recv:Arc::new(Mutex::new(0)),
            send_recv_vec,
            round_latency_vec,
        }
    }

    pub fn spawn_receiver(&self, tx_recv: tokio_mpsc::Sender<Bytes>){
        let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0,0,0,0)), self.self_addr.port());
        println!("receiver listens on {:?}", socket);
        Receiver::spawn(socket, PeerReceiverHandler{tx_recv});
    }

    /*
    * Spawn a task that handles sending messages to other peers.
    * The task reads from the channel, and behave based on what it received. When
    * it receives Send_format::Multicast via the channel, it sends the message to
    * all connected peers in bytes. Multicast include: Send, Final, Ready. When it
    * receives Send_format::Unicast via the channel, it sends the message to the
    * indicated peer (or dest). Unicast include: Echo, Ready.   -> Ready??
    */
    pub fn spawn_sender(
        &self,
        mut rx_send:tokio_mpsc::Receiver<CastType>
    ){
        let address_book = self.address_book.clone();
        let syn_msg = Message::Syn{ 
            sender: self.node_ind,
            pub_key: self.keypair.pub_key.clone(),
        };
        let mut peers:Vec<SocketAddr> = vec![];
        for address in &address_book {
            if *address != self.self_addr {
                peers.push(*address);
            }
        }
        let total_sent = Arc::clone(&self.total_sent);
        tokio::spawn(async move {
            let mut msg_sender = SimpleSender::new();
            msg_sender.init(Message::to_bytes(syn_msg).unwrap(), peers.clone()).await;
            loop {
                if let Some(msg) = rx_send.recv().await {
                    match msg {
                        CastType::Multicast {bytes} => {
                            {
                                let mut total_sent = total_sent.lock().unwrap();
                                *total_sent += (bytes.len() * peers.len()) as u32;
                            }
                            msg_sender.broadcast(peers.clone(), bytes).await;
                        }
                        CastType::Unicast {dest, bytes} => {
                            {
                                let mut total_sent = total_sent.lock().unwrap();
                                *total_sent += bytes.len() as u32;
                            }
                            msg_sender.send(address_book[dest as usize], bytes).await;
                        }
                    }
                }
            }
        });
    }
    /*
    * Spawns a task that send out Send_format::Multicast message to the sending task
    * via channel periodically.
    * The interval is set to 1 second. The task starts to send message to sending
    * task after this task wakes from 5 seconds of sleep.
    */
    pub fn spawn_periodic_sender(
        &self,
        tx_send: tokio_mpsc::Sender<CastType>,
        tx_main: tokio_mpsc::Sender<U8Arr>
    ){
        let node_ind = self.node_ind;
        let peer_cnt = self.num_nodes - 1;
        let total_sent = Arc::clone(&self.total_sent);
        let sent_vec = Arc::clone(&self.sent_vec);
        let send_sent_vec = Arc::clone(&self.send_sent_vec);
        let total_recv = Arc::clone(&self.total_recv);
        let recv_vec = Arc::clone(&self.recv_vec);
        let send_recv = Arc::clone(&self.send_recv);
        let send_recv_vec = Arc::clone(&self.send_recv_vec);
        let round_latency_vec = Arc::clone(&self.round_latency_vec);

        let txs = Arc::clone(&self.txs);
        let tx_hashes = Arc::clone(&self.tx_hashes);
        let signs = Arc::clone(&self.signs);
        let keypair = Arc::clone(&self.keypair);

        tokio::spawn(async move {
            tokio_time::sleep(tokio_time::Duration::from_secs(5)).await;
            let mut interval = tokio_time::interval(tokio_time::Duration::from_millis(1000));
            let mut curr_round = 0;
            let usize_ind = node_ind as usize;

            let payload = vec![node_ind as u8; PAYLOAD_SIZE];
            let mut start = Instant::now();
            loop {
                interval.tick().await;
                {
                    let duration = start.elapsed().as_millis();
                    let mut round_latency_vec = round_latency_vec.lock().unwrap();
                    round_latency_vec.push(duration);
                    start = Instant::now();
                }
                {
                    let mut recv_vec = recv_vec.lock().unwrap();
                    let mut total_recv = total_recv.lock().unwrap();
                    recv_vec.push(*total_recv);
                    *total_recv = 0;
                }
                {
                    let mut sent_vec = sent_vec.lock().unwrap();
                    let mut total_sent = total_sent.lock().unwrap();
                    sent_vec.push(*total_sent);
                    *total_sent = 0;
                }
                println!("--- sending message from round {} --- ", curr_round);
                {
                    let mut txs = txs[usize_ind].lock().unwrap();
                    txs.push(payload.clone());
                }
                {
                    let payload_digest = digest::digest(&digest::SHA256, &payload);
                    let mut tx_hashes = tx_hashes[usize_ind].lock().unwrap();
                    Self::append_echo(
                        &signs, 
                        curr_round, 
                        node_ind, 
                        keypair.sign(payload_digest.as_ref())
                    );
                    tx_hashes.push(payload_digest.as_ref().to_vec());
                }
                tx_send.send(
                    CastType::Multicast{
                        bytes: Message::Send{
                            sender: node_ind,
                            rn: curr_round as u32,
                            payload: payload.clone(),
                            // TODO: change dummy payload
                        }.to_bytes()
                        .unwrap(),
                    }
                ).await
                .expect("periodic sender:: failed to send send msg to peer");

                // tx_main.send(payload.clone()).await.expect("periodic:: failed to send to main");
                curr_round += 1;
            }
        });
    }

    pub async fn run_main_loop(
        mut self,
        mut rx_recv:tokio_mpsc::Receiver<Bytes>,
        rx_main:tokio_mpsc::Receiver<U8Arr>,
        tx_send:tokio_mpsc::Sender<CastType>
    ){
        let usize_ind = self.node_ind as usize;
        loop{
            // tokio::select!{
                if let Some(bytes) = rx_recv.recv().await {
                    {
                        let mut total_recv = self.total_recv.lock().unwrap();
                        *total_recv += bytes.len() as u32;
                    }
                    match Message::from_bytes(bytes).unwrap() {
                        Message::Syn{sender, pub_key} => {
                            if self.peer_pkeys[sender as usize] == None {
                                self.peer_pkeys[sender as usize] = Some(pub_key);
                            }
                            else { panic!("peer {} sent pkey twice!", sender); }
                        }
                        Message::Send{sender, rn, payload} => {
                            self.handle_send_msg(
                                sender as usize, 
                                rn as usize, 
                                payload, 
                                tx_send.clone()
                            ).await;
                        },
                        Message::Echo{sender, rn, sign} => {
                            self.handle_echo_msg(
                                sender as usize,
                                rn as usize,
                                sign,
                                tx_send.clone()
                            ).await;
                        },
                        Message::Fin{sender, rn, sign_cnt, signs} => {
                            self.handle_fin_msg(
                                sender as usize,
                                rn as usize,
                                sign_cnt as usize,
                                signs,
                                tx_send.clone()
                            ).await;
                        },
                        Message::Sup{sender, rn, originator, .. } => {
                            let (originator, rn) = (originator as usize, rn as usize);
                            while self.delivered[originator].len() <= rn {
                                self.delivered[originator].push(false);
                                self.delivered_cnt[originator].push(0);
                            }
                            // this is for self. should send sup message right after sending fin message
                            while self.sent_sup[originator].len() <= rn {
                                self.sent_sup[originator].push(false);
                            }
                            self.delivered_cnt[originator][rn] += 1;
                            let cnt = self.delivered_cnt[originator][rn] as usize;
                            if cnt >= self.f_cnt + 1 && !self.sent_sup[originator][rn] {
                                // TODO: send sup msg. amplification
                            } 
                            if cnt >= 2*self.f_cnt + 1 && !self.delivered[originator][rn] {
                                self.delivered[originator][rn] = true;
                                println!{"{}'s msg is delivered in round {}", originator, rn};
                            }
                        },
                    }
                };
                /* /
                Some(bytes) = rx_main.recv() => {
                    let rn:usize = self.txs[usize_ind].len();
                    let payload_digest = digest::digest(&digest::SHA256, &bytes);
                    let payload_digest_bytes = payload_digest.as_ref();

                    self.append_echo(
                        rn,
                        self.node_ind,
                        self.keypair.sign(payload_digest_bytes)
                    );
                    self.txs[usize_ind].push(bytes);
                    while self.tx_hashes[usize_ind].len() <= rn {
                        self.tx_hashes[usize_ind].push(Vec::<u8>::new());
                    }
                    self.tx_hashes[usize_ind][rn] = payload_digest_bytes.to_vec();
                    
                    if self.got_enough_echo(rn) {
                        tx_send.send(CastType::Multicast{
                            bytes: Message::Fin{
                                sender: self.node_ind,
                                rn: rn as u32,
                                sign_cnt: self.signs[rn].len() as u32,
                                signs: self.signs[rn].clone(),
                            }
                            .to_bytes()
                            .unwrap(),
                        })
                        .await
                        .expect("failed to send fin msg to peers");
                    }
                }
                */
        }
    } // end of run_main_loop()

    async fn handle_send_msg(
        &mut self, 
        sender:usize, 
        rn:usize, 
        payload:U8Arr,
        ref tx_send:tokio_mpsc::Sender<CastType>
    ){
        // Update sent_echo state before sending Echo
        if let Some(echo_rounds) = self.sent_echo.get_mut(sender) {
            while echo_rounds.len() <= rn {
                echo_rounds.push(false);
            }
        }
        // Calculate the digest of the payload 
        let payload_digest = digest::digest(&digest::SHA256, &payload);

        if self.sent_echo[sender][rn] == false {
            tx_send.send(CastType::Unicast{
                dest: sender as u32,
                bytes: Message::Echo{
                    sender: self.node_ind,
                    rn: rn as u32,
                    sign: self.keypair.sign(payload_digest.as_ref()),
                }
                .to_bytes()
                .unwrap()
            })
            .await
            .expect("failed to send echo msg");
            self.sent_echo[sender][rn] = true;

            {
                let mut sender_txs = self.txs[sender].lock().unwrap();
                while sender_txs.len() <= rn {
                    sender_txs.push(Vec::new());
                }
                sender_txs[rn] = payload;
            }
            {
                let mut sender_hashes = self.tx_hashes[sender].lock().unwrap();
                while sender_hashes.len() <= rn {
                    sender_hashes.push(Vec::new());
                }
                sender_hashes[rn] = payload_digest.as_ref().to_vec();
            }
        }
    }

    async fn handle_echo_msg(
        &mut self,
        sender:usize, 
        rn:usize, 
        sign:U8Arr, 
        ref tx_send:tokio_mpsc::Sender<CastType>
    ){
        while self.sent_fin.len() <= rn{
            self.sent_fin.push(false);
        }

        let pkey = self.peer_pkeys[sender].as_ref().unwrap();
        {
            let hash = self.tx_hashes[self.node_ind as usize].lock().unwrap();
            if !KeyPair::verify_signature(
                &pkey,
                &hash[rn],
                &sign
            ){
                println!("wrong signature!!");
                return;
            }

        }
        Self::append_echo(&self.signs, rn, sender as u32, sign);

        // TODO: heuristically wait for other f peers
        if Self::got_enough_echo(&self.signs, rn, self.num_nodes as usize) && !self.sent_fin[rn]{
            self.sent_fin[rn as usize] = true;
            let signs = self.signs[rn].lock().unwrap();
            tx_send.send(CastType::Multicast{
                bytes: Message::Fin{
                    sender: self.node_ind,
                    rn: rn as u32,
                    sign_cnt: signs.len() as u32,
                    signs: signs.clone(),
                }
                .to_bytes()
                .unwrap(),
            })
            .await
            .expect("failed to send fin msg to peers");

            tx_send.send(CastType::Multicast{
                bytes: Message::Sup{
                    sender: self.node_ind,
                    rn: rn as u32,
                    sign_cnt: signs.len() as u32,
                    signs: signs.clone(),
                    originator: self.node_ind,
                    // TODO: distinguish, assume optimistic case for now
                    payload:Vec::new(), 
                }
                .to_bytes()
                .unwrap(),
            })
            .await
            .expect("failed to send fin msg to peers");

            drop(signs);

            while self.delivered[self.node_ind as usize].len() <= rn {
                self.delivered[self.node_ind as usize].push(false);
                self.delivered_cnt[self.node_ind as usize].push(0);
            }
            self.delivered_cnt[self.node_ind as usize][rn] += 1;
        }
    }

    // TODO: after sending final message, should send sup message too
    async fn handle_fin_msg(
        &mut self,
        sender:usize,
        rn:usize,
        sign_cnt:usize,
        signs:Vec<(u32, U8Arr)>,
        ref tx_send:tokio_mpsc::Sender<CastType>
    ){
        if signs.len() != sign_cnt {
            eprintln!("Error: Mismatched sign count");
            return;
        }

        while self.sent_sup[sender].len() <= rn {
            self.sent_sup[sender].push(false);
        }

        if self.sent_sup[sender][rn] == true {
            return;
        }
        else { self.sent_sup[sender][rn] = true; }

        // verify each sig in signs list
        let mut valid_signatures = 0;
        let hashes = self.tx_hashes[sender].lock().unwrap();
        // if let Some(h_tx) = self.tx_hashes.get(sender).unwrap().get(rn) {
        if let Some(h_tx) = hashes.get(rn) {
            for (signer_id, sign) in signs.iter() {
                if *signer_id == self.node_ind {
                    valid_signatures += 1;
                    continue;
                }

                let pkey = self.peer_pkeys[*signer_id as usize].clone().unwrap();
                if KeyPair::verify_signature(&pkey, h_tx, sign) {
                    valid_signatures += 1;
                }
            }
        }
        else { eprintln!("hash not found!"); return; }
        
        if valid_signatures >= 2 * self.f_cnt + 1 {
            // Create a set of signer IDs for quick lookup
            let signers_set: HashSet<u32> = signs.iter().map(|(id, _)| *id).collect();
            // Iterate over all peers
            for i in 0..self.num_nodes {
                if i == self.node_ind {
                    while self.delivered[sender].len() <= rn {
                        self.delivered[sender].push(false);
                        self.delivered_cnt[sender].push(0);
                    }
                    self.delivered_cnt[sender][rn] += 1;
                    continue;
                }
                let sup_msg = if signers_set.contains(&i) {
                    // Peer is in signs, send SUP with empty payload
                    Message::Sup {
                        sender: self.node_ind,
                        rn: rn as u32,
                        sign_cnt: sign_cnt as u32, 
                        signs: signs.clone(),
                        originator: sender as u32,
                        payload: Vec::new(), // Empty payload
                    }
                } 
                else {
                    // Peer is not in signs, send SUP with payload if available
                    Message::Sup {
                        sender: self.node_ind,
                        rn: rn as u32,
                        sign_cnt: sign_cnt as u32, // No signs included
                        signs: signs.clone(),
                        originator: sender as u32,
                        payload: self.txs[sender].lock().unwrap()[rn].clone(),
                    }
                };
                if let Err(e) = tx_send.send(CastType::Unicast {
                    dest: i,
                    bytes: sup_msg.to_bytes().unwrap(),
                }).await {
                    eprintln!("Failed to send SUP message: {}", e);
                }
            } 
        } else {
            eprintln!("Verification failed. Not enough valid signatures: {} / {}", valid_signatures, 2 * self.f_cnt + 1);
            // Handle insufficient valid signatures
        } 
    }

    fn append_echo(
        ref signs:&Arc<Vec<Mutex<Vec<(u32, U8Arr)>>>>, 
        rn:usize, 
        sender:u32, 
        sign:U8Arr
    ){
        while signs.len() <= rn {
            signs.push(Mutex::new(Vec::<(u32, U8Arr)>::new()));
        }
        let mut sign_rn = signs[rn].lock().unwrap();
        sign_rn.push((sender, sign));
    }

    fn got_enough_echo(
        ref signs:&Arc<Vec<Mutex<Vec<(u32, U8Arr)>>>>, 
        rn:usize,
        echo_threashold:usize
    ) -> bool {
        // if self.signs[rn].len() > 2 * self.f_cnt {
        if signs[rn].lock().unwrap().len() == echo_threashold {
            return true;
        }
        false
    }
} // end of impl Sequencer


/*
* Send_format is the enum for the sending task.
* The sending task spawned with `spawn_sender()` will read a Send_format from
* the channel, and behave based on it.
*/
pub enum CastType {
    Unicast{dest:u32, bytes:Bytes},
    Multicast{bytes:Bytes},
}
/*
* PeerReceiverHandler is struct for the communication between receiver and main
* logic.
* The receiver will receive from other peers, and send the received bytes to the
* main logic immediately. The receiver will call the handler’s `dispatch()`
* method, and the method defines the behavior which sends the bytes via the
* channel to the main logic.
*/
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