use ring::rand;
use ring::signature::Ed25519KeyPair;

pub fn random() -> Ed25519KeyPair {
    let rng = rand::SystemRandom::new();
    let pkcs8_bytes = Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
    Ed25519KeyPair::from_pkcs8(pkcs8_bytes.as_ref().into()).unwrap()
}
pub fn from_seed(seed: [u8; 32]) -> Ed25519KeyPair {
    Ed25519KeyPair::from_seed_unchecked(&seed).unwrap()
}