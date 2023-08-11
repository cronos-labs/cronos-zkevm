use itertools::Itertools;
use std::convert::TryFrom;
use std::fmt::{Debug, Display};

use tokio::time::Instant;

use zksync_eth_client::{types::Error as EthClientError, EthInterface};
use zksync_types::ethabi::{Contract, Hash};

use zksync_contracts::zksync_contract;
use zksync_types::{
    l1::L1Tx,
    web3::{
        self,
        types::{BlockNumber, FilterBuilder, Log},
    },
    H160,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Log parsing filed: {0}")]
    LogParse(String),
    #[error("Eth client error: {0}")]
    EthClient(#[from] EthClientError),
    #[error("Infinite recursion caused by too many responses")]
    InfiniteRecursion,
}

#[derive(Debug)]
struct ContractTopics {
    new_priority_request: Hash,
}

impl ContractTopics {
    fn new(zksync_contract: &Contract) -> Self {
        Self {
            new_priority_request: zksync_contract
                .event("NewPriorityRequest")
                .expect("main contract abi error")
                .signature(),
        }
    }
}

#[async_trait::async_trait]
pub trait EthClient {
    async fn get_priority_op_events(
        &self,
        from: BlockNumber,
        to: BlockNumber,
        retries_left: usize,
    ) -> Result<Vec<L1Tx>, Error>;
    async fn finalized_block_number(&self) -> Result<u64, Error>;
}

pub const RETRY_LIMIT: usize = 5;
const TOO_MANY_RESULTS_INFURA: &str = "query returned more than";
const TOO_MANY_RESULTS_ALCHEMY: &str = "response size exceeded";
const TOO_MANY_RESULTS_CRONOS: &str = "maximum [from, to]";

#[derive(Debug)]
pub struct EthHttpQueryClient<E> {
    client: E,
    topics: ContractTopics,
    zksync_contract_addr: H160,
    confirmations_for_eth_event: Option<u64>,
}

impl<E: EthInterface> EthHttpQueryClient<E> {
    pub fn new(
        client: E,
        zksync_contract_addr: H160,
        confirmations_for_eth_event: Option<u64>,
    ) -> Self {
        vlog::debug!("New eth client, contract addr: {:x}", zksync_contract_addr);
        let topics = ContractTopics::new(&zksync_contract());
        Self {
            client,
            topics,
            zksync_contract_addr,
            confirmations_for_eth_event,
        }
    }

    async fn get_filter_logs<T>(
        &self,
        from: BlockNumber,
        to: BlockNumber,
        topics: Vec<Hash>,
    ) -> Result<Vec<T>, Error>
    where
        T: TryFrom<Log>,
        T::Error: Debug + Display,
    {
        let filter = FilterBuilder::default()
            .address(vec![self.zksync_contract_addr])
            .from_block(from)
            .to_block(to)
            .topics(Some(topics), None, None, None)
            .build();

        self.client
            .logs(filter, "watch")
            .await?
            .into_iter()
            .map(|log| T::try_from(log).map_err(|err| Error::LogParse(format!("{}", err))))
            .collect()
    }
}

#[async_trait::async_trait]
impl<E: EthInterface + Send + Sync + 'static> EthClient for EthHttpQueryClient<E> {
    async fn get_priority_op_events(
        &self,
        from: BlockNumber,
        to: BlockNumber,
        retries_left: usize,
    ) -> Result<Vec<L1Tx>, Error> {
        let start = Instant::now();

        let mut result = self
            .get_filter_logs(from, to, vec![self.topics.new_priority_request])
            .await;

        // This code is compatible with both Infura and Alchemy API providers.
        // Note: we don't handle rate-limits here - assumption is that we're never going to hit them.
        if let Err(Error::EthClient(EthClientError::EthereumGateway(err))) = &result {
            vlog::warn!("Provider returned error message: {:?}", err);
            let err_message = err.to_string();
            let err_code = if let web3::Error::Rpc(err) = err {
                Some(err.code.code())
            } else {
                None
            };

            let should_retry = |err_code, err_message: String| {
                // All of these can be emitted by either API provider.
                err_code == Some(-32603)             // Internal error
                || err_message.contains("failed")    // Server error
                || err_message.contains("timed out") // Time-out error
            };

            // check whether the error is related to having too many results
            if err_message.contains(TOO_MANY_RESULTS_INFURA)
                || err_message.contains(TOO_MANY_RESULTS_ALCHEMY)
                || err_message.contains(TOO_MANY_RESULTS_CRONOS)
            {
                // get the numeric block ids
                let from_number = match from {
                    BlockNumber::Number(num) => num,
                    _ => {
                        // invalid variant
                        return result;
                    }
                };
                let to_number = match to {
                    BlockNumber::Number(num) => num,
                    BlockNumber::Latest => self.client.block_number("watch").await?,
                    _ => {
                        // invalid variant
                        return result;
                    }
                };

                // divide range into two halves and recursively fetch them
                let mid = (from_number + to_number) / 2;

                // safety check to prevent infinite recursion (quite unlikely)
                if from_number >= mid {
                    return Err(Error::InfiniteRecursion);
                }
                vlog::warn!(
                    "Splitting block range in half: {:?} - {:?} - {:?}",
                    from,
                    mid,
                    to
                );
                let mut first_half = self
                    .get_priority_op_events(from, BlockNumber::Number(mid), RETRY_LIMIT)
                    .await?;
                let mut second_half = self
                    .get_priority_op_events(BlockNumber::Number(mid + 1u64), to, RETRY_LIMIT)
                    .await?;

                first_half.append(&mut second_half);
                result = Ok(first_half);
            } else if should_retry(err_code, err_message) && retries_left > 0 {
                vlog::warn!("Retrying. Retries left: {:?}", retries_left);
                result = self
                    .get_priority_op_events(from, to, retries_left - 1)
                    .await;
            }
        }

        let events: Vec<L1Tx> = result?
            .into_iter()
            .sorted_by_key(|event| event.serial_id())
            .collect();

        metrics::histogram!("eth_watcher.get_priority_op_events", start.elapsed());
        Ok(events)
    }

    async fn finalized_block_number(&self) -> Result<u64, Error> {
        if let Some(confirmations) = self.confirmations_for_eth_event {
            let latest_block_number = self.client.block_number("watch").await?.as_u64();
            Ok(latest_block_number.saturating_sub(confirmations))
        } else {
            self.client
                .block("finalized".to_string(), "watch")
                .await
                .map_err(Into::into)
                .map(|res| {
                    res.expect("Finalized block must be present on L1")
                        .number
                        .expect("Finalized block must contain number")
                        .as_u64()
                })
        }
    }
}
