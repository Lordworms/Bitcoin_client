use crate::transaction::transaction::SignedTransaction;
use std::collections::HashMap;
use crate::crypto::hash::{H256, Hashable};

/// Store all the received valid transactions which have not been included in the blockchain yet.
pub struct Mempool {
    // TODO Optional: you may use other data structures if you wish.
    pub hash_to_transaction: HashMap<H256, SignedTransaction>,
}

impl Mempool {
    pub fn new() -> Self {
        Mempool {
            hash_to_transaction: HashMap::new(),
        }
    }
    pub fn get_size(&self) -> usize{
        self.hash_to_transaction.len()
    }
    /// Get a transaction from the mempool by hash (or `None` if it does not exist)
    pub fn get_transaction(&self, hash: &H256) -> Option<&SignedTransaction> {
        self.hash_to_transaction.get(hash)
    }
    pub fn contains_hash(&self,hash:&H256)->bool{
        self.hash_to_transaction.contains_key(hash)
    }
    /// Insert a transaction into the mempool
    pub fn insert(&mut self, transaction: SignedTransaction) {
        // (Make sure you have implemented the `Hashable` trait for `SignedTransaction`, or there will be an error):
        let hash = transaction.hash();
        self.hash_to_transaction.insert(hash, transaction);
    }
    pub fn remove_transaction(&mut self,transaction_vec:Vec<SignedTransaction>){
        for trans in transaction_vec.iter(){
            if self.contains_hash(&trans.hash()){
                self.hash_to_transaction.remove(&trans.hash());
            }
        }
    }
    /// Remove a random transaction from the mempool and return it (or `None` if it is empty)
    pub fn pop(&mut self) -> Option<SignedTransaction> {
        let hash = self.hash_to_transaction.keys().next().cloned();
        if let Some(hash) = hash {
            self.hash_to_transaction.remove(&hash)
        } else {
            None
        }
    }
    pub fn is_empty(&self)->bool{
        self.hash_to_transaction.is_empty()
    }
        
    // TODO Optional: you may want to add more methods here...
}