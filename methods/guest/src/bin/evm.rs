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

risc0_zkvm::guest::entry!(main);


pub fn main() {
    // NOTE: env::read_slice requires a length argument. Reads must be of known
    // length. https://github.com/risc0/risc0/issues/402
    let length: &[u32] = env::read_slice(1);
    let input: &[u8] = env::read_slice(length[0] as usize);
    let input = ethabi::decode_whole(&[ParamType::String, ParamType::String, ParamType::String, ParamType::Uint(256)], input).unwrap();

    let tx_hash = input[0].clone().into_string().unwrap();
    let msg_sender = input[1].clone().into_address().unwrap();
    let creditor = input[2].clone().into_address().unwrap();
    let amount: U256 = input[3].clone().into_uint().unwrap();

    // TODO: do stuff

    //let client = Provider::<Http>::try_from("https://eth.llamarpc.com").expect("Invalid RPC url");
    /*let client = Arc::new(client);

    let tx = client.get_transaction(tx_hash).await.unwrap().unwrap();
    let block_numb = tx.block_number.unwrap();
    info!("Running TX: 0x{:x} at block {}", tx_hash, block_numb);

    let mut env = Env::default();
    env.block.number = U256::from(block_numb.as_u64());
    env.tx = evm_core::ether_trace::txenv_from_tx(tx);
    let trace_db = evm_core::ether_trace::TraceTx::new(client, Some(block_numb.as_u64())).unwrap();

    let mut evm = EVM::new();
    evm.database(trace_db);
    evm.env = env.clone();

    let mut evm = EVM::new();
    evm.database(db);
    evm.env = env;
    let (res, state) = evm.transact();*/

    /*
    let success = true; // TODO: check if the transaction was successful, from to right person, correct amount, etc.
    env::commit_slice(&ethabi::encode(&[Token::Bool(success), Token::Address(msg_sender)]));

    env::log("");*/

    // Commit the journal that will be decoded in the application contract.
    env::commit_slice(&ethabi::encode(&[Token::Bool(true), Token::Address(msg_sender)]));
}
