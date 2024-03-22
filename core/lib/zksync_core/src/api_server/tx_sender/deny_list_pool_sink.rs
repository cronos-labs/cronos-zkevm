use std::{collections::HashSet, str::FromStr};

use zksync_dal::{transactions_dal::L2TxSubmissionResult, ConnectionPool};
use zksync_types::{fee::TransactionExecutionMetrics, l2::L2Tx, Address};

use super::{tx_sink::TxSink, SubmitTxError};
use crate::metrics::{TxStage, APP_METRICS};

/// Wrapper for the master DB pool that allows to submit transactions to the mempool.
#[derive(Debug)]
pub struct DenyListPoolSink {
    deny_list_pool: ConnectionPool,
    deny_list: HashSet<Address>,
}

impl DenyListPoolSink {
    pub fn new(deny_list_pool: ConnectionPool, list: &str) -> Self {
        let deny_list: HashSet<Address> = list
            .split(',')
            .map(|element| Address::from_str(element).unwrap())
            .collect();

        Self {
            deny_list_pool,
            deny_list,
        }
    }
}

#[async_trait::async_trait]
impl TxSink for DenyListPoolSink {
    async fn submit_tx(
        &self,
        tx: L2Tx,
        execution_metrics: TransactionExecutionMetrics,
    ) -> Result<L2TxSubmissionResult, SubmitTxError> {
        let _sender = tx.initiator_account();
        if self.deny_list.contains(&_sender) {
            return Err(SubmitTxError::SenderInDenyList);
        }

        let submission_res_handle = self
            .deny_list_pool
            .access_storage_tagged("api")
            .await?
            .transactions_dal()
            .insert_transaction_l2(tx, execution_metrics)
            .await;

        APP_METRICS.processed_txs[&TxStage::Mempool(submission_res_handle)].inc();
        Ok(submission_res_handle)
    }
}
