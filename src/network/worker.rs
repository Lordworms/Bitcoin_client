use super::message::Message;
use super::peer;
use crate::basic::block::Block;
use crate::crypto::hash::{H256, Hashable};
use crate::basic::mempool::{Mempool, self};
use crate::network::server::Handle as ServerHandle;
use crate::transaction::transaction::{SignedTransaction,Transaction};
use crossbeam::channel;
use log::{debug, warn, info};
use crate::blockchain::blockchain::{Blockchain, Blockorigin};
use std::borrow::Borrow;
use std::thread;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
#[derive(Clone)]
pub struct Context {
    msg_chan: channel::Receiver<(Vec<u8>, peer::Handle)>,
    num_worker: usize,
    server: ServerHandle,
    blockchain: Arc<Mutex<Blockchain>>,
    mempool:Arc<Mutex<Mempool>>,
}

pub fn new(
    num_worker: usize,
    msg_src: channel::Receiver<(Vec<u8>, peer::Handle)>,
    server: &ServerHandle,
    blockchain: &Arc<Mutex<Blockchain>>,
    mempool:&Arc<Mutex<Mempool>>,
) -> Context {
    Context {
        msg_chan: msg_src,
        num_worker,
        server: server.clone(),
        blockchain:Arc::clone(blockchain),
        mempool:Arc::clone(mempool),
    }
}

impl Context {
    pub fn start(self) {
        let num_worker = self.num_worker;
        for i in 0..num_worker {
            let cloned = self.clone();
            thread::spawn(move || {
                cloned.worker_loop();
                warn!("Worker thread {} exited", i);
            });
        }
    }

    fn worker_loop(&self) {
        loop {
            let msg = self.msg_chan.recv().unwrap();
            let (msg, peer) = msg;
            let msg: Message = bincode::deserialize(&msg).unwrap();
            match msg {
                Message::Ping(nonce) => {
                    debug!("Ping: {}", nonce);
                    peer.write(Message::Pong(nonce.to_string()));
                }
                Message::Pong(nonce) => {
                    debug!("Pong: {}", nonce);
                }
                Message::NewBlockHashes(hash_vec)=>{
                    info!("Get new block hashes! {:?}",hash_vec);
                    let blockchain=self.blockchain.lock().unwrap();
                    let missed_hashes:Vec<_>=hash_vec.into_iter().filter(|hash| !blockchain.contain_block(hash)).collect();
                    if !missed_hashes.is_empty()
                    {
                        peer.write(Message::GetBlocks(missed_hashes));
                    }
                }
                Message::GetBlocks(hash_vec)=>{
                    info!("Get block hashes! {:?}",hash_vec);
                    let blockchain=self.blockchain.lock().unwrap();
                    let missed_block:Vec<_>=hash_vec.iter().
                    filter(|hash| blockchain.contain_block(hash)).
                    map(|hash| blockchain.get_block(hash).clone()).collect();
                    if !missed_block.is_empty(){
                        peer.write(Message::Blocks(missed_block));
                    }
                }
                Message::Blocks(block_vec)=>{
                    info!("Get new blocks! {:?}",block_vec);
                    let now_time=SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
                    let mut blockchain=self.blockchain.lock().unwrap();
                    let mut relay_hashes:Vec<H256>=Vec::new();
                    let mut missed_hashes:Vec<H256>=Vec::new();
                    let mut mempool=self.mempool.lock().unwrap();
                    for block in block_vec{
                        blockchain.hash_to_origin.entry(block.hash()).or_insert(Blockorigin::Recieved { delay_ms:now_time - block.header.timestamp });
                        if blockchain.contain_block(&block.hash()){
                            continue;
                        }
                        if !blockchain.pow_validity_check(&block){
                            continue;
                        }
                        if !blockchain.parent_check(&block){
                            blockchain.add_to_orphans(&block);
                            missed_hashes.push(block.header.parent);
                            continue;
                        }
                        blockchain.insert_all(&block, &mut relay_hashes);
                        //remove transaction in mempool
                        mempool.remove_transaction(block.get_content());
                    }
                    if !missed_hashes.is_empty(){
                        peer.write(Message::GetBlocks(missed_hashes));
                    }
                    if !relay_hashes.is_empty(){
                        self.server.broadcast(Message::NewBlockHashes(relay_hashes));
                    }

                }
                Message::NewTransactionHashes(hash_vec)=>{
                    let mut new_hsahes:Vec<H256>=Vec::new();
                    let mempool=self.mempool.lock().unwrap();
                    for hash in hash_vec.iter(){
                        if !mempool.contains_hash(hash){
                            new_hsahes.push(hash.clone());
                        }
                    }
                    if !new_hsahes.is_empty(){
                        peer.write(Message::GetTransactions(new_hsahes));
                    }
                }
                Message::GetTransactions(hash_vec)=>{
                    let mut transactions:Vec<SignedTransaction>=Vec::new();
                    let mempool=self.mempool.lock().unwrap();
                    for hash in hash_vec.iter(){
                        match mempool.get_transaction(hash){
                            Some(transaction)=>{
                                transactions.push(transaction.clone());
                            }
                            _=>{

                            }
                        }
                    }
                    if !transactions.is_empty(){
                        peer.write(Message::Transactions(transactions));
                    }
                }
                Message::Transactions(trans_vec)=>{
                    info!("Received new transactions!,{:?}",trans_vec);
                    let mut new_hashes:Vec<H256>=Vec::new();
                    let blockchain=self.blockchain.lock().unwrap();
                    let mut mempool=self.mempool.lock().unwrap();
                    for trans in trans_vec.iter(){
                        let cur_state=blockchain.get_tip_state();
                        if trans.verify_by_state(&cur_state){
                            mempool.insert(trans.clone());
                            new_hashes.push(trans.hash());//get new hashes
                        }
                    }
                }
                _=>{

                }
            }
        }
    }
}
