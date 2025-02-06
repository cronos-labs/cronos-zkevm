use std::collections::HashSet;

use zksync_node_api_server::tx_sender::{
    deny_list_pool_sink::DenyListPoolSink, master_pool_sink::MasterPoolSink,
};
use zksync_types::Address;

use crate::{
    implementations::resources::{
        pools::{MasterPool, PoolResource},
        web3_api::TxSinkResource,
    },
    wiring_layer::{WiringError, WiringLayer},
    FromContext, IntoContext,
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
#[context(crate = crate)]
pub struct Input {
    pub pool: PoolResource<MasterPool>,
}

#[derive(Debug, IntoContext)]
#[context(crate = crate)]
pub struct Output {
    pub tx_sink: TxSinkResource,
}

#[async_trait::async_trait]
impl WiringLayer for DenyListPoolSinkLayer {
    type Input = Input;
    type Output = Output;

    fn layer_name(&self) -> &'static str {
        "deny_list_pool_sink_layer"
    }

    async fn wire(self, input: Self::Input) -> Result<Self::Output, WiringError> {
        let pool = input.pool.get().await?;
        Ok(Output {
            tx_sink: DenyListPoolSink::new(MasterPoolSink::new(pool), self.deny_list).into(),
        })
    }
}
