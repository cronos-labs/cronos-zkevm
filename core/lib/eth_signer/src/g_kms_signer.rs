use std::result::Result;

use ethers_signers::Signer as EthSigner;
use google_cloud_gax::retry::RetrySetting;
use google_cloud_kms::{
    client::{Client, ClientConfig},
    signer::ethereum::Signer,
};
use hex;
use tracing::{self};
use zksync_basic_types::{
    web3::{keccak256, Signature},
    Address, H256, U256,
};
use zksync_crypto_primitives::{EIP712TypedStructure, Eip712Domain, PackedEthSignature};

use crate::{
    raw_ethereum_tx::{Transaction, TransactionParameters},
    EthereumSigner, SignerError,
};

pub const GOOGLE_KMS_OP_KEY_NAME: &str = "GOOGLE_KMS_OP_KEY_NAME";
pub const GOOGLE_KMS_OP_BLOB_KEY_NAME: &str = "GOOGLE_KMS_OP_BLOB_KEY_NAME";

#[derive(Clone)]
pub struct GKMSSigner {
    signer: Signer,
}

impl GKMSSigner {
    pub async fn new(key_name: String, _chain_id: u64) -> Result<Self, SignerError> {
        let config = ClientConfig::default()
            .with_auth()
            .await
            .map_err(|e| SignerError::SigningFailed(e.to_string()))?;

        let client = Client::new(config)
            .await
            .map_err(|e| SignerError::SigningFailed(e.to_string()))?;

        let signer = Signer::new(client, &key_name, _chain_id, Some(RetrySetting::default()))
            .await
            .map_err(|e| SignerError::SigningFailed(e.to_string()))?;

        tracing::info!("KMS signer address: {:?}", hex::encode(signer.address()));

        Ok(GKMSSigner { signer })
    }

    fn u256_to_h256(u: U256) -> H256 {
        let mut bytes = [0u8; 32];
        u.to_big_endian(&mut bytes);
        H256::from(bytes)
    }
}

#[async_trait::async_trait]
impl EthereumSigner for GKMSSigner {
    /// Get Ethereum address that matches the private key.
    async fn get_address(&self) -> Result<Address, SignerError> {
        Ok(self.signer.address())
    }

    /// Signs typed struct using Ethereum private key by EIP-712 signature standard.
    /// Result of this function is the equivalent of RPC calling `eth_signTypedData`.
    async fn sign_typed_data<S: EIP712TypedStructure + Sync>(
        &self,
        domain: &Eip712Domain,
        typed_struct: &S,
    ) -> Result<PackedEthSignature, SignerError> {
        let digest =
            H256::from(PackedEthSignature::typed_data_to_signed_bytes(domain, typed_struct).0);

        let signature = self
            .signer
            .sign_digest(digest.as_bytes())
            .await
            .map_err(|e| SignerError::SigningFailed(e.to_string()))?;

        // Convert the signature components to the appropriate format.
        let r_h256 = GKMSSigner::u256_to_h256(signature.r);
        let s_h256 = GKMSSigner::u256_to_h256(signature.s);

        // Normalize v to recovery id (0 or 1). The ethers Signature struct
        // returns v as 27 or 28; from_rsv expects the raw recovery id.
        let v_byte: u8 = if signature.v >= 27 {
            (signature.v - 27) as u8
        } else {
            signature.v as u8
        };

        let eth_sig = PackedEthSignature::from_rsv(&r_h256, &s_h256, v_byte);

        Ok(eth_sig)
    }

    /// Signs and returns the RLP-encoded transaction.
    async fn sign_transaction(
        &self,
        raw_tx: TransactionParameters,
    ) -> Result<Vec<u8>, SignerError> {
        // According to the code in web3 <https://docs.rs/web3/latest/src/web3/api/accounts.rs.html#86>
        // We should use `max_fee_per_gas` as `gas_price` if we use EIP1559
        let gas_price = raw_tx.max_fee_per_gas;
        let max_priority_fee_per_gas = raw_tx.max_priority_fee_per_gas;

        let tx = Transaction {
            to: raw_tx.to,
            nonce: raw_tx.nonce,
            gas: raw_tx.gas,
            gas_price,
            value: raw_tx.value,
            data: raw_tx.data,
            transaction_type: raw_tx.transaction_type,
            access_list: raw_tx.access_list.unwrap_or_default(),
            max_priority_fee_per_gas,
            max_fee_per_blob_gas: raw_tx.max_fee_per_blob_gas,
            blob_versioned_hashes: raw_tx.blob_versioned_hashes,
        };

        let encoded = tx.encode_pub(raw_tx.chain_id, None);
        let digest = H256(keccak256(encoded.as_ref()));

        let signature = self
            .signer
            .sign_digest(digest.as_bytes())
            .await
            .map_err(|e| SignerError::SigningFailed(e.to_string()))?;

        // Normalize v to recovery id (0 or 1). The ethers Signature struct
        // returns v as 27 or 28; raw recovery id otherwise.
        let recovery_id = if signature.v >= 27 {
            signature.v - 27
        } else {
            signature.v
        };

        let adjusted_v = if let Some(transaction_type) = tx.transaction_type.map(|t| t.as_u64()) {
            match transaction_type {
                0 => recovery_id + raw_tx.chain_id * 2 + 35, // EIP-155
                _ => recovery_id,                            // EIP-2930/1559/4844 use yParity
            }
        } else {
            recovery_id + raw_tx.chain_id * 2 + 35 // Legacy EIP-155
        };

        let r_h256 = GKMSSigner::u256_to_h256(signature.r);
        let s_h256 = GKMSSigner::u256_to_h256(signature.s);

        tracing::debug!(
            "KMS sign_transaction signature: v: {}, r: {}, s: {}",
            adjusted_v,
            hex::encode(r_h256),
            hex::encode(s_h256),
        );

        let web3_sig = Signature {
            v: adjusted_v,
            r: r_h256,
            s: s_h256,
        };

        let signed = tx.encode_pub(raw_tx.chain_id, Some(&web3_sig));

        return Ok(signed);
    }
}
