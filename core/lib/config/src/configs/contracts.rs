// External uses
use serde::Deserialize;
// Workspace uses
use zksync_basic_types::{Address, H256};
// Local uses
use super::envy_load;

/// Data about deployed contracts.
#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct ContractsConfig {
    pub mailbox_facet_addr: Address,
    pub executor_facet_addr: Address,
    pub governance_facet_addr: Address,
    pub diamond_cut_facet_addr: Address,
    pub getters_facet_addr: Address,
    pub verifier_addr: Address,
    pub diamond_init_addr: Address,
    pub diamond_upgrade_init_addr: Address,
    pub diamond_proxy_addr: Address,
    pub validator_timelock_addr: Address,
    pub genesis_tx_hash: H256,
    pub l1_erc20_bridge_proxy_addr: Address,
    pub l1_erc20_bridge_impl_addr: Address,
    pub l2_erc20_bridge_addr: Address,
    pub l1_allow_list_addr: Address,
    pub l2_testnet_paymaster_addr: Option<Address>,
}

impl ContractsConfig {
    pub fn from_env() -> Self {
        envy_load("contracts", "CONTRACTS_")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::configs::test_utils::{addr, hash, set_env};

    fn expected_config() -> ContractsConfig {
        ContractsConfig {
            mailbox_facet_addr: addr("0f6Fa881EF414Fc6E818180657c2d5CD7Ac6cCAd"),
            executor_facet_addr: addr("18B631537801963A964211C0E86645c1aBfbB2d3"),
            governance_facet_addr: addr("1e12b20BE86bEc3A0aC95aA52ade345cB9AE7a32"),
            diamond_cut_facet_addr: addr("8656770FA78c830456B00B4fFCeE6b1De0e1b888"),
            getters_facet_addr: addr("8656770FA78c830456B00B4fFCeE6b1De0e1b888"),
            verifier_addr: addr("34782eE00206EAB6478F2692caa800e4A581687b"),
            diamond_init_addr: addr("FFC35A5e767BE36057c34586303498e3de7C62Ba"),
            diamond_upgrade_init_addr: addr("FFC35A5e767BE36057c34586303498e3de7C62Ba"),
            diamond_proxy_addr: addr("F00B988a98Ca742e7958DeF9F7823b5908715f4a"),
            validator_timelock_addr: addr("F00B988a98Ca742e7958DeF9F7823b5908715f4a"),
            genesis_tx_hash: hash(
                "b99ebfea46cbe05a21cd80fe5597d97b204befc52a16303f579c607dc1ac2e2e",
            ),
            l1_erc20_bridge_proxy_addr: addr("8656770FA78c830456B00B4fFCeE6b1De0e1b888"),
            l1_erc20_bridge_impl_addr: addr("8656770FA78c830456B00B4fFCeE6b1De0e1b888"),
            l2_erc20_bridge_addr: addr("8656770FA78c830456B00B4fFCeE6b1De0e1b888"),
            l1_allow_list_addr: addr("8656770FA78c830456B00B4fFCeE6b1De0e1b888"),
            l2_testnet_paymaster_addr: Some(addr("FC073319977e314F251EAE6ae6bE76B0B3BAeeCF")),
        }
    }

    #[test]
    fn from_env() {
        let config = r#"
CONTRACTS_MAILBOX_FACET_ADDR="0x0f6Fa881EF414Fc6E818180657c2d5CD7Ac6cCAd"
CONTRACTS_EXECUTOR_FACET_ADDR="0x18B631537801963A964211C0E86645c1aBfbB2d3"
CONTRACTS_GOVERNANCE_FACET_ADDR="0x1e12b20BE86bEc3A0aC95aA52ade345cB9AE7a32"
CONTRACTS_DIAMOND_CUT_FACET_ADDR="0x8656770FA78c830456B00B4fFCeE6b1De0e1b888"
CONTRACTS_GETTERS_FACET_ADDR="0x8656770FA78c830456B00B4fFCeE6b1De0e1b888"
CONTRACTS_VERIFIER_ADDR="0x34782eE00206EAB6478F2692caa800e4A581687b"
CONTRACTS_DIAMOND_INIT_ADDR="0xFFC35A5e767BE36057c34586303498e3de7C62Ba"
CONTRACTS_DIAMOND_UPGRADE_INIT_ADDR="0xFFC35A5e767BE36057c34586303498e3de7C62Ba"
CONTRACTS_DIAMOND_PROXY_ADDR="0xF00B988a98Ca742e7958DeF9F7823b5908715f4a"
CONTRACTS_VALIDATOR_TIMELOCK_ADDR="0xF00B988a98Ca742e7958DeF9F7823b5908715f4a"
CONTRACTS_GENESIS_TX_HASH="0xb99ebfea46cbe05a21cd80fe5597d97b204befc52a16303f579c607dc1ac2e2e"
CONTRACTS_L1_ERC20_BRIDGE_PROXY_ADDR="0x8656770FA78c830456B00B4fFCeE6b1De0e1b888"
CONTRACTS_L1_ALLOW_LIST_ADDR="0x8656770FA78c830456B00B4fFCeE6b1De0e1b888"
CONTRACTS_L1_ERC20_BRIDGE_IMPL_ADDR="0x8656770FA78c830456B00B4fFCeE6b1De0e1b888"
CONTRACTS_L2_ERC20_BRIDGE_ADDR="0x8656770FA78c830456B00B4fFCeE6b1De0e1b888"
CONTRACTS_L2_TESTNET_PAYMASTER_ADDR="FC073319977e314F251EAE6ae6bE76B0B3BAeeCF"
        "#;
        set_env(config);

        let actual = ContractsConfig::from_env();
        assert_eq!(actual, expected_config());
    }
}
