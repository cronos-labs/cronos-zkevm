use zksync_config::{configs::wallets, GasAdjusterConfig};
use zksync_eth_client::{
    clients::{GKMSSigningClient, PKSigningClient},
    EthInterface,
};

use crate::{
    implementations::resources::{
        contracts::{L1ChainContractsResource, SettlementLayerContractsResource},
        eth_interface::{
            BoundEthInterfaceForBlobsResource, BoundEthInterfaceForL2Resource,
            BoundEthInterfaceResource, EthInterfaceResource, SettlementLayerClient,
            SettlementLayerClientResource,
        },
    },
    wiring_layer::{WiringError, WiringLayer},
    FromContext, IntoContext,
};

/// Wiring layer for [`PKSigningClient`].
#[derive(Debug)]
#[non_exhaustive]
pub enum SigningEthClientType {
    PKSigningEthClient,
    GKMSSigningEthClient,
}

#[derive(Debug)]
pub struct PKSigningEthClientLayer {
    gas_adjuster_config: GasAdjusterConfig,
    wallets: wallets::EthSender,
    client_type: SigningEthClientType,
}

#[derive(Debug, FromContext)]
#[context(crate = crate)]
pub struct Input {
    pub eth_client: EthInterfaceResource,
    pub contracts: SettlementLayerContractsResource,
    pub l1_contracts: L1ChainContractsResource,
    pub gateway_client: SettlementLayerClientResource,
}

#[derive(Debug, IntoContext)]
#[context(crate = crate)]
pub struct Output {
    pub signing_client: BoundEthInterfaceResource,
    /// Only provided if the blob operator key is provided to the layer.
    pub signing_client_for_blobs: Option<BoundEthInterfaceForBlobsResource>,
    pub signing_client_for_gateway: Option<BoundEthInterfaceForL2Resource>,
}

impl PKSigningEthClientLayer {
    pub fn new(
        gas_adjuster_config: GasAdjusterConfig,
        wallets: wallets::EthSender,
        client_type: SigningEthClientType,
    ) -> Self {
        Self {
            gas_adjuster_config,
            wallets,
            client_type,
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
        let EthInterfaceResource(query_client) = input.eth_client;

        let l1_chain_id = query_client
            .fetch_chain_id()
            .await
            .map_err(WiringError::internal)?;

        let signing_client;
        let signing_client_for_l2_gateway;
        let mut signing_client_for_blobs = None;

        match self.client_type {
            SigningEthClientType::PKSigningEthClient => {
                let private_key = self.wallets.operator.private_key();
                let pk_signing_client = PKSigningClient::new_raw(
                    private_key.clone(),
                    input
                        .l1_contracts
                        .0
                        .chain_contracts_config
                        .diamond_proxy_addr,
                    gas_adjuster_config.default_priority_fee_per_gas,
                    l1_chain_id,
                    query_client.clone(),
                );
                signing_client = BoundEthInterfaceResource(Box::new(pk_signing_client));

                signing_client_for_blobs = self.wallets.blob_operator.map(|blob_operator| {
                    let private_key = blob_operator.private_key();
                    let signing_client_for_blobs = PKSigningClient::new_raw(
                        private_key.clone(),
                        input
                            .l1_contracts
                            .0
                            .chain_contracts_config
                            .diamond_proxy_addr,
                        gas_adjuster_config.default_priority_fee_per_gas,
                        l1_chain_id,
                        query_client,
                    );
                    BoundEthInterfaceForBlobsResource(Box::new(signing_client_for_blobs))
                });

                signing_client_for_l2_gateway = match input.gateway_client.0 {
                    SettlementLayerClient::L2(gateway_client) => {
                        let private_key = self.wallets.operator.private_key();
                        let chain_id = gateway_client
                            .fetch_chain_id()
                            .await
                            .map_err(WiringError::internal)?;
                        let signing_client_for_blobs = PKSigningClient::new_raw(
                            private_key.clone(),
                            input.contracts.0.chain_contracts_config.diamond_proxy_addr,
                            gas_adjuster_config.default_priority_fee_per_gas,
                            chain_id,
                            gateway_client.clone(),
                        );
                        Some(BoundEthInterfaceForL2Resource(Box::new(
                            signing_client_for_blobs,
                        )))
                    }
                    SettlementLayerClient::L1(_) => None,
                };
            }
            SigningEthClientType::GKMSSigningEthClient => {
                let gkms_op_key_name = std::env::var("GOOGLE_KMS_OP_KEY_NAME").ok();
                tracing::info!(
                    "KMS op key name: {:?}",
                    std::env::var("GOOGLE_KMS_OP_KEY_NAME")
                );

                let sc = GKMSSigningClient::new_raw(
                    input
                        .l1_contracts
                        .0
                        .chain_contracts_config
                        .diamond_proxy_addr,
                    gas_adjuster_config.default_priority_fee_per_gas,
                    l1_chain_id,
                    query_client.clone(),
                    gkms_op_key_name
                        .clone()
                        .expect("gkms_op_key_name is required but was None")
                        .to_string(),
                )
                .await;

                signing_client = BoundEthInterfaceResource(Box::new(sc));

                let gkms_op_blob_key_name = std::env::var("GOOGLE_KMS_OP_BLOB_KEY_NAME").ok();
                tracing::info!(
                    "KMS op blob key name: {:?}",
                    std::env::var("GOOGLE_KMS_OP_BLOB_KEY_NAME")
                );

                if let Some(key_name) = gkms_op_blob_key_name {
                    let blobs_resources = GKMSSigningClient::new_raw(
                        input
                            .l1_contracts
                            .0
                            .chain_contracts_config
                            .diamond_proxy_addr,
                        gas_adjuster_config.default_priority_fee_per_gas,
                        l1_chain_id,
                        query_client,
                        key_name.to_string(),
                    )
                    .await;
                    signing_client_for_blobs =
                        Some(BoundEthInterfaceForBlobsResource(Box::new(blobs_resources)));
                };

                signing_client_for_l2_gateway = match input.gateway_client.0 {
                    SettlementLayerClient::L2(gateway_client) => {
                        let chain_id = gateway_client
                            .fetch_chain_id()
                            .await
                            .map_err(WiringError::internal)?;
                        let signing_client_for_blobs = GKMSSigningClient::new_raw(
                            input.contracts.0.chain_contracts_config.diamond_proxy_addr,
                            gas_adjuster_config.default_priority_fee_per_gas,
                            chain_id,
                            gateway_client.clone(),
                            gkms_op_key_name
                                .expect("gkms_op_key_name is required but was None")
                                .to_string(),
                        )
                        .await;
                        Some(BoundEthInterfaceForL2Resource(Box::new(
                            signing_client_for_blobs,
                        )))
                    }
                    SettlementLayerClient::L1(_) => None,
                };
            }
        }
        Ok(Output {
            signing_client,
            signing_client_for_blobs,
            signing_client_for_gateway: signing_client_for_l2_gateway,
        })
    }
}
