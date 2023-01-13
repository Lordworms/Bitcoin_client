use serde::{Serialize,Deserialize};
use ring::signature::{Ed25519KeyPair, Signature, KeyPair, VerificationAlgorithm, EdDSAParameters,UnparsedPublicKey,ED25519};
use ring::digest::{digest,SHA256};
use crate::crypto::hash::{H256, Hashable};
use rand::Rng;
#[derive(Serialize, Deserialize, Debug, Default,Clone, Eq, PartialEq, Hash)]
pub struct MsgInput
{
    pub hash:H256,
    pub index:u8
}
#[derive(Serialize, Deserialize, Debug, Clone,Copy)]
pub struct MsgOutput
{
    pub address:H256,
    pub value:u32
}
#[derive(Serialize, Deserialize, Debug, Default,Clone)]
pub struct Transaction 
{
   pub input:Vec<MsgInput>,
   pub output:Vec<MsgOutput>,
}
#[derive(Clone)]
pub struct SignedTransaction
{
    pub trans:Transaction,
    pub sig:Vec<u8>,
}
impl Hashable for Transaction
{
    fn hash(&self)->H256
    {
        let serialized_data=bincode::serialize(&self).unwrap();
        return digest(&SHA256,&serialized_data).into();
    }
}

/// Create digital signature of a transaction
pub fn sign(t: &Transaction, key: &Ed25519KeyPair) -> Signature {
    let bytes=bincode::serialize(&t).unwrap();
    key.sign(&bytes)
}

/// Verify digital signature of a transaction, using public key instead of secret key
pub fn verify(t: &Transaction, public_key: &<Ed25519KeyPair as KeyPair>::PublicKey, signature: &Signature) -> bool {
    let bytes=bincode::serialize(&t).unwrap();
    let pk=UnparsedPublicKey::new(&ED25519,public_key.as_ref().to_vec());
    let res=pk.verify(&bytes, &signature.as_ref().to_vec());
    res.is_ok()
}
pub fn generate_random_hash() -> H256 {
    let mut rng = rand::thread_rng();
    let random_bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
    let mut raw_bytes = [0; 32];
    raw_bytes.copy_from_slice(&random_bytes);
    (&raw_bytes).into()
}
#[cfg(any(test, test_utilities))]
mod tests {
    use super::*;
    use crate::crypto::key_pair;

    pub fn generate_random_transaction() -> Transaction {
       let input=vec![MsgInput{hash:generate_random_hash(),index:0}];
       let output=vec![MsgOutput{address:generate_random_hash(),value:0}];
       Transaction { input: input, output: (output) }
    }

    #[test]
    fn sign_verify() {
        let t = generate_random_transaction();
        let key = key_pair::random();
        let signature = sign(&t, &key);
        assert!(verify(&t, &(key.public_key()), &signature));
    }
}
