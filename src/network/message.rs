use serde::{Serialize, Deserialize};
use crate::crypto::hash::H256;
use crate::block::Block;
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
    Ping(String),
    Pong(String),
    NewBlockHashes(Vec<H256>),
    GetBlocks(Vec<H256>),
    Blocks(Vec<Block>),
}
