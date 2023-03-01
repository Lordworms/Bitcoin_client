//! This module implements the blockchain.
//! 
//! You need to implement the `Blockchain` struct and its methods.

use ring::signature::KeyPair;

use crate::basic::block::{Block, self};
use crate::crypto::hash::{H256, Hashable};
use std::collections::{HashMap, VecDeque};
use std::ops::Add;
use std::time::{SystemTime};
use crate::basic::key_pair;
use crate::basic::state::State;
use crate::api::address::H160 as Address;
pub enum Blockorigin{
    Mined,
    Recieved{delay_ms:u128}
}
pub struct Blockchain {
    chain_map:HashMap<H256,Block>,
    height_map:HashMap<H256,u8>,
    orphan_buffer:HashMap<H256,Vec<Block>>,
    hash_tip:H256,
    difficulty:H256,
    block_state:HashMap<H256,State>,
    pub hash_to_origin:HashMap<H256,Blockorigin>
}

impl Blockchain {
    /// Create a new blockchain, only containing the genesis block
    pub fn new() -> Self {
        let genis=Block::genesis();
        let hash=genis.hash();
        let mut chain_map:HashMap<H256,Block>=HashMap::new();
        let difficulty=genis.header.difficulty;
        chain_map.insert(hash, genis);
        let mut height_map:HashMap<H256,u8>=HashMap::new();
        height_map.insert(hash,0);

        let mut state=State::new();
        for i in 1..6{
            let addr_raw:[u8;20]=[i;20];
            let addr=Address::new(addr_raw);
            state.add_account(addr, 10000);   
        }
        let mut block_state:HashMap<H256, State>=HashMap::new();
        block_state.insert(hash,state);
        Blockchain {
            chain_map,
            height_map,
            difficulty: difficulty,
            orphan_buffer:HashMap::new(),
            hash_tip:hash,
            hash_to_origin:HashMap::new(),
            block_state
        }
    }
    pub fn update_state(&mut self,block:&Block)->bool{
        let mut prev_state=self.block_state.get(&block.header.parent).unwrap().clone();
        let mut valid=true;
        for transaction in block.get_content(){
            let mut sender=transaction.trans_raw.sender;
            let mut receiver=transaction.trans_raw.receiver;
            //if did not contains the sender, add the sender's infomation
            if !prev_state.contains_address(&receiver){
                prev_state.add_account(receiver, 0);
            }
            let accounts=prev_state.get_accounts();
            let (sender_nonce,sender_balance)=accounts.get(&sender).unwrap();
            let (receiver_nonce,receiver_balance)=accounts.get(&receiver).unwrap();
            let value=transaction.trans_raw.value;
            if sender_balance<&value{
                valid=false;
                break;
            }
            prev_state.add_an_account(sender, *sender_nonce+1, sender_balance-value);
            prev_state.add_an_account(receiver, *receiver_nonce, receiver_balance+value);
        }
        if valid {
            self.block_state.insert(block.hash(), prev_state.clone());
        }
        valid
    }
    /// Insert a block into blockchain
    pub fn insert(&mut self, block: &Block) {
        let hash=block.hash();
        let parent=block.header.parent;
        let par_height=self.height_map.get(&parent).unwrap();
        let son_height=par_height+1;
        self.height_map.insert(hash, son_height);
        self.chain_map.insert(hash,block.clone());
        if son_height > *self.height_map.get(&self.hash_tip).unwrap()
        {
            self.hash_tip=hash;
        }
        self.update_state(block);
    }
    pub fn parent_check(&self, block: &Block) -> bool {
        self.contain_block(&block.header.parent)
    }
    pub fn block_size(&self)->usize
    {
        self.chain_map.len()
    }
    pub fn add_to_orphans(&mut self,block:&Block){
        self.orphan_buffer.entry(block.hash()).or_insert(vec![]).push(block.clone());
    }
    pub fn all_block_delay(&self) -> Vec<u128>{
        let mut delay_vec:Vec<_>=self.hash_to_origin.values().filter_map(|blk|{
            match blk{
                Blockorigin::Mined =>
                {
                    None
                },
                Blockorigin::Recieved { delay_ms } =>
                {
                    Some(*delay_ms)
                }
            }
        }).collect();
        delay_vec.sort();
        delay_vec
    }
    /// Get the last block's hash of the longest chain
    pub fn tip(&self) -> H256 {
        self.hash_tip
    }
    pub fn get_block_state(&self,hash:&H256)->State{
        self.block_state.get(hash).unwrap().clone()
    }
    pub fn get_tip_state(&self)->State{
        self.get_block_state(&self.hash_tip)
    }
    pub fn pow_validity_check(&self, block: &Block) -> bool {
        block.hash() <= block.header.difficulty && block.header.difficulty == self.difficulty
    }
    pub fn contain_block(&self,hash:&H256) ->bool{
        self.chain_map.contains_key(&hash).into()
    }   
    pub fn insert_all(&mut self,block:&Block,out_hash:&mut Vec<H256>){
        if self.chain_map.contains_key(&block.hash()){
            return
        }
        self.insert(block);
        out_hash.push(block.hash());
        if self.orphan_buffer.contains_key(&block.hash()){
            for child in self.orphan_buffer.remove(&block.hash()).unwrap(){
                self.insert_all(&child,out_hash)
            }
        }
    }
    pub fn get_block(&self,hash:&H256)->&Block
    {
        self.chain_map.get(hash).unwrap()
    }
    pub fn average_size(&self)->usize{
        self.chain_map.values().map(|block| block.size()).sum::<usize>()/self.block_size()
    }
    /// Get the last block's hash of the longest chain
    pub fn all_blocks_in_longest_chain(&self) -> Vec<H256> {
       let mut tail_hash=self.hash_tip;
       let mut logest_chain_hashes=vec![tail_hash];
       while *self.height_map.get(&tail_hash).unwrap()> 0
       {
            tail_hash=self.chain_map.get(&tail_hash).unwrap().header.parent;
            logest_chain_hashes.push(tail_hash);
       }
       logest_chain_hashes.into_iter().rev().collect()
    }
}


#[cfg(any(test, test_utilities))]
mod tests {
    use super::*;
    use crate::basic::block::test::generate_random_block;
    use crate::crypto::hash::Hashable;

    #[test]
    fn insert_one() {
        let mut blockchain = Blockchain::new();
        let genesis_hash = blockchain.tip();
        let block = generate_random_block(&genesis_hash);
        blockchain.insert(&block);
        assert_eq!(blockchain.tip(), block.hash());
    }

    #[test]
    fn mp1_insert_chain() {
        let mut blockchain = Blockchain::new();
        let genesis_hash = blockchain.tip();
        let mut block = generate_random_block(&genesis_hash);
        blockchain.insert(&block);
        assert_eq!(blockchain.tip(), block.hash());
        for _ in 0..50 {
            let h = block.hash();
            block = generate_random_block(&h);
            blockchain.insert(&block);
            assert_eq!(blockchain.tip(), block.hash());
        }
    }

    #[test]
    fn mp1_insert_3_fork_and_back() {
        let mut blockchain = Blockchain::new();
        let genesis_hash = blockchain.tip();
        let block_1 = generate_random_block(&genesis_hash);
        blockchain.insert(&block_1);
        assert_eq!(blockchain.tip(), block_1.hash());
        let block_2 = generate_random_block(&block_1.hash());
        blockchain.insert(&block_2);
        assert_eq!(blockchain.tip(), block_2.hash());
        let block_3 = generate_random_block(&block_2.hash());
        blockchain.insert(&block_3);
        assert_eq!(blockchain.tip(), block_3.hash());
        let fork_block_1 = generate_random_block(&block_2.hash());
        blockchain.insert(&fork_block_1);
        assert_eq!(blockchain.tip(), block_3.hash());
        let fork_block_2 = generate_random_block(&fork_block_1.hash());
        blockchain.insert(&fork_block_2);
        assert_eq!(blockchain.tip(), fork_block_2.hash());
        let block_4 = generate_random_block(&block_3.hash());
        blockchain.insert(&block_4);
        assert_eq!(blockchain.tip(), fork_block_2.hash());
        let block_5 = generate_random_block(&block_4.hash());
        blockchain.insert(&block_5);
        assert_eq!(blockchain.tip(), block_5.hash());
        let block_6=generate_random_block(&block_1.hash());
        blockchain.insert(&block_6);
        assert_eq!(blockchain.tip(),block_5.hash());
    }

}