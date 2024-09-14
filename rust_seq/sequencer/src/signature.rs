use ring::{rand, signature};
use ring::signature::{Ed25519KeyPair, KeyPair as RingKeyPair}; // Import the KeyPair trait

pub struct KeyPair {
    pub pub_key: Vec<u8>,
    keypair:  Ed25519KeyPair,
}

impl KeyPair {
    pub fn new() -> Self {
        let rng = rand::SystemRandom::new();
        let document = Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
        let keypair = Ed25519KeyPair::from_pkcs8(document.as_ref()).unwrap();
        let pub_key = keypair.public_key().as_ref().to_vec();
        KeyPair { pub_key, keypair }
    }

    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        self.keypair.sign(message).as_ref().to_vec()
    }

    pub fn verify_signature(
        pub_key: &[u8],
        message: &[u8],
        sign: &[u8]
    ) -> bool {
        let pub_key = signature::UnparsedPublicKey::new(&signature::ED25519, pub_key);
        pub_key.verify(message, sign).is_ok()
    }
}

