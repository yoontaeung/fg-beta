use bincode::{deserialize};
use serde::{Deserialize, Serialize};
use bytes::{Bytes, BytesMut, BufMut};
const SYN_MSG:u8 = 0x0;
const SEND_MSG:u8 = 0x1;
const ECHO_MSG:u8 = 0x2;
const FIN_MSG:u8 = 0x3;
const SUP_MSG:u8 = 0x4;
const SIGN_LEN:usize = 64;
/*
* TODO: have to mind little and big endian!
* it will not cause any prob while the sender and receiver share same endian,
* but big problem will happen if they differ.
*/
#[derive(Deserialize, Serialize, Debug)]
pub enum Message {
    Syn{ sender: u32, pub_key:Vec<u8> },
    Send{ sender:u32, rn:u32, payload:Vec<u8> },
    Echo{ sender:u32, rn:u32, sign:Vec<u8> },
    Fin{
        sender:u32,
        rn:u32,
        sign_cnt:u32,
        signs:Vec<(u32, Vec<u8>)>   // taeung:: u32 for node_id??
    },
    Sup{
        sender:u32,
        rn:u32,
        sign_cnt:u32,
        signs:Vec<(u32, Vec<u8>)>,
        originator:u32,
        payload:Vec<u8>
    },
}
impl Message {
    pub fn from_bytes(bytes:Bytes) -> Result<Message, ()> {
        match bytes[0] {
            SYN_MSG => {
                Ok(Message::Syn {
                    sender: deserialize::<u32>(&bytes[1..5]).unwrap(),
                    pub_key: (bytes[5..]).to_vec(),
                })
            },
            SEND_MSG => {
                Ok(Message::Send {
                    sender: deserialize::<u32>(&bytes[1..5]).unwrap(),
                    rn: deserialize::<u32>(&bytes[5..9]).unwrap(),
                    /* TODO: to_vec() may be slow. it may deeply copy things */
                    payload: (bytes[9..]).to_vec(),
                })
            },
            ECHO_MSG => {
                Ok(Message::Echo {
                    sender: deserialize::<u32>(&bytes[1..5]).unwrap(),
                    rn: deserialize::<u32>(&bytes[5..9]).unwrap(),
                    /* TODO: to_vec() may be slow. it may deeply copy things */
                    sign: (bytes[9..]).to_vec(),
                })
            },
            FIN_MSG => {
                let sender = deserialize::<u32>(&bytes[1..5]).unwrap();
                let rn = deserialize::<u32>(&bytes[5..9]).unwrap();
                let sign_cnt = deserialize::<u32>(&bytes[9..13]).unwrap();
                let mut signs = Vec::with_capacity(sign_cnt as usize);
                let mut idx = 13;
                for _ in 0..sign_cnt {
                    let node_id = deserialize::<u32>(&bytes[idx..idx+4]).unwrap();
                    idx += 4;
                    let sign = bytes[idx..idx+SIGN_LEN].to_vec();
                    idx += SIGN_LEN;
                    signs.push((node_id, sign));
                }
                Ok(Message::Fin{ sender, rn, sign_cnt, signs })
            },
            SUP_MSG => {
                let sender = deserialize::<u32>(&bytes[1..5]).unwrap();
                let rn = deserialize::<u32>(&bytes[5..9]).unwrap();
                let sign_cnt = deserialize::<u32>(&bytes[9..13]).unwrap();
                let mut signs = Vec::with_capacity(sign_cnt as usize);
                let mut idx = 13;
                for _ in 0..sign_cnt {
                    let node_id = deserialize::<u32>(&bytes[idx..idx+4]).unwrap();
                    idx += 4;
                    let sign = bytes[idx..idx+SIGN_LEN].to_vec();
                    idx += SIGN_LEN;
                    signs.push((node_id, sign));
                }
                let originator = deserialize::<u32>(&bytes[idx..idx+4]).unwrap();
                idx += 4;
                let payload = bytes[idx..].to_vec();
                Ok(Message::Sup { sender, rn, sign_cnt, signs, originator, payload })
            },
            
            _ =>  Err(()),
        }
    }
    /* ownership? */
    pub fn to_bytes(self) -> Result<Bytes, ()> {
        match self {
            Message::Syn{sender, pub_key} => {
                let mut buf = BytesMut::with_capacity(1 + 4 + pub_key.len());
                buf.put_u8(SYN_MSG); // indicating send msg
                buf.put_u32_le(sender);
                buf.put(Bytes::from(pub_key));
                Ok(buf.into())
            }
            Message::Send{sender, rn, payload} => {
                let mut buf = BytesMut::with_capacity(payload.len() + 1 + 8);
                buf.put_u8(SEND_MSG); // indicating send msg
                buf.put_u32_le(sender);
                buf.put_u32_le(rn);
                buf.put(Bytes::from(payload));
                Ok(buf.into())
            },
            Message::Echo{sender, rn, sign} => {
                let mut buf = BytesMut::with_capacity(sign.len() + 1 + 8);
                buf.put_u8(ECHO_MSG); // indicating echo msg
                buf.put_u32_le(sender);
                buf.put_u32_le(rn);
                buf.put(Bytes::from(sign));
                Ok(buf.into())
            },
            Message::Fin{sender, rn, sign_cnt, signs} => {
                let mut buf = BytesMut::with_capacity(sign_cnt as usize*(64+4) + 1 + 12);
                buf.put_u8(FIN_MSG);
                buf.put_u32_le(sender);
                buf.put_u32_le(rn);
                buf.put_u32_le(sign_cnt);
                for (node_id, sign) in signs {
                    buf.put_u32_le(node_id);
                    buf.extend_from_slice(&sign);
                }
                Ok(buf.into())
            },
            Message::Sup { sender, rn, sign_cnt, signs, originator, payload } => {
                let mut buf = BytesMut::with_capacity(sign_cnt as usize*(64+4) + 1 + 12 + 4 + payload.len() as usize);
                buf.put_u8(SUP_MSG);
                buf.put_u32_le(sender);
                buf.put_u32_le(rn);
                buf.put_u32_le(sign_cnt);
                for (node_id, sign) in signs {
                    buf.put_u32_le(node_id);
                    buf.extend_from_slice(&sign);
                }
                buf.put_u32_le(originator);
                buf.put(Bytes::from(payload));
                Ok(buf.freeze())
            },
            
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_send_to_byte() {
        let msg = Message::Send{
            sender: 67305985,
            rn: 134678021,
            payload: [8, 9, 8, 9, 8, 9, 8, 9, 8, 9].to_vec(),
        };
        assert_eq!(
            msg.to_bytes().unwrap(), 
            [1, 1, 2, 3, 4, 5, 6, 7, 8, 8, 9, 8, 9, 8, 9, 8, 9, 8, 9].to_vec()
        );
    }

    #[test]
    fn test_send_from_byte() {
        let array:Vec<u8> = [1, 1, 2, 3, 4, 5, 6, 7, 8, 8, 9, 8, 9, 8, 9, 8, 9, 8, 9].to_vec();
        let msg =  Message::from_bytes(Bytes::from(array)).unwrap();
        if let Message::Send{sender, rn, payload} = msg {
            assert_eq!(sender, 67305985);
            assert_eq!(rn, 134678021);
            assert_eq!(payload, [8, 9, 8, 9, 8, 9, 8, 9, 8, 9].to_vec());
        }
        else { panic!(); }
    }
}