use zksync_basic_types::{Address, H160, H256};
use zksync_crypto_primitives::K256PrivateKey;

#[derive(Debug, Clone, PartialEq)]
pub struct AddressWallet {
    address: Address,
}

impl AddressWallet {
    pub fn from_address(address: Address) -> Self {
        Self { address }
    }

    pub fn address(&self) -> Address {
        self.address
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Wallet {
    address: Address,
    private_key: K256PrivateKey,
    gkms_key_name: Option<String>,
}

impl Wallet {
    pub fn new(private_key: K256PrivateKey) -> Self {
        Self {
            address: private_key.address(),
            private_key,
            gkms_key_name: None,
        }
    }

    pub fn from_private_key_bytes(
        private_key_bytes: H256,
        address: Option<Address>,
    ) -> anyhow::Result<Self> {
        let private_key = K256PrivateKey::from_bytes(private_key_bytes)?;
        let calculated_address = private_key.address();
        if let Some(address) = address {
            anyhow::ensure!(
                calculated_address == address,
                "Malformed wallet, address doesn't correspond private_key"
            );
        }

        Ok(Self {
            address: calculated_address,
            private_key,
            gkms_key_name: None,
        })
    }

    pub fn from_gkms_signer(address: Address, key_name: String) -> anyhow::Result<Self> {
        // we need to assign random private key for ignore_private_key wallets anyway to keep compatibility with existing systems
        Ok(Self {
            address,
            private_key: K256PrivateKey::random(),
            gkms_key_name: Some(key_name),
        })
    }

    pub fn address(&self) -> Address {
        self.address
    }

    pub fn private_key(&self) -> &K256PrivateKey {
        &self.private_key
    }

    pub fn gkms_key_name(&self) -> Option<String> {
        self.gkms_key_name.clone()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EthSender {
    pub operator: Wallet,
    pub blob_operator: Option<Wallet>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StateKeeper {
    pub fee_account: AddressWallet,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TokenMultiplierSetter {
    pub wallet: Wallet,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Wallets {
    pub eth_sender: Option<EthSender>,
    pub state_keeper: Option<StateKeeper>,
    pub token_multiplier_setter: Option<TokenMultiplierSetter>,
}

impl Wallets {
    pub fn for_tests() -> Wallets {
        Wallets {
            eth_sender: Some(EthSender {
                operator: Wallet::from_private_key_bytes(H256::repeat_byte(0x1), None).unwrap(),
                blob_operator: Some(
                    Wallet::from_private_key_bytes(H256::repeat_byte(0x2), None).unwrap(),
                ),
            }),
            state_keeper: Some(StateKeeper {
                fee_account: AddressWallet::from_address(H160::repeat_byte(0x3)),
            }),
            token_multiplier_setter: Some(TokenMultiplierSetter {
                wallet: Wallet::from_private_key_bytes(H256::repeat_byte(0x4), None).unwrap(),
            }),
        }
    }
}
