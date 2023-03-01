use std::collections::HashMap;
use crate::api::address::H160;
#[derive(Clone)]
pub struct State{
    pub accounts:HashMap<H160,(usize,usize)>//HashMap<account address, (account nonce, balance)>
}
impl State{
    pub fn new()->Self{
        let accounts:HashMap<H160,(usize,usize)>=HashMap::new();
        Self { accounts}
    }
    pub fn add_account(&mut self,addr:H160,balance:usize){
        self.accounts.insert(addr.clone(), (0,balance));
    }
    pub fn add_an_account(&mut self,addr:H160,nonce:usize,balance:usize){
        self.accounts.insert(addr.clone(), (nonce,balance));
    }
    pub fn get_accounts(&self)->HashMap<H160,(usize,usize)>{
        self.accounts.clone()
    }
    pub fn contains_address(&self,addr:&H160)->bool{
        self.accounts.contains_key(addr)
    }
}