use zksync_config::{configs::wallets, GasAdjusterConfig};
use zksync_node_framework::{
    wiring_layer::{WiringError, WiringLayer},
    FromContext, IntoContext,
};
use zksync_shared_resources::contracts::{
    L1ChainContractsResource, SettlementLayerContractsResource,
};
use zksync_web3_decl::{
    client::{DynClient, L1},
    node::SettlementLayerClient,
};

use super::resources::{BoundEthInterfaceForBlobsResource, BoundEthInterfaceForL2Resource};
use crate::{
    clients::{GKMSSigningClient, PKSigningClient},
    BoundEthInterface, EthInterface,
};

/// Wiring layer for [`PKSigningClient`].
#[derive(Debug)]
pub struct PKSigningEthClientLayer {
    gas_adjuster_config: GasAdjusterConfig,
    operator: wallets::Wallet,
    blob_operator: Option<wallets::Wallet>,
}

#[derive(Debug, FromContext)]
pub struct Input {
    eth_client: Box<DynClient<L1>>,
    gateway_client: SettlementLayerClient,
    contracts: SettlementLayerContractsResource,
    l1_contracts: L1ChainContractsResource,
}

#[derive(Debug, IntoContext)]
pub struct Output {
    signing_client: Box<dyn BoundEthInterface>,
    /// Only provided if the blob operator key is provided to the layer.
    signing_client_for_blobs: Option<BoundEthInterfaceForBlobsResource>,
    signing_client_for_gateway: Option<BoundEthInterfaceForL2Resource>,
}

impl PKSigningEthClientLayer {
    pub fn new(
        gas_adjuster_config: GasAdjusterConfig,
        operator: wallets::Wallet,
        blob_operator: Option<wallets::Wallet>,
    ) -> Self {
        Self {
            gas_adjuster_config,
            operator,
            blob_operator,
        }
    }
}

#[async_trait::async_trait]
impl WiringLayer for PKSigningEthClientLayer {
    type Input = Input;
    type Output = Output;

    fn layer_name(&self) -> &'static str {
        "pk_signing_eth_client_layer"
    }

    async fn wire(self, input: Self::Input) -> Result<Self::Output, WiringError> {
        let gas_adjuster_config = &self.gas_adjuster_config;
        let query_client = input.eth_client;

        let l1_diamond_proxy_addr = input
            .l1_contracts
            .0
            .chain_contracts_config
            .diamond_proxy_addr;
        let l1_chain_id = query_client
            .fetch_chain_id()
            .await
            .map_err(WiringError::internal)?;

        let signing_client: Box<dyn BoundEthInterface>;
        if let Some(gkms_op_key_name) = self.operator.gkms_key_name() {
            let gkms_sc = GKMSSigningClient::new_raw(
                l1_diamond_proxy_addr,
                gas_adjuster_config.default_priority_fee_per_gas,
                l1_chain_id,
                query_client.clone(),
                gkms_op_key_name,
            )
            .await;
            signing_client = Box::new(gkms_sc);
        } else {
            let private_key = self.operator.private_key();
            let sc = PKSigningClient::new_raw(
                private_key.clone(),
                l1_diamond_proxy_addr,
                gas_adjuster_config.default_priority_fee_per_gas,
                l1_chain_id,
                query_client.clone(),
            );
            signing_client = Box::new(sc);
        }

        let signing_client_for_blobs = if let Some(blob_operator) = self.blob_operator {
            if let Some(gkms_op_key_name) = blob_operator.gkms_key_name() {
                let signing_client_for_blobs = GKMSSigningClient::new_raw(
                    l1_diamond_proxy_addr,
                    gas_adjuster_config.default_priority_fee_per_gas,
                    l1_chain_id,
                    query_client,
                    gkms_op_key_name,
                )
                .await;
                Some(BoundEthInterfaceForBlobsResource(Box::new(
                    signing_client_for_blobs,
                )))
            } else {
                let private_key = blob_operator.private_key();
                let signing_client_for_blobs = PKSigningClient::new_raw(
                    private_key.clone(),
                    l1_diamond_proxy_addr,
                    gas_adjuster_config.default_priority_fee_per_gas,
                    l1_chain_id,
                    query_client,
                );
                Some(BoundEthInterfaceForBlobsResource(Box::new(
                    signing_client_for_blobs,
                )))
            }
        } else {
            None
        };

        let signing_client_for_gateway = match input.gateway_client {
            SettlementLayerClient::Gateway(gateway_client) => {
                let l2_chain_id = gateway_client
                    .fetch_chain_id()
                    .await
                    .map_err(WiringError::internal)?;

                let signing_client_for_gateway: Box<dyn BoundEthInterface>;

                if let Some(gkms_op_key_name) = self.operator.gkms_key_name() {
                    let gkms_sc = GKMSSigningClient::new_raw(
                        l1_diamond_proxy_addr,
                        gas_adjuster_config.default_priority_fee_per_gas,
                        l2_chain_id,
                        gateway_client,
                        gkms_op_key_name,
                    )
                    .await;

                    signing_client_for_gateway = Box::new(gkms_sc);
                } else {
                    let private_key = self.operator.private_key();
                    let sc = PKSigningClient::new_raw(
                        private_key.clone(),
                        l1_diamond_proxy_addr,
                        gas_adjuster_config.default_priority_fee_per_gas,
                        l2_chain_id,
                        gateway_client,
                    );

                    signing_client_for_gateway = Box::new(sc);
                }
                Some(BoundEthInterfaceForL2Resource(signing_client_for_gateway))
            }
            SettlementLayerClient::L1(_) => None,
        };

        Ok(Output {
            signing_client,
            signing_client_for_blobs,
            signing_client_for_gateway,
        })
    }
}
