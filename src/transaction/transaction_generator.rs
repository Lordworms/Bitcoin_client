use log::info;
use rand::Rng;
use serde::{Serialize,Deserialize};
use ring::signature::{Ed25519KeyPair, Signature, KeyPair, VerificationAlgorithm, EdDSAParameters, ED25519, UnparsedPublicKey};
use crate::crypto::hash::{H256, Hashable};

use crate::network::peer;
use crate::network::server::Handle as ServerHandle;
use crate::transaction;
use crate::transaction::transaction::{generate_random_signed_transaction_with_key, SignedTransaction, Transaction};
use std::ops::Add;
use std::thread;
use std::time;
use std::sync::{Arc, Mutex};
use crate::basic::mempool::Mempool;
use crate::network::message::Message;
use crate::blockchain::blockchain::{Blockchain,Blockorigin};
use crate::api::address::H160 as Address;

pub struct TransactionGenerator {
    server: ServerHandle,
    mempool: Arc<Mutex<Mempool>>,
    blockchain: Arc<Mutex<Blockchain>>,
    controlled_keypair: Ed25519KeyPair,
}

impl TransactionGenerator {
    pub fn new(
        server: &ServerHandle,
        mempool: &Arc<Mutex<Mempool>>,
        blockchain: &Arc<Mutex<Blockchain>>,
        controlled_keypair: Ed25519KeyPair
    ) -> TransactionGenerator {
        TransactionGenerator {
            server: server.clone(),
            mempool: Arc::clone(mempool),
            blockchain: Arc::clone(blockchain),
            controlled_keypair,
        }
    }

    pub fn start(self) {
        thread::spawn(move || {
            self.generation_loop();
            log::warn!("Transaction Generator exited");
        });
    }

    /// Generate random transactions and send them to the server
    fn generation_loop(&self) {
        const INTERVAL_MILLISECONDS: u64 = 3000; // how quickly to generate transactions
        const TRX_LIM:usize=100;
        let mut trans_cnt:usize=1;
        //initiate account
        let mut account_vec:Vec<Address>=Vec::new();
        
        for i in 1..6{
            let addr_raw:[u8;20]=[i;20];
            account_vec.push(Address::new(addr_raw));
        }
        loop {
            let interval = time::Duration::from_millis(INTERVAL_MILLISECONDS);
            thread::sleep(interval);
            if trans_cnt==TRX_LIM{
                break;
            }
            let mut rng = rand::thread_rng();
            let blockchain=self.blockchain.lock().unwrap();
            let now_state=blockchain.get_tip_state();
            let accounts=now_state.get_accounts();
            let sender_id=rng.gen_range(0, accounts.len());
            let (nonce,_)=accounts.get(&account_vec.get(sender_id).unwrap()).unwrap();
            let sender_addr:Address=account_vec.get(sender_id).unwrap().clone();
            let nonce:usize=nonce+1;
            let value:usize=100;

            let reciever_id=rng.gen_range(0,accounts.len());

            let receiver_addr:Address=account_vec.get(reciever_id).unwrap().clone();
            let trans_raw=Transaction{
                sender:sender_addr,
                receiver:receiver_addr,
                value,
                nonce,
            };
            
            let transaction=SignedTransaction::from_raw(trans_raw,&self.controlled_keypair);
            //check validation
           

            let mut mempool=self.mempool.lock().unwrap();
            let trans_hash=transaction.hash();
            mempool.insert(transaction);
            let trans_vec=vec![trans_hash];
            info!("generate a new transaction and broadcast to others! the new total number of transaction is {}\n",trans_cnt);
            info!("the total size of mempool is {}\n",mempool.get_size());
            self.server.broadcast(Message::NewTransactionHashes(trans_vec));
            trans_cnt+=1;
        }
    }
}