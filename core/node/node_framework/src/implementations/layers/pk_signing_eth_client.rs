use anyhow::Context as _;
use zksync_config::{
    configs::{gateway::GatewayChainConfig, wallets, ContractsConfig},
    EthConfig,
};
use zksync_eth_client::{
    clients::{GKMSSigningClient, PKSigningClient},
    EthInterface,
};

use crate::{
    implementations::resources::eth_interface::{
        BoundEthInterfaceForBlobsResource, BoundEthInterfaceForL2Resource,
        BoundEthInterfaceResource, EthInterfaceResource, GatewayEthInterfaceResource,
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
    eth_sender_config: EthConfig,
    contracts_config: ContractsConfig,
    gateway_chain_config: Option<GatewayChainConfig>,
    wallets: wallets::EthSender,
    client_type: SigningEthClientType,
}

#[derive(Debug, FromContext)]
#[context(crate = crate)]
pub struct Input {
    pub eth_client: EthInterfaceResource,
    pub gateway_client: Option<GatewayEthInterfaceResource>,
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
        eth_sender_config: EthConfig,
        contracts_config: ContractsConfig,
        gateway_chain_config: Option<GatewayChainConfig>,
        wallets: wallets::EthSender,
        client_type: SigningEthClientType,
    ) -> Self {
        Self {
            eth_sender_config,
            contracts_config,
            gateway_chain_config,
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
        let gas_adjuster_config = self
            .eth_sender_config
            .gas_adjuster
            .as_ref()
            .context("gas_adjuster config is missing")?;
        let EthInterfaceResource(query_client) = input.eth_client;

        let l1_chain_id = query_client
            .fetch_chain_id()
            .await
            .map_err(WiringError::internal)?;

        let signing_client;
        let signing_client_for_gateway;
        let mut signing_client_for_blobs = None;

        match self.client_type {
            SigningEthClientType::PKSigningEthClient => {
                let private_key = self.wallets.operator.private_key();

                let sc = PKSigningClient::new_raw(
                    private_key.clone(),
                    self.contracts_config.diamond_proxy_addr,
                    gas_adjuster_config.default_priority_fee_per_gas,
                    l1_chain_id,
                    query_client.clone(),
                );
                signing_client = BoundEthInterfaceResource(Box::new(sc));

                signing_client_for_blobs = self.wallets.blob_operator.map(|blob_operator| {
                    let private_key = blob_operator.private_key();
                    let signing_client_for_blobs = PKSigningClient::new_raw(
                        private_key.clone(),
                        self.contracts_config.diamond_proxy_addr,
                        gas_adjuster_config.default_priority_fee_per_gas,
                        l1_chain_id,
                        query_client,
                    );
                    BoundEthInterfaceForBlobsResource(Box::new(signing_client_for_blobs))
                });

                signing_client_for_gateway = if let (Some(client), Some(gateway_contracts)) =
                    (&input.gateway_client, self.gateway_chain_config.as_ref())
                {
                    if gateway_contracts.gateway_chain_id.0 != 0u64 {
                        let GatewayEthInterfaceResource(gateway_client) = client;
                        let signing_client_for_gateway = PKSigningClient::new_raw(
                            private_key.clone(),
                            gateway_contracts.diamond_proxy_addr,
                            gas_adjuster_config.default_priority_fee_per_gas,
                            gateway_contracts.gateway_chain_id,
                            gateway_client.clone(),
                        );
                        Some(BoundEthInterfaceForL2Resource(Box::new(
                            signing_client_for_gateway,
                        )))
                    } else {
                        None
                    }
                } else {
                    None
                };
            }
            SigningEthClientType::GKMSSigningEthClient => {
                let gkms_op_key_name = std::env::var("GOOGLE_KMS_OP_KEY_NAME").ok();
                tracing::info!(
                    "KMS op key name: {:?}",
                    std::env::var("GOOGLE_KMS_OP_KEY_NAME")
                );

                let sc = GKMSSigningClient::new_raw(
                    self.contracts_config.diamond_proxy_addr,
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
                        self.contracts_config.diamond_proxy_addr,
                        gas_adjuster_config.default_priority_fee_per_gas,
                        l1_chain_id,
                        query_client,
                        key_name.to_string(),
                    )
                    .await;
                    signing_client_for_blobs =
                        Some(BoundEthInterfaceForBlobsResource(Box::new(blobs_resources)));
                };

                signing_client_for_gateway = if let (Some(client), Some(gateway_contracts)) =
                    (&input.gateway_client, self.gateway_chain_config.as_ref())
                {
                    if gateway_contracts.gateway_chain_id.0 != 0u64 {
                        let GatewayEthInterfaceResource(gateway_client) = client;
                        let signing_client_for_gateway = GKMSSigningClient::new_raw(
                            gateway_contracts.diamond_proxy_addr,
                            gas_adjuster_config.default_priority_fee_per_gas,
                            gateway_contracts.gateway_chain_id,
                            gateway_client.clone(),
                            gkms_op_key_name
                                .expect("gkms_op_key_name is required but was None")
                                .to_string(),
                        )
                        .await;

                        Some(BoundEthInterfaceForL2Resource(Box::new(
                            signing_client_for_gateway,
                        )))
                    } else {
                        None
                    }
                } else {
                    None
                };
            }
        };

        Ok(Output {
            signing_client,
            signing_client_for_blobs,
            signing_client_for_gateway,
        })
    }
}
