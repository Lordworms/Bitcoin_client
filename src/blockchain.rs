//! This module implements the blockchain.
//! 
//! You need to implement the `Blockchain` struct and its methods.

use crate::block::Block;
use crate::crypto::hash::{H256, Hashable};
use std::collections::{HashMap, VecDeque};
use std::time::{SystemTime};
pub struct Blockchain {
    pub chains:HashMap<H256,Block>,
    pub heights:HashMap<H256,u8>,
    pub buffers:HashMap<H256,Block>,
    pub hash_tip:H256,
    pub total_delay:u128,
}

impl Blockchain {
    /// Create a new blockchain, only containing the genesis block
    pub fn new() -> Self {
        let genis=Block::genesis();
        let hash=genis.hash();
        let mut chainMap:HashMap<H256,Block>=HashMap::new();
        chainMap.insert(hash, genis);
        let mut heightsMap:HashMap<H256,u8>=HashMap::new();
        heightsMap.insert(hash,0);
        let bufferMap:HashMap<H256, Block>=HashMap::new();
        Blockchain {
            chains:chainMap,
            heights:heightsMap,
            buffers:bufferMap,
            hash_tip:hash,
            total_delay:0
        }
    }
    pub fn insert_block(&mut self,block:&Block)
    {
        
        let bhash=block.hash();
        //adjust the height and tips
        let now_height=self.heights[&block.header.parent]+1;
        self.heights.insert(bhash, now_height);
        if now_height > self.heights[&self.hash_tip]
        {
            println!("now height is {},tip height is {}",now_height,self.heights[&self.hash_tip]);
            println!("the hash is {:?} and the difficulty is {:?}",block.header.difficulty,self.hash_tip);
            self.hash_tip=bhash;
        }
        //insert into the chains
        self.chains.insert(bhash, block.clone());
        
        //adding the delay time
        let base_time=SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis();
        let delay=base_time-block.header.timestamp;
        self.total_delay+=delay;

    }
    /// Insert a block into blockchain
    pub fn insert(&mut self, block: &Block) 
    {
        let bhash=block.hash();
        match self.chains.get(&block.header.parent) {
            Some(b)=>
            {
                if !self.chains.contains_key(&bhash)
                {
                    self.insert_block(&block);
                    
                    //insert stale block (due to latency)
                    let mut remove_array:Vec<H256>=Vec::new();
                    let mut q:VecDeque<H256>=VecDeque::new();
                    q.push_back(bhash);
                    while !q.is_empty()
                    {
                        match q.pop_front()
                        {
                            Some(h)=>
                            {
                                for (bhash,blk) in self.buffers.iter()
                                {
                                    if blk.header.parent==h
                                    {
                                        //making copy and adding into chains
                                        let hash_copy=*bhash;
                                        remove_array.push(hash_copy);
                                        self.chains.insert(hash_copy, blk.clone());

                                        //adding delay
                                        let base_time=SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis();
                                        let delay=base_time-blk.header.timestamp;
                                        self.total_delay+=delay;

                                        //adding height and replace tips
                                        let now_height=self.heights[&blk.header.parent]+1;
                                        self.heights.insert(hash_copy, now_height);
                                        if now_height>self.heights[&self.hash_tip]
                                        {
                                            println!("now height is {},tip height is {}",now_height,self.heights[&self.hash_tip]);
                                            self.hash_tip=hash_copy;
                                        }
                                    }
                                }
                            },
                            None=>()
                        }
                    }
                    for hash in remove_array
                    {
                        self.buffers.remove(&hash);
                    }
                    println!("insert success!\n");
                }
                else  
                {
                    println!("insert fail!\n");    
                    println!("the hash is {:?} and the difficulty is {:?}",block.header.difficulty,self.hash_tip);
                }
            },
            _=>
            {
                if !self.chains.contains_key(&bhash)
                {
                    self.buffers.insert(bhash, block.clone());
                }
            },
        }
    }

    /// Get the last block's hash of the longest chain
    pub fn tip(&self) -> H256 {
        self.hash_tip
    }

    /// Get the last block's hash of the longest chain
    #[cfg(any(test, test_utilities))]
    pub fn all_blocks_in_longest_chain(&self) -> Vec<H256> {
        let mut now=self.hash_tip;
        let mut buf: [u8; 32] = [0; 32];
        let mut res:Vec<H256>=vec![];
        let end=buf.into();
        while now!=end
        {
            res.push(now);
            now=self.chains[&now].header.parent;
        }
        res
    }
}


#[cfg(any(test, test_utilities))]
mod tests {
    use super::*;
    use crate::block::test::generate_random_block;
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
    }

}