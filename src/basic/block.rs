//! This module defines the layout of a block.
//! 
//! You do not need to modify this file, except for the `default_difficulty` function.
//! Please read this file to understand the structure of a block.

use serde::{Serialize, Deserialize};
use crate::crypto::hash::{H256, Hashable};
use crate::crypto::key_pair::random;
use crate::transaction::transaction::SignedTransaction as Transaction;
//use std::collections::hash_map::RawEntryMut;
use std::time::{SystemTime};
use ring::{digest};
/// The block header
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Header {
    pub parent: H256,
    pub nonce: u32,
    pub difficulty: H256,
    pub timestamp: u128,
    pub merkle_root: H256,
}

/// Transactions contained in a block,the transaction is Signed transaction
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Content {
    pub transactions: Vec<Transaction>,
}

/// A block in the blockchain
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    pub header: Header,
    pub content: Content,
}

/// Returns the default difficulty, which is a big-endian 32-byte integer.
/// - Note: a valid block must satisfy that `block.hash() <= difficulty`.
///   In other words, the _smaller_ the `difficulty`, the harder it actually is to mine a block!
fn random_difficulty() -> H256 {
    // TODO: it's up to you to determine an appropriate difficulty.
    // For example, after executing the code below, `difficulty` represents the number 256^31.
    //
    // let mut difficulty = [0u8; 32];
    // difficulty[0] = 1;
    // difficulty
    let random = digest::digest(&ring::digest::SHA256,"af921c067ad9d870216ed5f7afae984258a35205a3f6eb65b46a0939decc823c
    ".as_bytes());
    let b = <H256>::from(random);
    b
}
fn default_difficulty()-> [u8; 32]{
    let mut difficulty = [0u8; 32];
    difficulty[0] = 1;
    difficulty
}
impl Block {
    /// Construct the (totally deterministic) genesis block
    pub fn genesis() -> Block {
        let transactions: Vec<Transaction> = vec![];
        let header = Header {
            parent: Default::default(),
            nonce: 0,
            difficulty:default_difficulty().into(),
            timestamp: 0,
            merkle_root:Default::default()
        };
        let content = Content { transactions };
        Block { header, content }
    }
    pub fn size(&self)->usize{
        bincode::serialize(&self).unwrap().len()
    }
    pub fn get_content(&self)->Vec<Transaction>{
        let mut trans_vec=Vec::new();
        for trans in &self.content.transactions{
            trans_vec.push(trans.clone());
        }
        trans_vec
    }
}

impl Hashable for Header {
    /// Hash the block header using SHA256.
    fn hash(&self) -> H256 {
        let bytes = bincode::serialize(&self).unwrap();
        ring::digest::digest(&ring::digest::SHA256, &bytes).into()
    }
}

impl Hashable for Block {
    /// Hash only the block header.
    fn hash(&self) -> H256 {
        self.header.hash()
    }
}

/* Please add the following code snippet into `src/transaction.rs`: */
// impl Hashable for Transaction {
//     fn hash(&self) -> H256 {
//         let bytes = bincode::serialize(&self).unwrap();
//         ring::digest::digest(&ring::digest::SHA256, &bytes).into()
//     }
// }

#[cfg(any(test, test_utilities))]
pub mod test {
    use super::*;
    use crate::crypto::hash::H256;
    use crate::crypto::merkle::MerkleTree;
    
    pub fn generate_random_block(parent: &H256) -> Block {
        let transactions: Vec<Transaction> = vec![Default::default()];
        let root = MerkleTree::new(&transactions).root();
        let header = Header {
            parent: *parent,
            nonce: rand::random(),
            difficulty: default_difficulty().into(),
            timestamp: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).expect("Time went backwards").as_millis(),
            merkle_root: root,
        };
        let content = Content { transactions };
        Block { header, content }
    }
}