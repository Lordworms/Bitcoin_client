use crate::crypto::merkle::{self, MerkleTree};
use crate::{blockchain, block};
use crate::crypto::hash::{H256, Hashable};
use crate::network::server::Handle as ServerHandle;
use std::sync::{Arc, Mutex};
use log::{info, debug};
use rand::Rng;
use crate::block::{Content, Header, Block};
use crate::transaction::{generate_random_signed_transaction, SignedTransaction, Transaction};
use crate::blockchain::Blockchain;
use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use std::time::{self, SystemTime, UNIX_EPOCH};
use std::thread;
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
) -> (Context, Handle) {
    let (signal_chan_sender, signal_chan_receiver) = unbounded();

    let ctx = Context {
        control_chan: signal_chan_receiver,
        operating_state: OperatingState::Paused,
        server: server.clone(),
        blockchain: Arc::clone(blockchain),
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
                let mut blockchain=self.blockchain.lock().unwrap();
                let parent=blockchain.tip();
                let timestamp=SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
                let transactions:Vec<SignedTransaction>=vec![Default::default()];
                let merkle_root=MerkleTree::new(&transactions).root();
                let nonce:u32=rand::random();
                let difficulty=blockchain.get_block(&parent).header.difficulty;
                let new_block_header=Header
                {
                    parent,
                    nonce,
                    difficulty,
                    merkle_root,
                    timestamp
                };
                let content=Content{transactions};
                let new_block=Block{
                    header:new_block_header,
                    content
                };
                
                if new_block.hash()<=difficulty{  
                    blockchain.insert(&new_block);
                    self.total_num_mined+=1;
                    //info!("Mined a new block {:?},the total number is {}",new_block,self.total_num_mined); 
                    self.server.broadcast(Message::NewBlockHashes(vec![new_block.hash()]));
                    blockchain.hash_to_origin.insert(new_block.hash(), Blockorigin::Mined);
                }
                // if blockchain.block_size() == 100{
                //     self.operating_state=OperatingState::Paused;
                // }

            }
        }
    }
}
