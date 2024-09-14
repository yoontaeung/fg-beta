#![allow(unused_imports)]
use message::Message;
use bincode::{deserialize, serialize};
use serde::{Deserialize, Serialize};
use bytes::{BytesMut, Bytes, BufMut};

fn main() {
    let array:Vec<u8> = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9].to_vec();
    let _int:u32 = 67305985;
    let _zeros = BytesMut::zeroed(10);

    let msg = Message::from_bytes(Bytes::from(array));
    if let Message::Send{sender, rn, payload} = msg.unwrap() {
        println!("s {}, rn {}, pay: {:?}", sender, rn, payload);
    };
    /*
    zeros.put_u32(int);
    println!("{:?}", zeros);

    let msg = Bytes::from(vec![1;64]);
    println!("bytes : \n{:?}\n", msg);

    let msg = Message::from_bytes(msg);
    println!("msg : {:?}\n", msg);
    */

    let msg = Message::Echo{
        sender: 67305985,
        rn: 134678021,
        // payload: vec![1;32],
        sign: [9, 8, 7, 6, 5, 4, 3, 2, 1, 0, 9, 8, 7].to_vec(),
    };
    println!("\n{:?}\n", msg);

    let msg = msg.to_bytes().unwrap();
    println!("bytes again : \n{:?}\n", msg);

    let msg = Message::from_bytes(msg);
    println!("{:?}", msg);
    
}