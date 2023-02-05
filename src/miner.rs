use crate::blockchain;
use crate::crypto::hash::{H256, Hashable};
use crate::network::server::Handle as ServerHandle;
use std::sync::{Arc, Mutex};
use log::info;
use rand::Rng;
use crate::block::{Content, Header, Block};
use crate::transaction::{generate_random_signed_transaction, SignedTransaction};
use crate::blockchain::Blockchain;
use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use std::time;
use std::thread;

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
    /// Channel for receiving control signal
    blockchain: Arc<Mutex<Blockchain>>,
    control_chan: Receiver<ControlSignal>,
    operating_state: OperatingState,
    server: ServerHandle,
    num_mined:u8,
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
        num_mined: 0
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
            }
            ControlSignal::Start(i) => {
                info!("Miner starting in continuous mode with lambda {}", i);
                self.operating_state = OperatingState::Run(i);
            }
        }
    }

    fn miner_loop(&mut self) {
        // main mining loop
        let start_time=time::Instant::now();
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

            let parent=self.blockchain.lock().unwrap().tip();
            let difficulty=self.blockchain.lock().unwrap().chains[&parent].header.difficulty;
            let root=H256::from([0;32]);//merkle root
            let signed_trans=generate_random_signed_transaction();
            let mut trans_vec:Vec<SignedTransaction>=vec![];
            trans_vec.push(signed_trans);
            let mut rng = rand::thread_rng();
            let contents=Content{transactions:trans_vec};
            let nonce:u32=rng.gen();
            let timestamp=time::SystemTime::now().duration_since(time::SystemTime::UNIX_EPOCH).unwrap().as_millis();
            let new_header=Header{parent,nonce,difficulty,timestamp,merkle_root:root};
            let new_block=Block{header:new_header,content:contents};
            let new_hash=new_block.hash();
            if new_block.hash()<=difficulty
            {
                self.num_mined+=1;
                self.blockchain.lock().unwrap().insert(&new_block);
                let now_time=time::Instant::now();
                println!("the current difficulty is {}",difficulty);
                println!("mined {} blocks, the time has passed {:?}, the new block's hash value is {:?}\n",self.num_mined,now_time.checked_duration_since(start_time),new_block.hash());
            }
            if self.num_mined>=100
            {
                break;
            }
            if let OperatingState::Run(i) = self.operating_state {
                if i != 0 {
                    let interval = time::Duration::from_micros(i as u64);
                    thread::sleep(interval);
                }
            }
        }
    }
}
