use std::{collections::HashSet, sync::Arc};

use zksync_dal::node::{MasterPool, PoolResource};
use zksync_node_framework::{
    wiring_layer::{WiringError, WiringLayer},
    FromContext, IntoContext,
};
use zksync_types::Address;

use crate::tx_sender::{
    deny_list_pool_sink::DenyListPoolSink, master_pool_sink::MasterPoolSink, tx_sink::TxSink,
};

/// Wiring layer for [`DenyListPoolSink`], [`TxSink`](zksync_node_api_server::tx_sender::tx_sink::TxSink) implementation.
pub struct DenyListPoolSinkLayer {
    deny_list: HashSet<Address>,
}

impl DenyListPoolSinkLayer {
    pub fn new(deny_list: HashSet<Address>) -> Self {
        Self { deny_list }
    }
}

#[derive(Debug, FromContext)]
pub struct Input {
    pool: PoolResource<MasterPool>,
}

#[derive(Debug, IntoContext)]
pub struct Output {
    tx_sink: Arc<dyn TxSink>,
}

#[async_trait::async_trait]
impl WiringLayer for DenyListPoolSinkLayer {
    type Input = Input;
    type Output = Output;

    fn layer_name(&self) -> &'static str {
        "deny_pool_sink_layer"
    }

    async fn wire(self, input: Self::Input) -> Result<Self::Output, WiringError> {
        let pool = input.pool.get().await?;
        Ok(Output {
            tx_sink: Arc::new(DenyListPoolSink::new(
                MasterPoolSink::new(pool),
                self.deny_list,
            )),
        })
    }
}
