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

//! Generated create containing the image ID and ELF binary of the build guest.

include!(concat!(env!("OUT_DIR"), "/methods.rs"));

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::str::FromStr;
    use std::sync::Arc;

    use clap::Parser;
    use ethers_core::types::{H256};
    use ethers_providers::{Middleware};
    use evm_core::ether_trace::{Http, Provider};
    use evm_core::{Env, EvmResult, EVM};
    use log::info;
    use risc0_zkvm::serde::{from_slice, to_vec};

    use ethabi::ethereum_types::U256;
    use ethabi::ParamType::Address;
    use ethabi::Token;
    use risc0_zkvm::{Prover, ProverOpts};
    use hex;

    use super::{EVM_ID, EVM_PATH};

    #[tokio::test]
    async fn evm() -> Result<(), Box<dyn Error>> {
        env_logger::init();

        // Add a transaction hash that contains a simple ETH transfer from creditor to depositor with the correct amount based on the L3L1Escrow on Layer 1
        let tx_hash = H256::from_str("0x671a3b40ecb7d51b209e68392df2d38c098aae03febd3a88be0f1fa77725bbd7")
            .expect("Invalid transaction hash");

        // RPC of Layer 3 (add your own RPC node here)
        let client = Provider::<Http>::try_from("https://xxxx-xxx-xxx.eu.ngrok.io").expect("Invalid RPC url");
        let client = Arc::new(client);

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

        let ((res, _state), trace_db) =
            tokio::task::spawn_blocking(move || (evm.transact(), evm.take_db()))
                .await
                .unwrap();

        // Stop for simple transfers (success code without return val)
        if res.exit_reason != evm_core::Return::Return && res.exit_reason != evm_core::Return::Stop {
            println!("TX failed in pre-flight, {:?}", res.exit_reason);
            panic!("TX failed in pre-flight");
        }

        let zkdb = trace_db.create_zkdb();

        // Skip seal as it is not needed to test the guest code.
        let mut prover = Prover::new_with_opts(
            &std::fs::read(EVM_PATH)?,
            EVM_ID,
            ProverOpts::default().with_skip_seal(true),
        )?;

        prover.add_input_u8_slice(&ethabi::encode(
            &[Token::Uint(U256::from(128)), Token::String(String::from("0x671a3b40ecb7d51b209e68392df2d38c098aae03febd3a88be0f1fa77725bbd7")),
                Token::Address(ethabi::ethereum_types::Address::from_str("0x4B45C30b8c4fAEC1c8eAaD5398F8b8e91BFbac15").unwrap()),
                Token::Address(ethabi::ethereum_types::Address::from_str("0x4B45C30b8c4fAEC1c8eAaD5398F8b8e91BFbac15").unwrap()),
                Token::Uint(U256::from(10))],));

        info!("Running zkvm...");
        let receipt = prover.run().expect("Failed to run guest");

        // SKIPPED SEAL! so no verification
        //info!("Verifying receipt...");
        //receipt.verify(&EVM_ID).expect("failed to verify receipt");

        let res: EvmResult = from_slice(&receipt.journal).expect("Failed to deserialize EvmResult");
        info!("exit reason: {:?}", res.exit_reason);
        info!("state updates: {}", res.state.len());


        assert_eq!(
            &receipt.journal,
            &ethabi::encode(&[Token::Bool(true), Token::Address(ethabi::ethereum_types::Address::from_str("0x4B45C30b8c4fAEC1c8eAaD5398F8b8e91BFbac15").unwrap())])
        );
        Ok(())
    }
}
