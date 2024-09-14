use std::net::{SocketAddr, IpAddr, Ipv4Addr};
use std::error::Error;
use std::sync::Arc;
use std::collections::HashSet;
use std::time::{Instant};
use tokio::sync::mpsc as tokio_mpsc;
use tokio::sync::{Mutex as tk_mutex, RwLock as tk_rwlock};
use tokio::time as tk_time;
use bytes::Bytes;
use async_trait::async_trait;
use network::{Receiver, MessageHandler, Writer, SimpleSender};
use message::Message;
use ring::digest;

use crate::signature::KeyPair;

#[cfg(test)]
#[path = "tests/sequencer_tests.rs"]
pub mod sequencer_tests;

type U8Arr = Vec<u8>;

pub struct MeasureDs {
    total_sent:tk_mutex<Vec<usize>>,
    total_recv:tk_mutex<Vec<usize>>,
    bytes_sent:tk_mutex<usize>,
    bytes_recv:tk_mutex<usize>,
    round_start:tk_mutex<Vec<Instant>>,
    deliver_latency:tk_mutex<Vec<u128>>,
}
impl MeasureDs {
    pub fn new() -> Self {
        Self {
            total_sent: tk_mutex::new(Vec::new()),
            total_recv: tk_mutex::new(Vec::new()),
            bytes_sent: tk_mutex::new(0),
            bytes_recv: tk_mutex::new(0),
            round_start: tk_mutex::new(Vec::new()),
            deliver_latency: tk_mutex::new(Vec::new()),
        }
    }
    async fn incr_bytes_sent(&self, len:usize){
        let mut bytes_sent = self.bytes_sent.lock().await;
        *bytes_sent += len;
    }

    async fn incr_bytes_recv(&self, len:usize){
        let mut bytes_recv = self.bytes_recv.lock().await;
        *bytes_recv += len;
    }

    async fn append_round(&self) {
        {
            let mut total_sent = self.total_sent.lock().await;
            let mut bytes_sent = self.bytes_sent.lock().await;
            total_sent.push(*bytes_sent);
            *bytes_sent = 0;
        }
        {
            let mut total_recv = self.total_recv.lock().await;
            let mut bytes_recv = self.bytes_recv.lock().await;
            total_recv.push(*bytes_recv);
            *bytes_recv = 0;
        }
        let mut round_start = self.round_start.lock().await;
        round_start.push(Instant::now());
    }

    async fn measure_latency(&self, rn:usize){
        let mut deliver_latency = self.deliver_latency.lock().await;
        while deliver_latency.len() <= rn {
            deliver_latency.push(0);
        }
        let round_start = self.round_start.lock().await;
        deliver_latency[rn] = round_start[rn].elapsed().as_millis();
    }

    pub async fn write_measurements(
        &self, 
        filename:String,
        node_ind:u32,
        node_num:u32, 
        payload_size:usize
    ){
        use std::fs::File;
        use std::io::Write;
        let mut file = File::create(filename).unwrap();
        let total_sent = self.total_sent.lock().await;
        let total_recv = self.total_recv.lock().await;
        let deliver_latency = self.deliver_latency.lock().await;

        _ = writeln!(file, "index: {}, node_num: {}, payload_size: {}", 
            node_ind, 
            node_num, 
            payload_size
        );
        _ = writeln!(file, "deliver_latency(ms) total_sent(byte) total_recv(byte)");
        for i in 0..total_sent.len() {
            if deliver_latency.len() > i {
                _ = writeln!(file, "r{:03}: {} {} {}", 
                    i,
                    deliver_latency[i], 
                    total_sent[i], 
                    total_recv[i]
                );
            }
            else {
                _ = writeln!(file, "r{:03}: INF {} {}", 
                    i,
                    total_sent[i], 
                    total_recv[i]
                );
            }
        }

    }
}

pub struct Sequencer {
    node_ind: u32,
    num_nodes: u32,
    f_cnt: usize,
    payload_size: usize,

    /* address related */
    self_addr: SocketAddr,
    address_book: Vec<SocketAddr>,

    /* key related */
    keypair: Arc<KeyPair>,
    peer_pkeys: Arc<tk_rwlock<Vec<Option<U8Arr>>>>,

    /* transactions and data related */
    tx_list: Arc<Vec<tk_rwlock<Vec<U8Arr>>>>, // txs[0][1][2] -> peer 0's msg of round 1, the third u8
    hash_list: Arc<Vec<tk_rwlock<Vec<U8Arr>>>>, // TODO: vec<arc<rwlock<vec<u8arr>>>>
    echo_list: Arc<tk_rwlock<Vec<Vec<(u32, U8Arr)>>>>, // signs[0][1] -> second (peer index, sign) in round 0

    /* checks if a node has sent message to peers */
    sent_echo: Arc<Vec<tk_mutex<Vec<bool>>>>,  // sent_echo[0][1] -> sent echo to sender 0 in round 1
    sent_fin: Arc<tk_mutex<Vec<bool>>>,   // sent_fin[0] -> round 0, sent finals to peers
    sent_sup: Arc<Vec<tk_mutex<Vec<bool>>>>,   // sent_sup[0][1] -> sent sup to sender 0 in round 1

    /* deliver related */
    delivered: Arc<Vec<tk_rwlock<Vec<bool>>>>,   // delivered[0][1]  -> peer 0's msg in round 1 is delivered. 
    recv_sup_cnt: Arc<Vec<tk_rwlock<Vec<u8>>>>, // recv_sup_cnt[0][1] -> cnt of peer 0's sup msg recv for round 1

    /* thruput, latency measurements */
    measure: Arc<MeasureDs>,
}

impl Sequencer {
    pub fn new(
        node_ind:u32, 
        num_nodes:u32, 
        address_book:Vec<SocketAddr>,
        payload_size:usize,
        measure:Arc<MeasureDs>,
    ) -> Self {
        let mut peer_pkeys = Vec::with_capacity(num_nodes as usize);
        let mut tx_list = Vec::with_capacity(num_nodes as usize);
        let mut hash_list = Vec::with_capacity(num_nodes as usize);
        let mut sent_echo = Vec::with_capacity(num_nodes as usize);
        let mut sent_sup = Vec::with_capacity(num_nodes as usize);
        let mut delivered = Vec::with_capacity(num_nodes as usize);
        let mut recv_sup_cnt = Vec::with_capacity(num_nodes as usize);

        for _i in 0..num_nodes {
            peer_pkeys.push(None);
            tx_list.push(tk_rwlock::new(Vec::new()));
            hash_list.push(tk_rwlock::new(Vec::new()));
            sent_echo.push(tk_mutex::new(Vec::new()));
            sent_sup.push(tk_mutex::new(Vec::new()));
            delivered.push(tk_rwlock::new(Vec::new()));
            recv_sup_cnt.push(tk_rwlock::new(Vec::new()));
        }

        Sequencer {
            /* basic info */
            node_ind,
            num_nodes,
            f_cnt: (num_nodes as usize - 1) / 3,
            payload_size,
            /* address */
            self_addr: address_book[node_ind as usize],
            address_book,
            /* keys */
            keypair: Arc::new(KeyPair::new()),
            peer_pkeys: Arc::new(tk_rwlock::new(peer_pkeys)),
            /* transactions */
            tx_list: Arc::new(tx_list),
            hash_list: Arc::new(hash_list),
            echo_list: Arc::new(tk_rwlock::new(Vec::new())),
            /* checking flags */
            sent_echo: Arc::new(sent_echo),
            sent_fin: Arc::new(tk_mutex::new(Vec::new())),
            sent_sup: Arc::new(sent_sup),
            /* deliver */
            delivered: Arc::new(delivered),
            recv_sup_cnt: Arc::new(recv_sup_cnt),
            measure,
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
        let measure = self.measure.clone();
        tokio::spawn(async move {
            let mut msg_sender = SimpleSender::new();
            msg_sender.init(Message::to_bytes(syn_msg).unwrap(), peers.clone()).await;
            loop {
                if let Some(msg) = rx_send.recv().await {
                    match msg {
                        CastType::Multicast {bytes} => {
                            measure.incr_bytes_sent(bytes.len() * peers.len()).await;
                            msg_sender.broadcast(peers.clone(), bytes).await;
                        }
                        CastType::Unicast {dest, bytes} => {
                            measure.incr_bytes_sent(bytes.len()).await;
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
    ){
        let node_ind = self.node_ind;

        let payload_size = self.payload_size;
        let tx_list = Arc::clone(&self.tx_list);
        let hash_list = Arc::clone(&self.hash_list);
        let echo_list = Arc::clone(&self.echo_list);
        let keypair = Arc::clone(&self.keypair);
        let measure = self.measure.clone();

        tokio::spawn(async move {
            tk_time::sleep(tk_time::Duration::from_secs(5)).await;

            let mut interval = tk_time::interval(tk_time::Duration::from_millis(1000));
            let mut curr_round = 0;
            let usize_ind = node_ind as usize;
            let payload = vec![node_ind as u8; payload_size];

            loop {
                interval.tick().await;
                println!("--- sending message from round {} --- ", curr_round);
                measure.append_round().await;

                let payload_digest = digest::digest(&digest::SHA256, &payload);
                // append self transactions
                { 
                    let mut tx_list = tx_list[usize_ind].write().await;
                    tx_list.push(payload.clone());
                }
                // append self H(transactions)
                { 
                    let mut hash_list = hash_list[usize_ind].write().await;
                    hash_list.push(payload_digest.as_ref().to_vec());
                }
                // append self S(H(transactions))
                append_echo(
                    &echo_list, 
                    curr_round, 
                    node_ind, 
                    keypair.sign(payload_digest.as_ref())
                ).await;

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

                curr_round += 1;
            }
        });
    }

    pub async fn run_main_loop(
        self,
        mut rx_recv:tokio_mpsc::Receiver<Bytes>,
        tx_send:tokio_mpsc::Sender<CastType>
    ){
        loop{
            if let Some(bytes) = rx_recv.recv().await {
                self.measure.incr_bytes_recv(bytes.len()).await;
                match Message::from_bytes(bytes).unwrap() {
                    Message::Syn{sender, pub_key} => {
                        let mut pkeys = self.peer_pkeys.write().await;
                        if pkeys[sender as usize] == None {
                            pkeys[sender as usize] = Some(pub_key);
                        }
                        else { panic!("peer {} sent pkey twice!", sender); }
                    }
                    Message::Send{sender, rn, payload} => {
                            let keypair = self.keypair.clone();
                            let sent_echo = self.sent_echo.clone();
                            let tx_list = self.tx_list.clone();
                            let hash_list = self.hash_list.clone();
                            let tx_send = tx_send.clone();
                            tokio::spawn(async move {
                                handle_send_msg(
                                self.node_ind,
                                sender as usize,
                                rn as usize,
                                payload,
                                keypair,
                                tx_list,
                                hash_list,
                                sent_echo,
                                tx_send,
                            ).await;
                        });
                    },
                    Message::Echo{sender, rn, sign} => {
                        let self_node_ind = self.node_ind;
                        let num_nodes = self.num_nodes;
                        let sent_fin = self.sent_fin.clone();
                        let peer_pkeys = self.peer_pkeys.clone();
                        let hash_list = self.hash_list.clone();
                        let echo_list = self.echo_list.clone();
                        let delivered = self.delivered.clone();
                        let recv_sup_cnt = self.recv_sup_cnt.clone();
                        let tx_send = tx_send.clone();
                        tokio::spawn(async move {
                            handle_echo_msg(
                                self_node_ind,
                                sender as usize,
                                rn as usize,
                                sign,
                                num_nodes as usize, // should change to 2f+1
                                sent_fin,
                                peer_pkeys,
                                hash_list,
                                echo_list,
                                delivered,
                                recv_sup_cnt,
                                tx_send,
                            ).await;
                        });
                    },
                    Message::Fin{sender, rn, sign_cnt, signs} => {
                        let self_node_ind = self.node_ind;
                        let num_nodes = self.num_nodes;
                        let sent_sup = self.sent_sup.clone();
                        let hash_list = self.hash_list.clone();
                        let peer_pkeys = self.peer_pkeys.clone();
                        let delivered = self.delivered.clone();
                        let recv_sup_cnt = self.recv_sup_cnt.clone();
                        let tx_list = self.tx_list.clone();
                        let tx_send = tx_send.clone();
                        tokio::spawn(async move {
                            handle_fin_msg(
                                self_node_ind,
                                sender as usize,
                                rn as usize,
                                sign_cnt as usize,
                                num_nodes,
                                signs,
                                sent_sup,
                                hash_list,
                                peer_pkeys,
                                delivered,
                                recv_sup_cnt,
                                tx_list,
                                tx_send,
                            ).await;
                        });
                    },
                    Message::Sup{ rn, originator, .. } => {
                        let threashold = self.f_cnt * 2 + 1;
                        let delivered = self.delivered.clone();
                        let recv_sup_cnt = self.recv_sup_cnt.clone();
                        let measure = self.measure.clone();
                        let self_node_ind = self.node_ind;
                        tokio::spawn(async move {
                            handle_sup_msg(
                                self_node_ind as usize,
                                originator as usize,
                                rn as usize,
                                threashold,
                                delivered,
                                recv_sup_cnt,
                                measure,
                            ).await;
                        });
                    },
                }
            };
        }
    } // end of run_main_loop()

} // end of impl Sequencer

async fn handle_send_msg(
    self_node_ind:u32,
    sender:usize, 
    rn:usize, 
    payload:U8Arr,
    keypair:Arc<KeyPair>,
    tx_list:Arc<Vec<tk_rwlock<Vec<U8Arr>>>>,
    hash_list:Arc<Vec<tk_rwlock<Vec<U8Arr>>>>,
    sent_echo:Arc<Vec<tk_mutex<Vec<bool>>>>,
    tx_send:tokio_mpsc::Sender<CastType>
){
    let payload_digest = digest::digest(&digest::SHA256, &payload);
    {
        let mut sent_echo = sent_echo[sender].lock().await;
        while sent_echo.len() <= rn {
            sent_echo.push(false);
        }

        if sent_echo[rn] == false {
            sent_echo[rn] = true;
            drop(sent_echo);

            tx_send.send(CastType::Unicast{
                dest: sender as u32,
                bytes: Message::Echo{
                    sender: self_node_ind,
                    rn: rn as u32,
                    sign: keypair.sign(payload_digest.as_ref()),
                }
                .to_bytes()
                .unwrap()
            })
            .await
            .expect("failed to send echo msg");

            {
                let mut tx_list = tx_list[sender].write().await;
                while tx_list.len() <= rn {
                    tx_list.push(Vec::new());
                }
                tx_list[rn] = payload;
            }
            {
                let mut hash_list = hash_list[sender].write().await;
                while hash_list.len() <= rn {
                    hash_list.push(Vec::new());
                }
                hash_list[rn] = payload_digest.as_ref().to_vec();
            }
        }
    }
}

async fn handle_echo_msg(
    self_node_ind:u32,
    sender:usize, 
    rn:usize, 
    sign:U8Arr, 
    echo_threashold:usize,
    sent_fin:Arc<tk_mutex<Vec<bool>>>,
    peer_pkeys:Arc<tk_rwlock<Vec<Option<U8Arr>>>>,
    hash_list:Arc<Vec<tk_rwlock<Vec<U8Arr>>>>,
    echo_list:Arc<tk_rwlock<Vec<Vec<(u32, U8Arr)>>>>,
    delivered:Arc<Vec<tk_rwlock<Vec<bool>>>>,
    recv_sup_cnt:Arc<Vec<tk_rwlock<Vec<u8>>>>,
    ref tx_send:tokio_mpsc::Sender<CastType>
){

    {
        let peer_pkeys = peer_pkeys.read().await;
        let hash = hash_list[self_node_ind as usize].read().await;
        if !KeyPair::verify_signature(
            &peer_pkeys[sender].as_ref().unwrap(),
            &hash[rn],
            &sign
        ){
            println!("wrong signature!!");
            return;
        }

    }
    append_echo(&echo_list, rn, sender as u32, sign).await;

    // TODO: heuristically wait for other f peers
    let mut sent_fin = sent_fin.lock().await;
    while sent_fin.len() <= rn {
        sent_fin.push(false);
    }
    if !sent_fin[rn] 
        && got_enough_echo(&echo_list, rn, echo_threashold).await 
    {
        sent_fin[rn] = true;
        drop(sent_fin);

        let echo_list = echo_list.read().await;
        tx_send.send(CastType::Multicast{
            bytes: Message::Fin{
                sender: self_node_ind,
                rn: rn as u32,
                sign_cnt: echo_list[rn].len() as u32,
                signs: echo_list[rn].clone(),
            }
            .to_bytes()
            .unwrap(),
        })
        .await
        .expect("failed to send fin msg to peers");

        tx_send.send(CastType::Multicast{
            bytes: Message::Sup{
                sender: self_node_ind,
                rn: rn as u32,
                sign_cnt: echo_list[rn].len() as u32,
                signs: echo_list[rn].clone(),
                originator: self_node_ind,
                // TODO: distinguish, assume optimistic case for now
                payload:Vec::new(), 
            }
            .to_bytes()
            .unwrap(),
        })
        .await
        .expect("failed to send fin msg to peers");

        drop(echo_list);

        {
            let mut delivered = delivered[self_node_ind as usize].write().await;
            let mut recv_sup_cnt = recv_sup_cnt[self_node_ind as usize].write().await;
            while delivered.len() <= rn {
                delivered.push(false);
                recv_sup_cnt.push(0);
            }
            recv_sup_cnt[rn] += 1;
        }
    }
} // end of handle_echo_msg()

// TODO: after sending final message, should send sup message too
async fn handle_fin_msg(
    self_node_ind:u32,
    sender:usize,
    rn:usize,
    sign_cnt:usize,
    num_nodes:u32,
    sign_list:Vec<(u32, U8Arr)>,
    sent_sup:Arc<Vec<tk_mutex<Vec<bool>>>>,
    hash_list:Arc<Vec<tk_rwlock<Vec<U8Arr>>>>,
    peer_pkeys:Arc<tk_rwlock<Vec<Option<U8Arr>>>>,
    delivered:Arc<Vec<tk_rwlock<Vec<bool>>>>,
    recv_sup_cnt:Arc<Vec<tk_rwlock<Vec<u8>>>>,
    tx_list:Arc<Vec<tk_rwlock<Vec<U8Arr>>>>,
    ref tx_send:tokio_mpsc::Sender<CastType>
){
    if sign_list.len() != sign_cnt {
        eprintln!("Error: Mismatched sign count");
        return;
    }

    {
        let mut sent_sup = sent_sup[sender].lock().await;
        while sent_sup.len() <= rn {
            sent_sup.push(false);
        }
        if sent_sup[rn] == true {
            return;
        }
        else {sent_sup[rn] = true; }
    }

    let mut valid_signatures = 0;
    {
        let hashes = hash_list[sender].read().await;
        if let Some(h_tx) = hashes.get(rn) {
            for (signer_id, sign) in sign_list.iter() {
                if *signer_id == self_node_ind {
                    valid_signatures += 1;
                    continue;
                }

                let pkey = peer_pkeys.read().await;
                if KeyPair::verify_signature(
                    &pkey[*signer_id as usize].clone().unwrap(), 
                    h_tx, 
                    sign
                ) {
                    valid_signatures += 1;
                }
            }
        }
        else { eprintln!("hash not found!"); return; }
    }
    
    let f_cnt = (num_nodes - 1) / 3;
    if valid_signatures >= 2 * f_cnt + 1 {
        let signers_set: HashSet<u32> = sign_list.iter().map(|(id, _)| *id).collect();
        for i in 0..num_nodes {
            if i == self_node_ind {
                {
                    let mut delivered = delivered[sender].write().await;
                    let mut recv_sup_cnt = recv_sup_cnt[sender].write().await;
                    while delivered.len() <= rn {
                        delivered.push(false);
                        recv_sup_cnt.push(0);
                    }
                    recv_sup_cnt[rn] += 1;
                }
                continue;
            }
            let sup_msg = if signers_set.contains(&i) {
                Message::Sup {
                    sender: self_node_ind,
                    rn: rn as u32,
                    sign_cnt: sign_cnt as u32, 
                    signs: sign_list.clone(),
                    originator: sender as u32,
                    payload: Vec::new(), // Empty payload
                }
            } 
            else {
                Message::Sup {
                    sender: self_node_ind,
                    rn: rn as u32,
                    sign_cnt: sign_cnt as u32, // No signs included
                    signs: sign_list.clone(),
                    originator: sender as u32,
                    payload: tx_list[sender].read().await[rn].clone(),
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
        eprintln!("Verification failed. Not enough valid signatures: {} / {}", valid_signatures, 2 * f_cnt + 1);
        // TODO: Handle insufficient valid signatures
    } 
} // end of handle_fin_msg()

async fn handle_sup_msg(
    self_node_ind:usize,
    originator:usize,
    rn: usize,
    threashold: usize,
    delivered:Arc<Vec<tk_rwlock<Vec<bool>>>>,
    recv_sup_cnt:Arc<Vec<tk_rwlock<Vec<u8>>>>,
    measure:Arc<MeasureDs>,
){
    let mut delivered = delivered[originator].write().await;
    let mut recv_sup_cnt = recv_sup_cnt[originator].write().await;
    while delivered.len() <= rn {
        delivered.push(false);
        recv_sup_cnt.push(0);
    }
    recv_sup_cnt[rn] += 1;
    /* 
     * TODO: amplify
     * chk sent_sup's length, 
     * send() if got more than f+1 sup msg and has not sent sup msg
    */
    if delivered[rn] == false && recv_sup_cnt[rn] as usize >= threashold {
        delivered[rn] = true;
        println!("{}'s msg for round {} is delivered!", originator, rn);
        if originator == self_node_ind {
            measure.measure_latency(rn).await;
        }
    }
}

async fn append_echo(
    ref echo_list:&Arc<tk_rwlock<Vec<Vec<(u32, U8Arr)>>>>, 
    rn:usize, 
    sender:u32, 
    sign:U8Arr
){
    let mut echo_list = echo_list.write().await;
    while echo_list.len() <= rn {
        echo_list.push(Vec::<(u32, U8Arr)>::new());
    }
    echo_list[rn].push((sender, sign));
}

async fn got_enough_echo(
    ref echo_list:&Arc<tk_rwlock<Vec<Vec<(u32, U8Arr)>>>>, 
    rn:usize,
    echo_threashold:usize
) -> bool {
    if let Some(echo_list_rn) = echo_list.read().await.get(rn){
        if echo_list_rn.len() == echo_threashold {
            return true;
        }
    }
    false
}

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