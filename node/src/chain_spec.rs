use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use parami_runtime::{
    opaque::SessionKeys, AccountId, AuthorityDiscoveryConfig, BabeConfig, Balance, BalancesConfig,
    Block, ElectionsConfig, GenesisConfig, GrandpaConfig, ImOnlineConfig, SessionConfig, Signature,
    SocietyConfig, StakerStatus, StakingConfig, SudoConfig, SystemConfig, TechnicalCommitteeConfig,
    BABE_GENESIS_EPOCH_CONFIG, DOLLARS, MAX_NOMINATIONS, WASM_BINARY,
};
use sc_chain_spec::ChainSpecExtension;
use sc_service::ChainType;
use sc_telemetry::TelemetryEndpoints;
use serde::{Deserialize, Serialize};
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_consensus_babe::AuthorityId as BabeId;
use sp_core::{ed25519, sr25519, Pair, Public};
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::{
    traits::{IdentifyAccount, Verify},
    Perbill,
};

type AccountPublic = <Signature as Verify>::Signer;

const PROPERTIES: &str = r#"{"tokenDecimals": 18 , "tokenSymbol": "AD3"}"#;
const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

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
    /// The light sync state extension used by the sync-state rpc.
    pub light_sync_state: sc_sync_state_rpc::LightSyncStateExtension,
}

/// Specialized `ChainSpec`.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig, Extensions>;

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

fn staging_testnet_config_genesis() -> GenesisConfig {
    // stash, controller, session-key
    // generated with secret:
    // for i in 1 2 3 4 ; do for j in stash controller; do subkey inspect "$secret"/fir/$j/$i; done; done
    //
    // and
    //
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

    testnet_genesis(
        initial_authorities,
        vec![],
        root_key,
        Some(endowed_accounts),
    )
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
        Some("ad3"),
        serde_json::from_str(PROPERTIES).ok(),
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
    initial_nominators: Vec<AccountId>,
    root_key: AccountId,
    endowed_accounts: Option<Vec<AccountId>>,
) -> GenesisConfig {
    let wasm_binary = WASM_BINARY
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

    // stakers: all validators and nominators.
    let mut rng = rand::thread_rng();
    let stakers = initial_authorities
        .iter()
        .map(|x| (x.0.clone(), x.1.clone(), STASH, StakerStatus::Validator))
        .chain(initial_nominators.iter().map(|x| {
            use rand::{seq::SliceRandom, Rng};
            let limit = (MAX_NOMINATIONS as usize).min(initial_authorities.len());
            let count = rng.gen::<usize>() % limit;
            let nominations = initial_authorities
                .as_slice()
                .choose_multiple(&mut rng, count)
                .into_iter()
                .map(|choice| choice.0.clone())
                .collect::<Vec<_>>();
            (
                x.clone(),
                x.clone(),
                STASH,
                StakerStatus::Nominator(nominations),
            )
        }))
        .collect::<Vec<_>>();

    let num_endowed_accounts = endowed_accounts.len();

    const ENDOWMENT: Balance = 10_000_000 * DOLLARS;
    const STASH: Balance = ENDOWMENT / 1000;

    GenesisConfig {
        system: SystemConfig {
            code: wasm_binary.to_vec(),
        },

        authority_discovery: AuthorityDiscoveryConfig { keys: vec![] },
        babe: BabeConfig {
            authorities: vec![],
            epoch_config: Some(BABE_GENESIS_EPOCH_CONFIG),
        },
        balances: BalancesConfig {
            balances: endowed_accounts
                .iter()
                .cloned()
                .map(|x| (x, ENDOWMENT))
                .collect(),
        },
        elections: ElectionsConfig {
            members: endowed_accounts
                .iter()
                .take((num_endowed_accounts + 1) / 2)
                .cloned()
                .map(|member| (member, STASH))
                .collect(),
        },
        grandpa: GrandpaConfig {
            authorities: vec![],
        },
        im_online: ImOnlineConfig { keys: vec![] },
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
        society: SocietyConfig {
            members: endowed_accounts
                .iter()
                .take((num_endowed_accounts + 1) / 2)
                .cloned()
                .collect(),
            pot: 0,
            max_members: 999,
        },
        staking: StakingConfig {
            validator_count: initial_authorities.len() as u32,
            minimum_validator_count: initial_authorities.len() as u32,
            invulnerables: initial_authorities.iter().map(|x| x.0.clone()).collect(),
            slash_reward_fraction: Perbill::from_percent(10),
            stakers,
            ..Default::default()
        },
        sudo: SudoConfig { key: root_key },
        technical_committee: TechnicalCommitteeConfig {
            members: endowed_accounts
                .iter()
                .take((num_endowed_accounts + 1) / 2)
                .cloned()
                .collect(),
            phantom: Default::default(),
        },

        assets: Default::default(),
        council: Default::default(),
        democracy: Default::default(),
        technical_membership: Default::default(),
        treasury: Default::default(),
        vesting: Default::default(),

        // airdrop: Default::default(),
        // ad: Default::default(),
        advertiser: Default::default(),
        did: Default::default(),
        linker: Default::default(),
        magic: Default::default(),
        nft: Default::default(),
        swap: Default::default(),
        tag: Default::default(),
    }
}

fn development_config_genesis() -> GenesisConfig {
    testnet_genesis(
        vec![authority_keys_from_seed("Alice")],
        vec![],
        get_account_id_from_seed::<sr25519::Public>("Alice"),
        None,
    )
}

/// Development config (single validator Alice)
pub fn development_config() -> ChainSpec {
    ChainSpec::from_genesis(
        "Development",
        "dev",
        ChainType::Development,
        development_config_genesis,
        vec![],
        None,
        Some("ad3"),
        serde_json::from_str(PROPERTIES).ok(),
        Default::default(),
    )
}

fn local_testnet_genesis() -> GenesisConfig {
    testnet_genesis(
        vec![
            authority_keys_from_seed("Alice"),
            authority_keys_from_seed("Bob"),
        ],
        vec![],
        get_account_id_from_seed::<sr25519::Public>("Alice"),
        None,
    )
}

/// Local testnet config (multivalidator Alice + Bob)
pub fn local_testnet_config() -> ChainSpec {
    ChainSpec::from_genesis(
        "Local Testnet",
        "local_testnet",
        ChainType::Local,
        local_testnet_genesis,
        vec![],
        None,
        Some("ad3"),
        serde_json::from_str(PROPERTIES).ok(),
        Default::default(),
    )
}
