use parami_dana_runtime::{AccountId, AuraConfig, AuraId, GenesisConfig, GrandpaConfig, Signature};
use sc_service::ChainType;
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_core::{sr25519, Pair, Public};
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::{
    traits::{IdentifyAccount, Verify},
    Perbill,
};

// The URL for the telemetry server.
// const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig>;

/// Helper function to generate a crypto pair from seed
pub fn get_public_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

type AccountPublic = <Signature as Verify>::Signer;

/// Helper function to generate an account ID from seed
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
    AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
    AccountPublic::from(get_public_from_seed::<TPublic>(seed)).into_account()
}

/// Helper function to generate stash, controller and session key from seed
pub fn authority_keys_from_seed(seed: &str) -> (AccountId, AccountId, AuraId, GrandpaId) {
    (
        get_account_id_from_seed::<sr25519::Public>(&format!("{}//stash", seed)),
        get_account_id_from_seed::<sr25519::Public>(seed),
        get_public_from_seed::<AuraId>(seed),
        get_public_from_seed::<GrandpaId>(seed),
    )
}

pub fn development_config() -> ChainSpec {
    // Give your base currency a unit name and decimal places
    let mut properties = sc_chain_spec::Properties::new();
    properties.insert("tokenSymbol".into(), "AD3".into());
    properties.insert("tokenDecimals".into(), 18u32.into());
    properties.insert("ss58Format".into(), 42u32.into());

    ChainSpec::from_genesis(
        // Name
        "Development",
        // ID
        "dev",
        ChainType::Development,
        move || {
            testnet_genesis(
                vec![authority_keys_from_seed("Alice")],
                vec![],
                get_account_id_from_seed::<sr25519::Public>("Alice"),
                None,
            )
        },
        // Bootnodes
        Vec::new(),
        // Telemetry
        None,
        // Protocol ID
        None,
        // Fork Id
        None,
        // Properties
        Some(properties),
        // Extensions
        Default::default(),
    )
}

pub fn local_testnet_config() -> ChainSpec {
    // Give your base currency a unit name and decimal places
    let mut properties = sc_chain_spec::Properties::new();
    properties.insert("tokenSymbol".into(), "AD3".into());
    properties.insert("tokenDecimals".into(), 18u32.into());
    properties.insert("ss58Format".into(), 42u32.into());

    ChainSpec::from_genesis(
        // Name
        "Local Testnet",
        // ID
        "local_testnet",
        ChainType::Local,
        move || {
            testnet_genesis(
                vec![
                    authority_keys_from_seed("Alice"),
                    authority_keys_from_seed("Bob"),
                ],
                vec![],
                get_account_id_from_seed::<sr25519::Public>("Alice"),
                None,
            )
        },
        // Bootnodes
        Vec::new(),
        // Telemetry
        None,
        // Protocol ID
        Some("ad3"),
        // Fork Id
        None,
        // Properties
        Some(properties),
        // Extensions
        Default::default(),
    )
}

fn testnet_genesis(
    initial_authorities: Vec<(AccountId, AccountId, AuraId, GrandpaId)>,
    initial_nominators: Vec<AccountId>,
    root_key: AccountId,
    endowed_accounts: Option<Vec<AccountId>>,
) -> parami_dana_runtime::GenesisConfig {
    let wasm_binary = parami_dana_runtime::WASM_BINARY
        .ok_or_else(|| "Development wasm not available".to_string())
        .unwrap();

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
    // endow all authorities and nominators.
    initial_authorities
        .iter()
        .map(|x| &x.0)
        .chain(initial_nominators.iter())
        .for_each(|x| {
            if !endowed_accounts.contains(x) {
                endowed_accounts.push(x.clone())
            }
        });

    let num_endowed_accounts = endowed_accounts.len();

    const ENDOWMENT: parami_dana_runtime::Balance = 10_000_000 * parami_dana_runtime::DOLLARS;
    let grandpa = initial_authorities
        .iter()
        .map(|x| (x.3.clone(), 1))
        .collect();

    parami_dana_runtime::GenesisConfig {
        system: parami_dana_runtime::SystemConfig {
            code: wasm_binary.to_vec(),
        },

        balances: parami_dana_runtime::BalancesConfig {
            balances: endowed_accounts
                .iter()
                .cloned()
                .map(|x| (x, ENDOWMENT))
                .collect(),
        },
        assets: Default::default(),

        aura: AuraConfig {
            authorities: initial_authorities.iter().map(|x| (x.2.clone())).collect(),
        },
        grandpa: GrandpaConfig {
            authorities: grandpa,
        },

        democracy: Default::default(),
        council: Default::default(),
        technical_committee: parami_dana_runtime::TechnicalCommitteeConfig {
            members: endowed_accounts
                .iter()
                .take((num_endowed_accounts + 1) / 2)
                .cloned()
                .collect(),
            phantom: Default::default(),
        },
        technical_membership: Default::default(),
        treasury: Default::default(),

        sudo: parami_dana_runtime::SudoConfig {
            key: Some(root_key),
        },

        ad: Default::default(),
        advertiser: Default::default(),
        did: Default::default(),
        linker: Default::default(),
        nft: Default::default(),
        swap: Default::default(),
        tag: Default::default(),
    }
}
