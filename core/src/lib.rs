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

pub use hashbrown::HashMap;
pub use primitive_types::{H160 as Address, H256, U256};
use revm::db::Database;
use revm::{Account, AccountInfo, Bytecode};
// use log::info;

// Re-export revm members for external usage.
pub use revm::{Env, ExecutionResult, Return, TransactTo, EVM};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize)]
struct ResTrack<T> {
    idx: usize,
    elms: Vec<T>,
}

impl<T> ResTrack<T>
where
    T: Clone,
{
    pub fn reset(&mut self) {
        self.idx = 0;
    }

    pub fn get(&mut self) -> T {
        let res = self.elms[self.idx].clone();
        self.idx += 1;
        res
    }

    pub fn set(&mut self, elm: &T) {
        self.elms.push(elm.clone());
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ZkDb {
    basic: ResTrack<Option<AccountInfo>>,
    code_hash: ResTrack<Bytecode>,
    storage: ResTrack<U256>,
    block: ResTrack<H256>,
}

impl Database for ZkDb {
    type Error = ();
    /// Get basic account information.
    fn basic(&mut self, _address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        Ok(self.basic.get())
    }
    /// Get account code by its hash
    fn code_by_hash(&mut self, _code_hash: H256) -> Result<Bytecode, Self::Error> {
        Ok(self.code_hash.get())
    }
    /// Get storage value of address at index.
    fn storage(&mut self, _address: Address, _index: U256) -> Result<U256, Self::Error> {
        Ok(self.storage.get())
    }
    // History related
    fn block_hash(&mut self, _number: U256) -> Result<H256, Self::Error> {
        Ok(self.block.get())
    }
}

#[cfg(feature = "ethers")]
pub mod ether_trace {
    use std::sync::Arc;

    use bytes::Bytes;
    use ethers_core::types::{BlockId, Transaction, H160 as eH160, U64 as eU64};
    use ethers_providers::Middleware;
    pub use ethers_providers::{Http, Provider};
    use revm::TxEnv;
    use tokio::runtime::Handle;

    use super::*;

    pub fn txenv_from_tx(tx: Transaction) -> TxEnv {
        TxEnv {
            caller: tx.from,
            gas_limit: tx.gas.as_u64(),
            gas_price: tx.gas_price.unwrap(),
            gas_priority_fee: tx.max_priority_fee_per_gas,
            transact_to: if let Some(to_addr) = tx.to {
                TransactTo::Call(to_addr)
            } else {
                TransactTo::Create(revm::CreateScheme::Create) // TODO: create2
            },
            value: tx.value,
            data: Bytes::from(tx.input.to_vec()), // TODO: gotta be a cleaner way
            chain_id: tx.chain_id.map(|elm| elm.as_u64()),
            nonce: Some(tx.nonce.as_u64()),
            access_list: Vec::new(), // TODO
        }
    }

    pub struct TraceTx<M>
    where
        M: Middleware,
    {
        client: Arc<M>,
        handle: Handle,
        block_number: Option<BlockId>,
        db: ZkDb,
    }

    impl<M> TraceTx<M>
    where
        M: Middleware,
    {
        pub fn new(client: Arc<M>, block_number: Option<u64>) -> Option<Self> {
            let client = client;

            let mut out = Self {
                client,
                handle: Handle::current(),
                block_number: None,
                db: Default::default(),
            };
            let bnum = if let Some(block_number) = block_number {
                block_number.into()
            } else {
                out.block_on(out.client.get_block_number()).ok()?
            };

            out.block_number = Some(BlockId::from(bnum));
            Some(out)
        }

        fn block_on<F: core::future::Future>(&self, f: F) -> F::Output {
            self.handle.block_on(f)
        }

        pub fn create_zkdb(mut self) -> ZkDb {
            self.db.basic.reset();
            self.db.code_hash.reset();
            self.db.storage.reset();
            self.db.block.reset();
            self.db
        }
    }

    impl<M> Database for TraceTx<M>
    where
        M: Middleware,
    {
        type Error = ();
        fn basic(&mut self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
            let add = eH160::from(address.0);

            let f = async {
                let nonce = self.client.get_transaction_count(add, self.block_number);
                let balance = self.client.get_balance(add, self.block_number);
                let code = self.client.get_code(add, self.block_number);
                tokio::join!(nonce, balance, code)
            };
            let (nonce, balance, code) = self.block_on(f);
            let balance = balance.unwrap_or_else(|e| panic!("ethers get balance error: {e:?}"));
            let nonce = nonce
                .unwrap_or_else(|e| panic!("ethers get nonce error: {e:?}"))
                .as_u64();

            let bytecode = Bytecode::new_raw(
                code.unwrap_or_else(|e| panic!("ethers get code error: {e:?}"))
                    .0,
            );
            let res = Some(AccountInfo::new(balance, nonce, bytecode));
            self.db.basic.set(&res);
            Ok(res)
        }

        fn code_by_hash(&mut self, _code_hash: H256) -> Result<Bytecode, Self::Error> {
            // not needed because we already load code with basic info
            panic!("Should not be called. Code is already loaded");
        }

        fn storage(&mut self, address: Address, index: U256) -> Result<U256, Self::Error> {
            let add = eH160::from(address.0);
            let mut bytes = [0; 32];
            index.to_big_endian(&mut bytes);
            let index = H256::from(bytes);
            let f = async {
                let storage = self
                    .client
                    .get_storage_at(add, index, self.block_number)
                    .await
                    .unwrap();
                U256::from(storage.0)
            };
            let res = self.block_on(f);
            self.db.storage.set(&res);
            Ok(res)
        }

        fn block_hash(&mut self, number: U256) -> Result<H256, Self::Error> {
            if number > U256::from(u64::MAX) {
                return Ok(revm::KECCAK_EMPTY);
            }
            let number = eU64::from(u64::try_from(number).unwrap());
            let f = async {
                self.client
                    .get_block(BlockId::from(number))
                    .await
                    .ok()
                    .flatten()
            };
            let res = H256(self.block_on(f).unwrap().hash.unwrap().0);
            self.db.block.set(&res);
            Ok(res)
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EvmResult {
    pub exit_reason: Return,
    pub state: HashMap<Address, Account>,
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::str::FromStr;
    use std::sync::Arc;

    use ether_trace::{Http, Provider};
    use ethers_providers::Middleware;

    use super::*;

    // Ignored because it requires a live RPC_URL to run
    #[ignore]
    #[tokio::test]
    async fn trace_tx() {
        let rpc_url = env::var("RPC_URL").unwrap();

        let tx_hash =
            H256::from_str("0x671a3b40ecb7d51b209e68392df2d38c098aae03febd3a88be0f1fa77725bbd7")
                .unwrap();

        let client = Provider::<Http>::try_from(rpc_url).unwrap();
        let client = Arc::new(client);

        let tx = client.get_transaction(tx_hash).await.unwrap().unwrap();
        let block_numb = tx.block_number.unwrap();
        assert_eq!(block_numb, ethers_core::types::U64::from(16424130));

        let mut env = Env::default();
        env.block.number = U256::from(block_numb.as_u64());
        env.tx = ether_trace::txenv_from_tx(tx);

        let trace_db = ether_trace::TraceTx::new(client, Some(block_numb.as_u64())).unwrap();

        // Run the TX with tracing:
        let mut evm = EVM::new();
        evm.database(trace_db);
        evm.env = env.clone();

        // Trick to allow block_on() blocking in async -> sync -> async
        let ((res, _state), trace_db) =
            tokio::task::spawn_blocking(move || (evm.transact(), evm.take_db()))
                .await
                .unwrap();

        assert_eq!(res.exit_reason, Return::Return);
        assert_eq!(res.gas_used, 29316);

        let zkdb = trace_db.create_zkdb();
        assert_eq!(zkdb.basic.elms.len(), 3);
        assert_eq!(zkdb.code_hash.elms.len(), 0);
        assert_eq!(zkdb.storage.elms.len(), 2);
        assert_eq!(zkdb.block.elms.len(), 0);

        let mut evm = EVM::new();
        evm.database(zkdb);
        evm.env = env;

        let (res, _state) = evm.transact();
        assert_eq!(res.exit_reason, Return::Return);
    }
}
