// Copyright 2023 RISC Zero, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![no_main]
#![no_std]

use ethabi::ethereum_types::U256;
use ethabi::{ParamType, Token};
use risc0_zkvm::guest::env;
use ethers::providers::{Http, Middleware, Provider};
use ethers::providers::{Http, Middleware, Provider, StreamExt};
use ethers::types::{H256, TxHash};
use primitive_types::{H160 as Address, U256};

risc0_zkvm::guest::entry!(main);

#[tokio::main]
pub async fn main() {
    // NOTE: env::read_slice requires a length argument. Reads must be of known
    // length. https://github.com/risc0/risc0/issues/402
    let length: &[u32] = env::read_slice(1);
    let input: &[u8] = env::read_slice(length[0] as usize);
    let input = ethabi::decode_whole(&[ParamType::String, ParamType::String, ParamType::String, ParamType::Uint(256)], input).unwrap();

    let tx_hash = input[0].clone().into_string().unwrap();
    let msg_sender = input[1].clone().into_address().unwrap();
    let creditor = input[2].clone().into_address().unwrap();
    let amount: U256 = input[3].clone().into_uint().unwrap();

    println!("nice");

    let client = Provider::<Http>::try_from("https://eth.llamarpc.com").expect("Invalid RPC url");

    let tx_hash = H256::from_str(tx_hash).expect("Invalid transaction hash");

    let tx = client.get_transaction(tx_hash).await.unwrap().unwrap();

    println!("Block Hash {:?} received {:?}", tx_hash, tx.block_hash);
    println!("From {:?} received {:?}", msg_sender, tx.from);
    println!("Chain_ID {:?} received {:?}", creditor, tx.chain_id);
    println!("Gas Price{:?} received {:?}", amount, tx.value);

    // check
    let res =  creditor == tx.from && amount == tx.value && tx.to == msg_sender;

    env::commit_slice(&ethabi::encode(&[Token::Bool(res), Token::Address(msg_sender)]));
}
