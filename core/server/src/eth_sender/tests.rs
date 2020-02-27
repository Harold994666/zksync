use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};

use web3::contract::{tokens::Tokenize, Options};
use web3::types::{H256, U256};

use eth_client::SignedCallResult;

use super::database::DatabaseAccess;
use super::ethereum_interface::EthereumInterface;
use super::transactions::{ExecutedTxStatus, OperationETHState, TransactionETHState};

#[derive(Debug, Default)]
struct MockDatabase {
    restore_state: VecDeque<OperationETHState>,
    unconfirmed_operations: RefCell<HashMap<H256, TransactionETHState>>,
    confirmed_operations: RefCell<HashMap<H256, TransactionETHState>>,
}

impl MockDatabase {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_restorable_state(
        restore_state: impl IntoIterator<Item = OperationETHState>,
    ) -> Self {
        Self {
            restore_state: restore_state.into_iter().collect(),
            ..Default::default()
        }
    }
}

impl DatabaseAccess for MockDatabase {
    fn restore_state(&self) -> Result<VecDeque<OperationETHState>, failure::Error> {
        Ok(self.restore_state.clone())
    }

    fn save_unconfirmed_operation(&self, tx: &TransactionETHState) -> Result<(), failure::Error> {
        self.unconfirmed_operations
            .borrow_mut()
            .insert(tx.signed_tx.hash, tx.clone());

        Ok(())
    }

    fn confirm_operation(&self, hash: &H256) -> Result<(), failure::Error> {
        let mut unconfirmed_operations = self.unconfirmed_operations.borrow_mut();
        assert!(
            unconfirmed_operations.contains_key(hash),
            "Request to confirm operation that was not stored"
        );

        let operation = unconfirmed_operations.remove(hash).unwrap();
        self.confirmed_operations
            .borrow_mut()
            .insert(*hash, operation);

        Ok(())
    }
}

#[derive(Default)]
struct MockEthereum {
    pub block_number: u64,
    pub nonce: U256,
    pub gas_price: U256,
    pub tx_statuses: RefCell<HashMap<H256, ExecutedTxStatus>>,
    pub sent_txs: RefCell<HashMap<H256, SignedCallResult>>,
}

impl EthereumInterface for MockEthereum {
    fn get_tx_status(&self, hash: &H256) -> Result<Option<ExecutedTxStatus>, failure::Error> {
        Ok(self.tx_statuses.borrow().get(hash).cloned())
    }

    fn block_number(&self) -> Result<u64, failure::Error> {
        Ok(self.block_number)
    }

    fn gas_price(&self) -> Result<U256, failure::Error> {
        Ok(self.gas_price)
    }

    fn current_nonce(&self) -> Result<U256, failure::Error> {
        Ok(self.nonce)
    }

    fn send_tx(&self, signed_tx: &SignedCallResult) -> Result<(), failure::Error> {
        self.sent_txs
            .borrow_mut()
            .insert(signed_tx.hash, signed_tx.clone());

        Ok(())
    }

    fn sign_call_tx<P: Tokenize>(
        &self,
        _func: &str,
        params: P,
        options: Options,
    ) -> Result<SignedCallResult, failure::Error> {
        let raw_tx = ethabi::encode(params.into_tokens().as_ref());

        Ok(SignedCallResult {
            raw_tx: raw_tx,
            gas_price: options.gas_price.unwrap(),
            nonce: options.nonce.unwrap(),
            hash: H256::random(), // Okay for test purposes.
        })
    }
}
