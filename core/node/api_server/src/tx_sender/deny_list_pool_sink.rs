use std::collections::HashSet;

use zksync_dal::transactions_dal::L2TxSubmissionResult;
use zksync_multivm::interface::{tracer::ValidationTraces, TransactionExecutionMetrics};
use zksync_types::{l2::L2Tx, Address};

use super::{master_pool_sink::MasterPoolSink, tx_sink::TxSink, SubmitTxError};
//use crate::api_server::tx_sender::master_pool_sink::MasterPoolSink;

/// Wrapper for the master DB pool that allows to submit transactions to the mempool.
#[derive(Debug)]
pub struct DenyListPoolSink {
    deny_list: HashSet<Address>,
    master_pool_sync: MasterPoolSink,
}

impl DenyListPoolSink {
    pub fn new(master_pool_sync: MasterPoolSink, deny_list: HashSet<Address>) -> Self {
        Self {
            master_pool_sync,
            deny_list,
        }
    }
}

#[async_trait::async_trait]
impl TxSink for DenyListPoolSink {
    async fn submit_tx(
        &self,
        tx: &L2Tx,
        execution_metrics: TransactionExecutionMetrics,
        validation_traces: ValidationTraces,
    ) -> Result<L2TxSubmissionResult, SubmitTxError> {
        let address_and_nonce = (tx.initiator_account(), tx.nonce());
        if self.deny_list.contains(&address_and_nonce.0) {
            return Err(SubmitTxError::SenderInDenyList(tx.initiator_account()));
        }

        self.master_pool_sync
            .submit_tx(tx, execution_metrics, validation_traces)
            .await
    }
}
