use log::debug;
use serde::{Serialize,Deserialize};
use ring::signature::{Ed25519KeyPair, Signature, KeyPair, VerificationAlgorithm, EdDSAParameters,UnparsedPublicKey,ED25519};
use ring::digest::{digest,SHA256};
use crate::crypto::hash::{H256, Hashable};
use rand::{Rng, thread_rng};
use crate::api::address::H160 as Address;
use crate::crypto::key_pair;
use crate::basic::state::State;
const ADDR_SIZE:usize=20;
#[derive(Serialize, Deserialize,Debug,Default,Clone)]
pub struct Transaction 
{
   pub sender:Address,
   pub nonce:usize,
   pub receiver:Address,
   pub value:usize,
}
#[derive(Serialize, Deserialize, Debug,Default,Clone)]
pub struct SignedTransaction
{
    pub trans_raw:Transaction,
    pub signature:Vec<u8>,
    pub pub_key:Vec<u8>,
}
impl Hashable for SignedTransaction {
    fn hash(&self) -> H256 {
        let bytes = bincode::serialize(&self).unwrap();
        ring::digest::digest(&ring::digest::SHA256, &bytes).into()
    }
}
impl Hashable for Transaction{
    fn hash(&self) ->H256{
        let bytes=bincode::serialize(&self).unwrap();
        ring::digest::digest(&ring::digest::SHA256,&bytes).into()
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
pub fn generate_random_transaction() -> Transaction {
    
    let mut sender_addr_ori:[u8;ADDR_SIZE]=[0;ADDR_SIZE];
    let mut receiver_addr_ori:[u8;ADDR_SIZE]=[0;ADDR_SIZE];
    let mut rng=thread_rng();
    for i in 0..ADDR_SIZE{
        sender_addr_ori[i]=rng.gen();
        receiver_addr_ori[i]=rng.gen();
    }
    let sender_addr=Address::new(sender_addr_ori);
    let receiver_addr=Address::new(receiver_addr_ori);
    let val:usize=rng.gen();
    let nonce:usize=rng.gen();
    Transaction { sender: (sender_addr), nonce: (nonce), receiver: (receiver_addr), value: (val) }
}
 pub fn generate_random_signed_transaction_with_key(key:&Ed25519KeyPair)->SignedTransaction{
    let t = generate_random_transaction();
    let sig=sign(&t,&key);
    SignedTransaction { trans_raw: (t), signature: (sig.as_ref().to_vec()),pub_key:key.public_key().as_ref().to_vec() }
}
 impl SignedTransaction {
    /// Create a new transaction from a raw transaction and a key pair
    pub fn from_raw(raw: Transaction, key: &Ed25519KeyPair) -> SignedTransaction {
        let pub_key = key.public_key().as_ref().to_vec();
        let signature = sign(&raw, key).as_ref().to_vec();
        SignedTransaction { trans_raw:raw, pub_key, signature }
    }
    pub fn verify_by_state(&self,now_state:&State)->bool{
        let sender=self.trans_raw.sender;
        if !self.verify_signature(){
            debug!("not a valid public key, verify failed\n");
            return false
        }
        let accounts=now_state.get_accounts();
        if !accounts.contains_key(&sender){
            debug!("did not find a account in now state!\n");
            return false;
        }
        let (sender_nonce,sender_balance)=accounts.get(&sender).unwrap();
        let value=self.trans_raw.value;
        if sender_balance>&value && sender_nonce+1==self.trans_raw.nonce{
            //debug!("verify success!\n");
            return true;
        }
        false
    }
    /// Verify the signature of this transaction
    pub fn verify_signature(&self) -> bool {
        let serialized_raw = bincode::serialize(&self.trans_raw).unwrap();
        let public_key = ring::signature::UnparsedPublicKey::new(
            &ring::signature::ED25519, &self.pub_key[..]);
        public_key.verify(&serialized_raw, self.signature.as_ref()).is_ok()
    }
}
#[cfg(any(test, test_utilities))]
mod tests {
    use super::*;
    
    #[test]
    fn sign_verify() {
        let t = generate_random_transaction();
        let key = key_pair::random();
        let signature = sign(&t, &key);
        assert!(verify(&t, &(key.public_key()), &signature));
    }
}
