use cess_node_runtime::{
	AccountId, AuthorityDiscoveryConfig, BabeConfig, BalancesConfig, Balance, CouncilConfig,
	GenesisConfig, GrandpaConfig, Block, IndicesConfig,
	ImOnlineConfig, SessionConfig, Signature, StakingConfig, SessionKeys, SudoConfig, StakerStatus,
	SystemConfig, TechnicalCommitteeConfig, wasm_binary_unwrap, MaxNominations, DOLLARS
};
use sc_service::ChainType;
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_consensus_babe::AuthorityId as BabeId;
use sp_core::{crypto::UncheckedInto, sr25519, Pair, Public};
use grandpa_primitives::AuthorityId as GrandpaId;
use sp_runtime::{
	traits::{IdentifyAccount, Verify},
	Perbill,
};
use hex_literal::hex;
use serde::{Deserialize, Serialize};
use sc_chain_spec::ChainSpecExtension;
use sc_telemetry::TelemetryEndpoints;


// The URL for the telemetry server.
// const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.


/// Generate a crypto pair from seed.
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

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

type AccountPublic = <Signature as Verify>::Signer;

fn session_keys(
	grandpa: GrandpaId,
	babe: BabeId,
	im_online: ImOnlineId,
	authority_discovery: AuthorityDiscoveryId,
) -> SessionKeys {
	SessionKeys { grandpa, babe, im_online, authority_discovery }
}

/// Generate an account ID from seed.
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Helper function to generate stash, controller and session key from seed
pub fn authority_keys_from_seed(seed: &str) -> (AccountId, AccountId, GrandpaId, BabeId, ImOnlineId, AuthorityDiscoveryId) {
	(
		get_account_id_from_seed::<sr25519::Public>(&format!("{}//stash", seed)),
		get_account_id_from_seed::<sr25519::Public>(seed),
		get_from_seed::<GrandpaId>(seed),
		get_from_seed::<BabeId>(seed),
		get_from_seed::<ImOnlineId>(seed),
		get_from_seed::<AuthorityDiscoveryId>(seed),
	)
}

fn cess_testnet_config_genesis() -> GenesisConfig {
	#[rustfmt::skip]
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
			// cXfg2SYcq85nyZ1U4ccx6QnAgSeLQB8aXZ2jstbw9CPGSmhXY
			hex!["1ec940be673d3613e94c4d44e3f4621422c1a0778a53a34b2b45f3118f823c03"].into(),
			// cXffK7BmstE5rXcK8pxKLufkffp9iASMntxUm6ctpR6xS3icV
			hex!["1e3e1c69dfbd27d398e92da4844a9abdc2786ac01588d87a2e1f5ec06ea2a936"].into(),
			// cXiHP2jLVk4iUrJdQVjBc2txoKjHANpomxcfHeCkKcxpesfDy
			hex!["92389e795eef50e9af11c997f82d7bb809b0bb9a76f96b6fb29bd60b7a2cbef7"]
				.unchecked_into(),
			// cXfw8G7ThY48EqDMnQdZbwjmu69yELy3dMcuWyacG9snCMdPx
			hex!["2a4d86b50b2d98c3bfb02c00f9731753b01f8151774544f4e78e11ef4bb1eb79"]
				.unchecked_into(),
			// cXfw8G7ThY48EqDMnQdZbwjmu69yELy3dMcuWyacG9snCMdPx
			hex!["2a4d86b50b2d98c3bfb02c00f9731753b01f8151774544f4e78e11ef4bb1eb79"]
				.unchecked_into(),
			// cXfw8G7ThY48EqDMnQdZbwjmu69yELy3dMcuWyacG9snCMdPx
			hex!["2a4d86b50b2d98c3bfb02c00f9731753b01f8151774544f4e78e11ef4bb1eb79"]
				.unchecked_into(),
		),
		(
			// cXiHpiCFn6xkypa2Q8SroghuME1hBtDDrJYEEAXLwkfy6y277
			hex!["928f11288d2ba2a6c625fbf34366ea533566e6f6b85661d2ea6df4fb6158b14b"].into(),
			// cXiKthh2dyY1taTydtdxiqQwXY1HKZcXvYGmjS2UmuPi2qNDS
			hex!["9422fe68db986721051c6cd9841a6afc2c3edc6de3cd74c7b0e45f50b3d6c671"].into(),
			// cXhWdqfECn1yMd9E6eKT6MD3NqyWGve8F1gLAvM9CAH4RH6LR
			hex!["70186d841754d9f4225f922976682d580e58fce0cfd07696cb6d4324b1ede379"]
				.unchecked_into(),
			// cXfvtwoYtMD1Z8nZrK5JptFSdBAVyjQ1t2i6scvMLYK75ChrP
			hex!["2a20b3a025722789f18ca7e459ec21d4f232b1a1d245272f14248d3cfa8b412a"]
				.unchecked_into(),
			// cXfvtwoYtMD1Z8nZrK5JptFSdBAVyjQ1t2i6scvMLYK75ChrP
			hex!["2a20b3a025722789f18ca7e459ec21d4f232b1a1d245272f14248d3cfa8b412a"]
				.unchecked_into(),
			// cXfvtwoYtMD1Z8nZrK5JptFSdBAVyjQ1t2i6scvMLYK75ChrP
			hex!["2a20b3a025722789f18ca7e459ec21d4f232b1a1d245272f14248d3cfa8b412a"]
				.unchecked_into(),
		),
		(
			// cXic3WhctsJ9cExmjE9vog49xaLuVbDLcFi2odeEnvV5Sbq4f
			hex!["a0749440bc4322cc700bc3678d986f50d2fe91d9322a4a1b3046c3f1bd944940"].into(),
			// cXj4Z6VmYHkD64xEadQCNgFUDaV4F4ibeupuU4ELJa8uWH7mw
			hex!["b4ac8b37a20935b0b390c61cd849ea533b060bb97e32b68801ec3b6220a8e131"].into(),
			// cXjQgQVu9Zuitzkb8oiTYTKt7QUm9jupKbzqVNVwPELwz26rT
			hex!["c4060f93e5bb97ed21491e90ff78268caa67ec84a5d3663ffd65332509fe0dd9"]
				.unchecked_into(),
			// cXfwFY2GaMueyyaSrjMRSQM48mLU7Z5JRmdB4FJE7weicJ7pu
			hex!["2a66038471e6a62a2df2195efef9d25263858711337cf8dc31804f196bdb7840"]
				.unchecked_into(),
			// cXfwFY2GaMueyyaSrjMRSQM48mLU7Z5JRmdB4FJE7weicJ7pu
			hex!["2a66038471e6a62a2df2195efef9d25263858711337cf8dc31804f196bdb7840"]
				.unchecked_into(),
			// cXfwFY2GaMueyyaSrjMRSQM48mLU7Z5JRmdB4FJE7weicJ7pu
			hex!["2a66038471e6a62a2df2195efef9d25263858711337cf8dc31804f196bdb7840"]
				.unchecked_into(),
		),
	];

	// generated with secret: subkey inspect "$secret"/fir
	let root_key: AccountId = hex![
		// cXffK7BmstE5rXcK8pxKLufkffp9iASMntxUm6ctpR6xS3icV
		"1e3e1c69dfbd27d398e92da4844a9abdc2786ac01588d87a2e1f5ec06ea2a936"
	]
	.into();

	let endowed_accounts: Vec<AccountId> = vec![
		root_key.clone(),
		hex!["1ec940be673d3613e94c4d44e3f4621422c1a0778a53a34b2b45f3118f823c03"].into(),
		hex!["92389e795eef50e9af11c997f82d7bb809b0bb9a76f96b6fb29bd60b7a2cbef7"].into(),
		hex!["2a4d86b50b2d98c3bfb02c00f9731753b01f8151774544f4e78e11ef4bb1eb79"].into(),
		hex!["928f11288d2ba2a6c625fbf34366ea533566e6f6b85661d2ea6df4fb6158b14b"].into(),
		hex!["9422fe68db986721051c6cd9841a6afc2c3edc6de3cd74c7b0e45f50b3d6c671"].into(),
		hex!["70186d841754d9f4225f922976682d580e58fce0cfd07696cb6d4324b1ede379"].into(),
		hex!["2a20b3a025722789f18ca7e459ec21d4f232b1a1d245272f14248d3cfa8b412a"].into(),
		hex!["a0749440bc4322cc700bc3678d986f50d2fe91d9322a4a1b3046c3f1bd944940"].into(),
		hex!["b4ac8b37a20935b0b390c61cd849ea533b060bb97e32b68801ec3b6220a8e131"].into(),
		hex!["c4060f93e5bb97ed21491e90ff78268caa67ec84a5d3663ffd65332509fe0dd9"].into(),
		hex!["2a66038471e6a62a2df2195efef9d25263858711337cf8dc31804f196bdb7840"].into(),
	];

	testnet_genesis(initial_authorities, vec![], root_key, Some(endowed_accounts))
}

pub fn cess_testnet_config() -> ChainSpec {
	ChainSpec::from_json_bytes(&include_bytes!("../ccg/cess-testnet-spec-raw.json")[..]).unwrap()
}

pub fn cess_testnet_generate_config() -> ChainSpec {
	let boot_nodes = vec![];
	ChainSpec::from_genesis(
		"cess-testnet",
		"cess-testnet",
		ChainType::Live,
		cess_testnet_config_genesis,
		boot_nodes,
		None,
		Some("TCESS"),
		None,
		Some(
			serde_json::from_str(
				"{\"tokenDecimals\": 12, \"tokenSymbol\": \"TCESS\", \"SS58Prefix\": 11330}",
			)
			.expect("Provided valid json map"),
		),
		Default::default(),
	)
}

fn development_config_genesis() -> GenesisConfig {
	testnet_genesis(
		vec![authority_keys_from_seed("Alice")],
		vec![],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		None,
	)
}

pub fn development_config() -> ChainSpec {
	ChainSpec::from_genesis(
		// Name
		"Development",
		// ID
		"dev",
		ChainType::Development,
		development_config_genesis,
		// Bootnodes
		vec![],
		// Telemetry
		None,
		// Protocol ID
		None,
		// Properties
		None,

		None,
		// Extensions
		Default::default(),
	)
}


fn local_testnet_genesis() -> GenesisConfig {
	testnet_genesis(
		vec![authority_keys_from_seed("Alice"), authority_keys_from_seed("Bob")],
		vec![],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		None,
	)
}

pub fn local_testnet_config() -> ChainSpec {
	ChainSpec::from_genesis(
		// Name
		"Local Testnet",
		// ID
		"local_testnet",
		ChainType::Local,
		local_testnet_genesis,
		// Bootnodes
		vec![],
		// Telemetry
		None,
		// Protocol ID
		None,
		// Properties
		None,

		None,
		// Extensions
		Default::default(),
	)
}

/// Configure initial storage state for FRAME modules.
fn testnet_genesis(
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

	let mut endowed_accounts: Vec<AccountId> = endowed_accounts.unwrap_or_else(|| {
		vec![
			get_account_id_from_seed::<sr25519::Public>("Alice"),
			get_account_id_from_seed::<sr25519::Public>("Bob"),
			get_account_id_from_seed::<sr25519::Public>("Charlie"),
			get_account_id_from_seed::<sr25519::Public>("Dave"),
			get_account_id_from_seed::<sr25519::Public>("Eve"),
			get_account_id_from_seed::<sr25519::Public>("Ferdie"),
			get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
			get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
			get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
			get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
			get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
			get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
		]
	});

	// endow all authorities and nominators.
	initial_authorities
		.iter()
		.map(|x| &x.0)
		.chain(initial_nominators.iter())
		.for_each(|x| {
			if !endowed_accounts.contains(&x) {
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
			let limit = (MaxNominations::get() as usize).min(initial_authorities.len());
			let count = rng.gen::<usize>() % limit;
			let nominations = initial_authorities
				.as_slice()
				.choose_multiple(&mut rng, count)
				.into_iter()
				.map(|choice| choice.0.clone())
				.collect::<Vec<_>>();
			(x.clone(), x.clone(), STASH, StakerStatus::Nominator(nominations))
		}))
		.collect::<Vec<_>>();

	let num_endowed_accounts = endowed_accounts.len();

	const ENDOWMENT: Balance = 10_000_000 * DOLLARS;
	const STASH: Balance = ENDOWMENT / 1000;

	GenesisConfig {
		system: SystemConfig { code: wasm_binary_unwrap().to_vec() },
		balances: BalancesConfig {
			// Configure endowed accounts with initial balance of 1 << 60.
			balances: endowed_accounts.iter().cloned().map(|k| (k, 1 << 65)).collect(),
		},
		indices: IndicesConfig { indices: vec![] },
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
			validator_count: initial_authorities.len() as u32,
			minimum_validator_count: 1,
			invulnerables: initial_authorities.iter().map(|x| x.0.clone()).collect(),
			slash_reward_fraction: Perbill::from_percent(10),
			stakers,
			..Default::default()
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
		babe: BabeConfig {
			authorities: vec![],
			epoch_config: Some(cess_node_runtime::BABE_GENESIS_EPOCH_CONFIG),
		},	
		im_online: ImOnlineConfig { keys: vec![] },
		authority_discovery: AuthorityDiscoveryConfig { keys: vec![] },
		grandpa: GrandpaConfig { authorities: vec![] },
		technical_membership: Default::default(),
		treasury: Default::default(),
		sudo: SudoConfig {
			// Assign network admin rights.
			key: Some(root_key),
		},
		assets: Default::default(),
		transaction_payment: Default::default(),
	}
}
