// This file is part of Substrate.

// Copyright (C) 2018-2021 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Substrate chain configurations.

use grandpa_primitives::AuthorityId as GrandpaId;
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use parami_runtime::constants::currency::*;
use parami_runtime::Block;
use parami_runtime::{
    wasm_binary_unwrap, AuthorityDiscoveryConfig, BabeConfig, BalancesConfig, CouncilConfig,
    DemocracyConfig, ElectionsConfig, GrandpaConfig, ImOnlineConfig, SessionConfig, SessionKeys,
    SocietyConfig, StakerStatus, StakingConfig, SudoConfig, SystemConfig, TechnicalCommitteeConfig,
};
use sc_chain_spec::ChainSpecExtension;
use sc_service::ChainType;
use sc_telemetry::TelemetryEndpoints;
use serde::{Deserialize, Serialize};
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_consensus_babe::AuthorityId as BabeId;
use sp_core::{ed25519, sr25519, Pair, Public};
use sp_runtime::{
    traits::{IdentifyAccount, Verify},
    Perbill,
};

pub use parami_primitives::{AccountId, Balance, Signature};
pub use parami_runtime::GenesisConfig;

type AccountPublic = <Signature as Verify>::Signer;

const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";
const TOKEN_PROPERTIES: &str = r#"
        {
            "tokenDecimals": 15,
            "tokenSymbol": "AD3"
        }"#;
/// Node `ChainSpec` extensions.
///
/// Additional parameters for some Substrate core modules,
/// customizable from the chain spec.
#[derive(Default, Clone, Serialize, Deserialize, ChainSpecExtension)]
#[serde(rename_all = "camelCase")]
pub struct Extensions {
    /// Block numbers with known hashes.
    pub fork_blocks: sc_client_api::ForkBlocks<Block>,
    /// Known bad block hashes.
    pub bad_blocks: sc_client_api::BadBlocks<Block>,
}

/// Specialized `ChainSpec`.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig, Extensions>;
/// Flaming Fir testnet generator
pub fn flaming_fir_config() -> Result<ChainSpec, String> {
    ChainSpec::from_json_bytes(&include_bytes!("../res/flaming-fir.json")[..])
}

fn session_keys(
    grandpa: GrandpaId,
    babe: BabeId,
    im_online: ImOnlineId,
    authority_discovery: AuthorityDiscoveryId,
) -> SessionKeys {
    SessionKeys {
        grandpa,
        babe,
        im_online,
        authority_discovery,
    }
}

#[derive(Serialize, Deserialize)]
struct Allocation {
    balances: Vec<(String, String)>,
}

// Give each initial participant the allocation,
fn get_initial_allocation() -> Result<(Vec<(AccountId, Balance)>, Balance), String> {
    use std::fs::File;
    use std::io::Read;

    let mut file = File::open("initial_drop.json").expect("Unable to open");
    let mut data = String::new();
    file.read_to_string(&mut data).unwrap();

    let json: Allocation = serde_json::from_str(&data).unwrap();
    let balances_json = json.balances;

    let balances: Vec<(AccountId, Balance)> = balances_json
        .clone()
        .into_iter()
        .map(|elem| {
            return (
                elem.0.parse().unwrap(),
                elem.1.to_string().parse::<Balance>().unwrap(),
            );
        })
        .collect();

    let total: Balance = balances_json
        .into_iter()
        .map(|e| e.1.to_string().parse::<Balance>().unwrap())
        .sum();
    Ok((balances, total))
}

fn staging_testnet_config_genesis() -> GenesisConfig {
    // stash, controller, session-key
    // generated with secret:
    // for i in 1 2 3 4 ; do for j in stash controller; do subkey inspect "$secret"/fir/$j/$i; done; done
    // and
    // for i in 1 2 3 4 ; do for j in session; do subkey --ed25519 inspect "$secret"//fir//$j//$i; done; done

    let initial_authorities: Vec<(
        AccountId,
        AccountId,
        GrandpaId,
        BabeId,
        ImOnlineId,
        AuthorityDiscoveryId,
    )> = vec![
        (
            "5ELomM16X264LYbPdRdQ884MrauXQWicxv632g8TotfRiDgi"
                .parse()
                .unwrap(),
            "5FmkZKr2b5euYc4szf3QKREHDh9b47ihwaLsbiku8TKkkb7W"
                .parse()
                .unwrap(),
            "5EZbXLZb18WXh6DERzXYJrh1YuK3AVockvk5oernG9eZJz5b"
                .parse::<ed25519::Public>()
                .unwrap()
                .into(),
            "5FmkZKr2b5euYc4szf3QKREHDh9b47ihwaLsbiku8TKkkb7W"
                .parse::<sr25519::Public>()
                .unwrap()
                .into(),
            "5FmkZKr2b5euYc4szf3QKREHDh9b47ihwaLsbiku8TKkkb7W"
                .parse::<sr25519::Public>()
                .unwrap()
                .into(),
            "5FmkZKr2b5euYc4szf3QKREHDh9b47ihwaLsbiku8TKkkb7W"
                .parse::<sr25519::Public>()
                .unwrap()
                .into(),
        ),
        (
            "5F7g57PkN9SNrcJ5LBn9rjQWMFU9FSMX7UaNRJtMkm21doLi"
                .parse()
                .unwrap(),
            "5CXVyypcm7NG1SN4wCWhXkAyLuzNtpJwjK6PA1R5hPh5gre3"
                .parse()
                .unwrap(),
            "5D46UMD5kwiHnRuPDZyYqahQg62ceCHGAwYcodkfH4TRusY5"
                .parse::<ed25519::Public>()
                .unwrap()
                .into(),
            "5CXVyypcm7NG1SN4wCWhXkAyLuzNtpJwjK6PA1R5hPh5gre3"
                .parse::<sr25519::Public>()
                .unwrap()
                .into(),
            "5CXVyypcm7NG1SN4wCWhXkAyLuzNtpJwjK6PA1R5hPh5gre3"
                .parse::<sr25519::Public>()
                .unwrap()
                .into(),
            "5CXVyypcm7NG1SN4wCWhXkAyLuzNtpJwjK6PA1R5hPh5gre3"
                .parse::<sr25519::Public>()
                .unwrap()
                .into(),
        ),
        (
            "5FREQDU6Mk5qDb6tRjdgoVHFa83u11i3V4FKP681URA6q5iv"
                .parse()
                .unwrap(),
            "5GEz8dHNYrXukkfd5wEXZSCwE1WFYseV5eDuu8vPXv5vAnTb"
                .parse()
                .unwrap(),
            "5EdF93kPfcmkMcyieF2s8pwRpjWXfu9LJgrRhA5KvBz5DhNM"
                .parse::<ed25519::Public>()
                .unwrap()
                .into(),
            "5GEz8dHNYrXukkfd5wEXZSCwE1WFYseV5eDuu8vPXv5vAnTb"
                .parse::<sr25519::Public>()
                .unwrap()
                .into(),
            "5GEz8dHNYrXukkfd5wEXZSCwE1WFYseV5eDuu8vPXv5vAnTb"
                .parse::<sr25519::Public>()
                .unwrap()
                .into(),
            "5GEz8dHNYrXukkfd5wEXZSCwE1WFYseV5eDuu8vPXv5vAnTb"
                .parse::<sr25519::Public>()
                .unwrap()
                .into(),
        ),
        (
            "5EUnFvq6H21XPwbqUEefKzZfa1xUtZLiW4XZFLF9o3vzAQF8"
                .parse()
                .unwrap(),
            "5E5Ksdp62in5PBayKCM2mgWkey5RrZiggCP62E4oxXndXvp3"
                .parse()
                .unwrap(),
            "5EnrBxqKWPGd3oHYZHQRwQXCnvXKBCYX4wb6AqMCekSrcyDf"
                .parse::<ed25519::Public>()
                .unwrap()
                .into(),
            "5E5Ksdp62in5PBayKCM2mgWkey5RrZiggCP62E4oxXndXvp3"
                .parse::<sr25519::Public>()
                .unwrap()
                .into(),
            "5E5Ksdp62in5PBayKCM2mgWkey5RrZiggCP62E4oxXndXvp3"
                .parse::<sr25519::Public>()
                .unwrap()
                .into(),
            "5E5Ksdp62in5PBayKCM2mgWkey5RrZiggCP62E4oxXndXvp3"
                .parse::<sr25519::Public>()
                .unwrap()
                .into(),
        ),
        (
            "5CXieqfZ5s2wi2RBgarJnLa1KZFjRQ1KEjVS7dwAuETFFstG"
                .parse()
                .unwrap(),
            "5DyVEtzV6nYCJKNpQTxiVxEUG6Dx4fkqk6PrTGZCpZ87Ly5c"
                .parse()
                .unwrap(),
            "5F6YkYDBoDEaEf7mKgtLyh53MDhvgu8S9o5UW8nsSL8Kcwku"
                .parse::<ed25519::Public>()
                .unwrap()
                .into(),
            "5DyVEtzV6nYCJKNpQTxiVxEUG6Dx4fkqk6PrTGZCpZ87Ly5c"
                .parse::<sr25519::Public>()
                .unwrap()
                .into(),
            "5DyVEtzV6nYCJKNpQTxiVxEUG6Dx4fkqk6PrTGZCpZ87Ly5c"
                .parse::<sr25519::Public>()
                .unwrap()
                .into(),
            "5DyVEtzV6nYCJKNpQTxiVxEUG6Dx4fkqk6PrTGZCpZ87Ly5c"
                .parse::<sr25519::Public>()
                .unwrap()
                .into(),
        ),
    ];

    let root_key: AccountId = "5CY9GuoHjBrYRvLU7itALA1FCDRGhmPrRJR3XgvkodPcpMV9"
        .parse()
        .unwrap();

    // 5 endowed accounts
    let endowed_accounts: Vec<AccountId> = vec![
        "5FvDj5YQF3m1MUsgHDd4mSnCNNFevFuVQnQ3Bbb7Yi6kxE3a"
            .parse()
            .unwrap(),
        "5HdjGZdqsXR1K35DZtFa7wHT1nxm32pSA8GqPkfqMWWn3AsV"
            .parse()
            .unwrap(),
        "5Dk3RDfwZNkgW6tdqoXVZdY7fTVBRZ1qwDHjiy3o8RdzdjBS"
            .parse()
            .unwrap(),
        "5HKLJdgA7LciNspVtkqYRc1wX3EDHWax3StFwjiHNZEvtX3G"
            .parse()
            .unwrap(),
        "5FnaseCGczaTq5xjgyvQLnu9ZFcNzR99KGXZ3A9yDDHp6fHv"
            .parse()
            .unwrap(),
    ];

    let mut genesis = testnet_genesis(initial_authorities, root_key, Some(endowed_accounts));

    let (initial_alloc, _initial_total) =
        get_initial_allocation().expect("can not get initial allocation");

    genesis.balances.balances = initial_alloc;
    genesis
}

/// Staging testnet config.
pub fn staging_testnet_config() -> ChainSpec {
    let boot_nodes = vec![
        "/dns/us1.dev.ad3.app/tcp/30333/p2p/12D3KooWMHs1sd41Gk8UmFJjhwn7Pmo58nWNe6pi9Dz2p8DaFDmw"
            .parse()
            .unwrap(),
        "/dns/sg1.dev.ad3.app/tcp/30333/p2p/12D3KooWGVEHNi64iL1VyKqjonsQUMFYrZsVmCHhq2Eq75S5oFS1"
            .parse()
            .unwrap(),
        "/dns/sg2.dev.ad3.app/tcp/30333/p2p/12D3KooWDQY6ExjvaSzT7vjNbPpeKGP7nLvUaEp6iiqPtz21v9yd"
            .parse()
            .unwrap(),
    ];
    let properties = serde_json::from_str(TOKEN_PROPERTIES).unwrap();
    ChainSpec::from_genesis(
        "Parami Dana",
        "parami_dana",
        ChainType::Live,
        staging_testnet_config_genesis,
        boot_nodes,
        Some(
            TelemetryEndpoints::new(vec![(STAGING_TELEMETRY_URL.to_string(), 0)])
                .expect("Staging telemetry url is valid; qed"),
        ),
        None,
        properties,
        Default::default(),
    )
}

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

/// Helper function to generate an account ID from seed
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
    AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
    AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Helper function to generate stash, controller and session key from seed
pub fn authority_keys_from_seed(
    seed: &str,
) -> (
    AccountId,
    AccountId,
    GrandpaId,
    BabeId,
    ImOnlineId,
    AuthorityDiscoveryId,
) {
    (
        get_account_id_from_seed::<sr25519::Public>(&format!("{}//stash", seed)),
        get_account_id_from_seed::<sr25519::Public>(seed),
        get_from_seed::<GrandpaId>(seed),
        get_from_seed::<BabeId>(seed),
        get_from_seed::<ImOnlineId>(seed),
        get_from_seed::<AuthorityDiscoveryId>(seed),
    )
}

/// Helper function to create GenesisConfig for testing
pub fn testnet_genesis(
    initial_authorities: Vec<(
        AccountId,
        AccountId,
        GrandpaId,
        BabeId,
        ImOnlineId,
        AuthorityDiscoveryId,
    )>,
    root_key: AccountId,
    endowed_accounts: Option<Vec<AccountId>>,
) -> GenesisConfig {
    let mut endowed_accounts: Vec<AccountId> = endowed_accounts.unwrap_or_else(|| {
        vec![
            get_account_id_from_seed::<sr25519::Public>("Alice"),
            get_account_id_from_seed::<sr25519::Public>("Bob"),
            get_account_id_from_seed::<sr25519::Public>("Charlie"),
            get_account_id_from_seed::<sr25519::Public>("Dave"),
            get_account_id_from_seed::<sr25519::Public>("Eve"),
            get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
            get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
            get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
            get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
            get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
        ]
    });
    initial_authorities.iter().for_each(|x| {
        if !endowed_accounts.contains(&x.0) {
            endowed_accounts.push(x.0.clone())
        }
    });

    let num_endowed_accounts = endowed_accounts.len();

    const ENDOWMENT: Balance = 10_000_000 * DOLLARS;
    const STASH: Balance = ENDOWMENT / 1000;

    GenesisConfig {
        system: SystemConfig {
            code: wasm_binary_unwrap().to_vec(),
            changes_trie_config: Default::default(),
        },
        balances: BalancesConfig {
            balances: endowed_accounts
                .iter()
                .cloned()
                .map(|x| (x, ENDOWMENT))
                .collect(),
        },
        session: SessionConfig {
            keys: initial_authorities
                .iter()
                .map(|x| {
                    (
                        x.0.clone(),
                        x.0.clone(),
                        session_keys(x.2.clone(), x.3.clone(), x.4.clone(), x.5.clone()),
                    )
                })
                .collect::<Vec<_>>(),
        },
        staking: StakingConfig {
            validator_count: initial_authorities.len() as u32 * 2,
            minimum_validator_count: initial_authorities.len() as u32,
            stakers: initial_authorities
                .iter()
                .map(|x| (x.0.clone(), x.1.clone(), STASH, StakerStatus::Validator))
                .collect(),
            invulnerables: initial_authorities.iter().map(|x| x.0.clone()).collect(),
            slash_reward_fraction: Perbill::from_percent(10),
            ..Default::default()
        },
        democracy: DemocracyConfig::default(),
        elections: ElectionsConfig {
            members: endowed_accounts
                .iter()
                .take((num_endowed_accounts + 1) / 2)
                .cloned()
                .map(|member| (member, STASH))
                .collect(),
        },
        council: CouncilConfig::default(),
        technical_committee: TechnicalCommitteeConfig {
            members: endowed_accounts
                .iter()
                .take((num_endowed_accounts + 1) / 2)
                .cloned()
                .collect(),
            phantom: Default::default(),
        },
        sudo: SudoConfig { key: root_key },
        babe: BabeConfig {
            authorities: vec![],
            epoch_config: Some(parami_runtime::BABE_GENESIS_EPOCH_CONFIG),
        },
        im_online: ImOnlineConfig { keys: vec![] },
        authority_discovery: AuthorityDiscoveryConfig { keys: vec![] },
        grandpa: GrandpaConfig {
            authorities: vec![],
        },
        technical_membership: Default::default(),
        treasury: Default::default(),
        society: SocietyConfig {
            members: endowed_accounts
                .iter()
                .take((num_endowed_accounts + 1) / 2)
                .cloned()
                .collect(),
            pot: 0,
            max_members: 999,
        },
        vesting: Default::default(),

        airdrop: Default::default(),
        ad: Default::default(),
    }
}

fn development_config_genesis() -> GenesisConfig {
    testnet_genesis(
        vec![authority_keys_from_seed("Alice")],
        get_account_id_from_seed::<sr25519::Public>("Alice"),
        None,
    )
}

/// Development config (single validator Alice)
pub fn development_config() -> ChainSpec {
    let properties = serde_json::from_str(TOKEN_PROPERTIES).unwrap();
    ChainSpec::from_genesis(
        "Development",
        "dev",
        ChainType::Development,
        development_config_genesis,
        vec![],
        None,
        None,
        properties,
        Default::default(),
    )
}

fn local_testnet_genesis() -> GenesisConfig {
    testnet_genesis(
        vec![
            authority_keys_from_seed("Alice"),
            authority_keys_from_seed("Bob"),
        ],
        get_account_id_from_seed::<sr25519::Public>("Alice"),
        None,
    )
}

/// Local testnet config (multivalidator Alice + Bob)
pub fn local_testnet_config() -> ChainSpec {
    let properties = serde_json::from_str(TOKEN_PROPERTIES).unwrap();
    ChainSpec::from_genesis(
        "Local Testnet",
        "local_testnet",
        ChainType::Local,
        local_testnet_genesis,
        vec![],
        None,
        None,
        properties,
        Default::default(),
    )
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::service::{new_full_base, new_light_base, NewFullBase};
    use sc_service_test;
    use sp_runtime::BuildStorage;

    fn local_testnet_genesis_instant_single() -> GenesisConfig {
        testnet_genesis(
            vec![authority_keys_from_seed("Alice")],
            get_account_id_from_seed::<sr25519::Public>("Alice"),
            None,
        )
    }

    /// Local testnet config (single validator - Alice)
    pub fn integration_test_config_with_single_authority() -> ChainSpec {
        let properties = serde_json::from_str(TOKEN_PROPERTIES).unwrap();
        ChainSpec::from_genesis(
            "Integration Test",
            "test",
            ChainType::Development,
            local_testnet_genesis_instant_single,
            vec![],
            None,
            None,
            properties,
            Default::default(),
        )
    }

    /// Local testnet config (multivalidator Alice + Bob)
    pub fn integration_test_config_with_two_authorities() -> ChainSpec {
        let properties = serde_json::from_str(TOKEN_PROPERTIES).unwrap();
        ChainSpec::from_genesis(
            "Integration Test",
            "test",
            ChainType::Development,
            local_testnet_genesis,
            vec![],
            None,
            None,
            properties,
            Default::default(),
        )
    }

    #[test]
    #[ignore]
    fn test_connectivity() {
        sc_service_test::connectivity(
            integration_test_config_with_two_authorities(),
            |config| {
                let NewFullBase {
                    task_manager,
                    client,
                    network,
                    transaction_pool,
                    ..
                } = new_full_base(config, |_, _| ())?;
                Ok(sc_service_test::TestNetComponents::new(
                    task_manager,
                    client,
                    network,
                    transaction_pool,
                ))
            },
            |config| {
                let (keep_alive, _, client, network, transaction_pool) = new_light_base(config)?;
                Ok(sc_service_test::TestNetComponents::new(
                    keep_alive,
                    client,
                    network,
                    transaction_pool,
                ))
            },
        );
    }

    #[test]
    fn test_create_development_chain_spec() {
        development_config().build_storage().unwrap();
    }

    #[test]
    fn test_create_local_testnet_chain_spec() {
        local_testnet_config().build_storage().unwrap();
    }

    #[test]
    fn test_staging_test_net_chain_spec() {
        staging_testnet_config().build_storage().unwrap();
    }
}
