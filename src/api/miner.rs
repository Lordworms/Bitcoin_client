use crate::basic::mempool::Mempool;
use crate::crypto::merkle::{self, MerkleTree};
use crate::basic::block;
use crate::blockchain::blockchain;
use crate::crypto::hash::{H256, Hashable};
use crate::network::server::Handle as ServerHandle;
use crate::transaction;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use log::{info, debug};
use rand::Rng;
use crate::basic::block::{Content, Header, Block};
use crate::transaction::transaction::{SignedTransaction, Transaction};
use crate::blockchain::blockchain::Blockchain;
use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use std::time::{self, SystemTime, UNIX_EPOCH};
use std::{thread, mem};
use crate::network::message::Message;
use blockchain::Blockorigin;
enum ControlSignal {
    Start(u64), // the number controls the lambda of interval between block generation
    Exit,
}

enum OperatingState {
    Paused,
    Run(u64),
    ShutDown,
}

pub struct Context {
    control_chan:Receiver<ControlSignal>,
    operating_state:OperatingState,
    server:ServerHandle,
    blockchain:Arc<Mutex<Blockchain>>,
    mempool:Arc<Mutex<Mempool>>,
    total_num_mined: u64,
    start_time:Option<SystemTime>,
}

#[derive(Clone)]
pub struct Handle {
    /// Channel for sending signal to the miner thread
    control_chan: Sender<ControlSignal>,
}

pub fn new(
    server: &ServerHandle,
    blockchain: &Arc<Mutex<Blockchain>>,
    mempool:&Arc<Mutex<Mempool>>,
) -> (Context, Handle) {
    let (signal_chan_sender, signal_chan_receiver) = unbounded();

    let ctx = Context {
        control_chan: signal_chan_receiver,
        operating_state: OperatingState::Paused,
        server: server.clone(),
        blockchain: Arc::clone(blockchain),
        mempool:Arc::clone(mempool),
        total_num_mined: 0,
        start_time:None
    };

    let handle = Handle {
        control_chan: signal_chan_sender,
    };

    (ctx, handle)
}

impl Handle {
    pub fn exit(&self) {
        self.control_chan.send(ControlSignal::Exit).unwrap();
    }

    pub fn start(&self, lambda: u64) {
        self.control_chan
            .send(ControlSignal::Start(lambda))
            .unwrap();
    }

}

impl Context {
    pub fn start(mut self) {
        thread::Builder::new()
            .name("miner".to_string())
            .spawn(move || {
                self.miner_loop();
            })
            .unwrap();
        info!("Miner initialized into paused mode");
    }

    fn handle_control_signal(&mut self, signal: ControlSignal) {
        match signal {
            ControlSignal::Exit => {
                info!("Miner shutting down");
                self.operating_state = OperatingState::ShutDown;
                if let Some(start_time)=self.start_time
                {
                    let second_spent=SystemTime::now().duration_since(start_time).unwrap().as_secs_f64();
                    let mine_rate=(self.total_num_mined as f64)/second_spent;
                    info!("Mined {} blocks in {} time and mine rate is {}",self.total_num_mined,second_spent,mine_rate);
                    let blockchain=self.blockchain.lock().unwrap();
                    info!("Now blockchain has {} blocks",blockchain.block_size());
                    let longest_chain=blockchain.all_blocks_in_longest_chain();
                    info!("the longest chain is {:?},it has {} blcoks",longest_chain,longest_chain.len());
                    info!("average block size is {}",blockchain.average_size());
                    info!("delay for every block {:?}",blockchain.all_block_delay());
                }
            }
            ControlSignal::Start(i) => {
                info!("Miner starting in continuous mode with lambda {}", i);
                self.operating_state = OperatingState::Run(i);
                if self.start_time==None{
                    self.start_time=Some(SystemTime::now());
                }
            }
        }
    }

    fn miner_loop(&mut self) {
        // main mining loop
        loop {
            // check and react to control signals
            match self.operating_state {
                OperatingState::Paused => {
                    let signal = self.control_chan.recv().unwrap();
                    self.handle_control_signal(signal);
                    continue;
                }
                OperatingState::ShutDown => {
                    return;
                }
                _ => match self.control_chan.try_recv() {
                    Ok(signal) => {
                        self.handle_control_signal(signal);
                    }
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => panic!("Miner control channel detached"),
                },
            }
            if let OperatingState::ShutDown = self.operating_state {
                return;
            }

            if let OperatingState::Run(i) = self.operating_state {
                if i != 0 {
                    let interval = time::Duration::from_micros(i as u64);
                    thread::sleep(interval);
                }
                let mut mempool=self.mempool.lock().unwrap();
                //do if the mempool has 3 or more transactions
                if mempool.get_size()<3{
                    continue;
                }
                let mut blockchain=self.blockchain.lock().unwrap();
                let parent=blockchain.tip();
                let timestamp=SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
                let mut data:Vec<H256>=vec![];
                let nonce:u32=rand::random();
                let difficulty=blockchain.get_block(&parent).header.difficulty;
                let mut transactions:Vec<SignedTransaction>=Vec::new();
                
                // let mut tx_tracker:HashSet<Vec<u8>>=HashSet::new();
                //for every block, add 3 transactions into it
                for (hash,transaction) in mempool.hash_to_transaction.iter(){
                    if transactions.len()>=3{
                        break;
                    }
                    let now_state=blockchain.get_tip_state();
                    if transaction.verify_by_state(&now_state){
                        transactions.push(transaction.clone());
                        data.push(hash.clone());
                    }
                    else {
                        info!("transaction verify failed!\n");
                    }
                }
                if data.len()<3{
                    info!("data length is less than 3\n");
                    continue;
                }
                let merkle_root=MerkleTree::new(&data).root();
                let new_block_header=Header{
                    parent,
                    nonce,
                    difficulty,
                    merkle_root,
                    timestamp
                };
                let transaction_copy=transactions.clone();
                let content=Content{transactions};
                let new_block=Block{
                    header:new_block_header,
                    content
                };
                
                if new_block.hash()<=difficulty{  
                    debug!("mined a new block,now the block number is {}",self.total_num_mined);
                    blockchain.insert(&new_block);
                    self.total_num_mined+=1;
                    info!("Mined a new block {:?},the total number is {}",new_block,self.total_num_mined); 
                    self.server.broadcast(Message::NewBlockHashes(vec![new_block.hash()]));
                    mempool.remove_transaction(transaction_copy);
                    blockchain.hash_to_origin.insert(new_block.hash(), Blockorigin::Mined);
                }
            }
        }
    }
}
