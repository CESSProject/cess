use cess_node_primitives::{AccountId, Balance, Block, Signature};
use cess_node_runtime::{
	constants::currency::DOLLARS, wasm_binary_unwrap, MaxNominations, SS58Prefix, SessionKeys, StakerStatus,
};
use pallet_audit::sr25519::AuthorityId as SegmentBookId;
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use polkadot_sdk::*;
use sc_chain_spec::{ChainSpecExtension, Properties};
use sc_service::ChainType;
use sc_telemetry::TelemetryEndpoints;
use serde::{Deserialize, Serialize};
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_consensus_babe::AuthorityId as BabeId;
use sp_consensus_beefy::ecdsa_crypto::AuthorityId as BeefyId;
use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sp_core::{crypto::UncheckedInto, sr25519, Pair, Public};
use sp_runtime::{
	traits::{IdentifyAccount, Verify},
	Perbill,
};

// The URL for the telemetry server.
const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

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
pub type ChainSpec = sc_service::GenericChainSpec<Extensions>;

type AccountPublic = <Signature as Verify>::Signer;
#[rustfmt::skip]
type AuthorityKeys = (
	AccountId,
	AccountId,
	GrandpaId,
	BabeId,
	ImOnlineId,
	AuthorityDiscoveryId,
	SegmentBookId,
	BeefyId,
);

fn session_keys(
	grandpa: GrandpaId,
	babe: BabeId,
	im_online: ImOnlineId,
	authority_discovery: AuthorityDiscoveryId,
	beefy: BeefyId,
) -> SessionKeys {
	SessionKeys { grandpa, babe, im_online, authority_discovery, beefy }
}

/// Generate an account ID from seed.
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Helper function to generate stash, controller and session key from seed
pub fn authority_keys_from_seed(seed: &str) -> AuthorityKeys {
	(
		get_account_id_from_seed::<sr25519::Public>(&format!("{}//stash", seed)),
		get_account_id_from_seed::<sr25519::Public>(seed),
		get_from_seed::<GrandpaId>(seed),
		get_from_seed::<BabeId>(seed),
		get_from_seed::<ImOnlineId>(seed),
		get_from_seed::<AuthorityDiscoveryId>(seed),
		get_from_seed::<SegmentBookId>(seed),
		get_from_seed::<BeefyId>(seed),
	)
}

fn properties() -> Properties {
	let mut properties = Properties::new();
	properties.insert("tokenSymbol".into(), "TCESS".into());
	properties.insert("tokenDecimals".into(), 18.into());
	properties.insert("ss58Format".into(), SS58Prefix::get().into());
	properties
}

fn cess_testnet_genesis() -> serde_json::Value {
	let initial_authorities: Vec<AuthorityKeys> = vec![
		(
			// cXfg2SYcq85nyZ1U4ccx6QnAgSeLQB8aXZ2jstbw9CPGSmhXY
			array_bytes::hex_n_into_unchecked("1ec940be673d3613e94c4d44e3f4621422c1a0778a53a34b2b45f3118f823c03"),
			// cXffK7BmstE5rXcK8pxKLufkffp9iASMntxUm6ctpR6xS3icV
			array_bytes::hex_n_into_unchecked("1e3e1c69dfbd27d398e92da4844a9abdc2786ac01588d87a2e1f5ec06ea2a936"),
			// cXiHP2jLVk4iUrJdQVjBc2txoKjHANpomxcfHeCkKcxpesfDy
			array_bytes::hex2array_unchecked("92389e795eef50e9af11c997f82d7bb809b0bb9a76f96b6fb29bd60b7a2cbef7")
				.unchecked_into(),
			// cXfw8G7ThY48EqDMnQdZbwjmu69yELy3dMcuWyacG9snCMdPx
			array_bytes::hex2array_unchecked("2a4d86b50b2d98c3bfb02c00f9731753b01f8151774544f4e78e11ef4bb1eb79")
				.unchecked_into(),
			// cXfw8G7ThY48EqDMnQdZbwjmu69yELy3dMcuWyacG9snCMdPx
			array_bytes::hex2array_unchecked("2a4d86b50b2d98c3bfb02c00f9731753b01f8151774544f4e78e11ef4bb1eb79")
				.unchecked_into(),
			// cXfw8G7ThY48EqDMnQdZbwjmu69yELy3dMcuWyacG9snCMdPx
			array_bytes::hex2array_unchecked("2a4d86b50b2d98c3bfb02c00f9731753b01f8151774544f4e78e11ef4bb1eb79")
				.unchecked_into(),
			array_bytes::hex2array_unchecked("2a4d86b50b2d98c3bfb02c00f9731753b01f8151774544f4e78e11ef4bb1eb79")
				.unchecked_into(),
			array_bytes::hex2array_unchecked("035cea616c14eef7250f8e08a8609b49acdefb2c8dd56ac92f83475b520f4bc673")
				.unchecked_into(),
		),
		(
			// cXiHpiCFn6xkypa2Q8SroghuME1hBtDDrJYEEAXLwkfy6y277
			array_bytes::hex_n_into_unchecked("928f11288d2ba2a6c625fbf34366ea533566e6f6b85661d2ea6df4fb6158b14b"),
			// cXiKthh2dyY1taTydtdxiqQwXY1HKZcXvYGmjS2UmuPi2qNDS
			array_bytes::hex_n_into_unchecked("9422fe68db986721051c6cd9841a6afc2c3edc6de3cd74c7b0e45f50b3d6c671"),
			// cXhWdqfECn1yMd9E6eKT6MD3NqyWGve8F1gLAvM9CAH4RH6LR
			array_bytes::hex2array_unchecked("70186d841754d9f4225f922976682d580e58fce0cfd07696cb6d4324b1ede379")
				.unchecked_into(),
			// cXfvtwoYtMD1Z8nZrK5JptFSdBAVyjQ1t2i6scvMLYK75ChrP
			array_bytes::hex2array_unchecked("2a20b3a025722789f18ca7e459ec21d4f232b1a1d245272f14248d3cfa8b412a")
				.unchecked_into(),
			// cXfvtwoYtMD1Z8nZrK5JptFSdBAVyjQ1t2i6scvMLYK75ChrP
			array_bytes::hex2array_unchecked("2a20b3a025722789f18ca7e459ec21d4f232b1a1d245272f14248d3cfa8b412a")
				.unchecked_into(),
			// cXfvtwoYtMD1Z8nZrK5JptFSdBAVyjQ1t2i6scvMLYK75ChrP
			array_bytes::hex2array_unchecked("2a20b3a025722789f18ca7e459ec21d4f232b1a1d245272f14248d3cfa8b412a")
				.unchecked_into(),
			array_bytes::hex2array_unchecked("2a20b3a025722789f18ca7e459ec21d4f232b1a1d245272f14248d3cfa8b412a")
				.unchecked_into(),
			array_bytes::hex2array_unchecked("026cf46275ef5df7d320cc5c959fec34636a9b8ba51334d54b6c19380e0643dc31")
				.unchecked_into(),
		),
		(
			// cXic3WhctsJ9cExmjE9vog49xaLuVbDLcFi2odeEnvV5Sbq4f
			array_bytes::hex_n_into_unchecked("a0749440bc4322cc700bc3678d986f50d2fe91d9322a4a1b3046c3f1bd944940"),
			// cXj4Z6VmYHkD64xEadQCNgFUDaV4F4ibeupuU4ELJa8uWH7mw
			array_bytes::hex_n_into_unchecked("b4ac8b37a20935b0b390c61cd849ea533b060bb97e32b68801ec3b6220a8e131"),
			// cXjQgQVu9Zuitzkb8oiTYTKt7QUm9jupKbzqVNVwPELwz26rT
			array_bytes::hex2array_unchecked("c4060f93e5bb97ed21491e90ff78268caa67ec84a5d3663ffd65332509fe0dd9")
				.unchecked_into(),
			// cXfwFY2GaMueyyaSrjMRSQM48mLU7Z5JRmdB4FJE7weicJ7pu
			array_bytes::hex2array_unchecked("2a66038471e6a62a2df2195efef9d25263858711337cf8dc31804f196bdb7840")
				.unchecked_into(),
			// cXfwFY2GaMueyyaSrjMRSQM48mLU7Z5JRmdB4FJE7weicJ7pu
			array_bytes::hex2array_unchecked("2a66038471e6a62a2df2195efef9d25263858711337cf8dc31804f196bdb7840")
				.unchecked_into(),
			// cXfwFY2GaMueyyaSrjMRSQM48mLU7Z5JRmdB4FJE7weicJ7pu
			array_bytes::hex2array_unchecked("2a66038471e6a62a2df2195efef9d25263858711337cf8dc31804f196bdb7840")
				.unchecked_into(),
			//cXfwFY2GaMueyyaSrjMRSQM48mLU7Z5JRmdB4FJE7weicJ7pu
			array_bytes::hex2array_unchecked("2a66038471e6a62a2df2195efef9d25263858711337cf8dc31804f196bdb7840")
				.unchecked_into(),
			array_bytes::hex2array_unchecked("034455f01992d48e2e28ac521930a03865324447ce5ba1e8331a42d17315a27f22")
				.unchecked_into(),
		),
		(
			// cXik7GNf8qYgt6TtGajELHN8QRjd9iy4pd6soPnjcccsenSuh
			array_bytes::hex_n_into_unchecked("a69b2d0f8bb3f43f152c55e8cb1a605f927776feaa3cb08ead51de0ef3a3f34c"),
			// cXg2eDnb8ngoDw5nvviQAdyAkCr9wzYUsnb6bbqSqeLV8rmsT
			array_bytes::hex_n_into_unchecked("2e829b75650eac0d4c8dcfb8aa724d2df70dc5a44b16a8800b87403c3a4d8959"),
			// cXhcCoc4Ko1MPGa9yJY7XHYGusoqA6nB6JBTsadVfYZYBR96L
			array_bytes::hex2array_unchecked("74579f9baed2d7151cb0f5adfe0084f59b4bbf8696d5aa0493f584097bfe3cb1")
				.unchecked_into(),
			// cXgqJbnKuaHXi6ssKaLRBK7BsHMYyqnQkhiPuhjjSF1TGZWsF
			array_bytes::hex2array_unchecked("521917850191d8787c10d9e35a0f3ff218e992e4ed476e5c33f7de5ab04f1a38")
				.unchecked_into(),
			// cXgqJbnKuaHXi6ssKaLRBK7BsHMYyqnQkhiPuhjjSF1TGZWsF
			array_bytes::hex2array_unchecked("521917850191d8787c10d9e35a0f3ff218e992e4ed476e5c33f7de5ab04f1a38")
				.unchecked_into(),
			// cXgqJbnKuaHXi6ssKaLRBK7BsHMYyqnQkhiPuhjjSF1TGZWsF
			array_bytes::hex2array_unchecked("521917850191d8787c10d9e35a0f3ff218e992e4ed476e5c33f7de5ab04f1a38")
				.unchecked_into(),
			// cXgqJbnKuaHXi6ssKaLRBK7BsHMYyqnQkhiPuhjjSF1TGZWsF
			array_bytes::hex2array_unchecked("521917850191d8787c10d9e35a0f3ff218e992e4ed476e5c33f7de5ab04f1a38")
				.unchecked_into(),
			array_bytes::hex2array_unchecked("02dcbd5374c49be753cfd8935b9efa0d42eeff7235c3f4a552b7877ef9a50fcbb9")
				.unchecked_into(),
		),
		(
			array_bytes::hex_n_into_unchecked("3aa51ef77986966c8aa4c801229e3d0210500bc92be5c7c6542fc540826dad7e"),
			array_bytes::hex_n_into_unchecked("4cd8cdd8684aa21c647cc62207815ff9f8414ee72e8068be3db4e19ae34bde7f"),
			array_bytes::hex2array_unchecked("2692ccff99f336e6291c5f01ca19b0ddd53662adafd07a94ce27d50db888a838")
				.unchecked_into(),
			array_bytes::hex2array_unchecked("fa93c04ccad2297b8f231291d46599fe9ef54db4c2b41f2a5fdd469adf5dfd18")
				.unchecked_into(),
			array_bytes::hex2array_unchecked("fa93c04ccad2297b8f231291d46599fe9ef54db4c2b41f2a5fdd469adf5dfd18")
				.unchecked_into(),
			array_bytes::hex2array_unchecked("fa93c04ccad2297b8f231291d46599fe9ef54db4c2b41f2a5fdd469adf5dfd18")
				.unchecked_into(),
			array_bytes::hex2array_unchecked("fa93c04ccad2297b8f231291d46599fe9ef54db4c2b41f2a5fdd469adf5dfd18")
				.unchecked_into(),
			array_bytes::hex2array_unchecked("038ac7eac0aa202a33797544dec561b3837fe2bbedb8b1553bfbf08184e0cb9b5d")
				.unchecked_into(),
		),
	];

	// generated with secret: subkey inspect "$secret"/fir
	let root_key: AccountId = array_bytes::hex_n_into_unchecked(
		// cXjubUUW33cMzWtA8XLsmZMLMU6yVKzoJzYtGPEjPBdbLacat
		"da1393ab827176a08c6206fd92f82b84ccbac41681f0025b7e8a679ebbea212e",
	);

	let endowed_accounts: Vec<AccountId> = vec![
		root_key.clone(),
		array_bytes::hex_n_into_unchecked("1ec940be673d3613e94c4d44e3f4621422c1a0778a53a34b2b45f3118f823c03"),
		array_bytes::hex_n_into_unchecked("92389e795eef50e9af11c997f82d7bb809b0bb9a76f96b6fb29bd60b7a2cbef7"),
		array_bytes::hex_n_into_unchecked("2a4d86b50b2d98c3bfb02c00f9731753b01f8151774544f4e78e11ef4bb1eb79"),
		array_bytes::hex_n_into_unchecked("928f11288d2ba2a6c625fbf34366ea533566e6f6b85661d2ea6df4fb6158b14b"),
		array_bytes::hex_n_into_unchecked("9422fe68db986721051c6cd9841a6afc2c3edc6de3cd74c7b0e45f50b3d6c671"),
		array_bytes::hex_n_into_unchecked("70186d841754d9f4225f922976682d580e58fce0cfd07696cb6d4324b1ede379"),
		array_bytes::hex_n_into_unchecked("2a20b3a025722789f18ca7e459ec21d4f232b1a1d245272f14248d3cfa8b412a"),
		array_bytes::hex_n_into_unchecked("a0749440bc4322cc700bc3678d986f50d2fe91d9322a4a1b3046c3f1bd944940"),
		array_bytes::hex_n_into_unchecked("b4ac8b37a20935b0b390c61cd849ea533b060bb97e32b68801ec3b6220a8e131"),
		array_bytes::hex_n_into_unchecked("c4060f93e5bb97ed21491e90ff78268caa67ec84a5d3663ffd65332509fe0dd9"),
		array_bytes::hex_n_into_unchecked("2a66038471e6a62a2df2195efef9d25263858711337cf8dc31804f196bdb7840"),
		array_bytes::hex_n_into_unchecked("2ed4a2c67291bf3eaa4de538ab120ba21b302db5704551864226d2fae8f87937"),
		array_bytes::hex_n_into_unchecked("d0a9eef85d7762e89280b8fdfd4ce031530b95421214fcc28c554dbb4d9fe927"),
		array_bytes::hex_n_into_unchecked("5ce2722592557b41c2359fec3367f782703706784f193abc735b937abae71e30"),
		array_bytes::hex_n_into_unchecked("a69b2d0f8bb3f43f152c55e8cb1a605f927776feaa3cb08ead51de0ef3a3f34c"),
		array_bytes::hex_n_into_unchecked("2e829b75650eac0d4c8dcfb8aa724d2df70dc5a44b16a8800b87403c3a4d8959"),
		array_bytes::hex_n_into_unchecked("74579f9baed2d7151cb0f5adfe0084f59b4bbf8696d5aa0493f584097bfe3cb1"),
		array_bytes::hex_n_into_unchecked("521917850191d8787c10d9e35a0f3ff218e992e4ed476e5c33f7de5ab04f1a38"),
		array_bytes::hex_n_into_unchecked("3aa51ef77986966c8aa4c801229e3d0210500bc92be5c7c6542fc540826dad7e"),
		array_bytes::hex_n_into_unchecked("4cd8cdd8684aa21c647cc62207815ff9f8414ee72e8068be3db4e19ae34bde7f"),
		array_bytes::hex_n_into_unchecked("2692ccff99f336e6291c5f01ca19b0ddd53662adafd07a94ce27d50db888a838"),
		array_bytes::hex_n_into_unchecked("fa93c04ccad2297b8f231291d46599fe9ef54db4c2b41f2a5fdd469adf5dfd18"),
	];

	testnet_genesis(initial_authorities, vec![], root_key, Some(endowed_accounts))
}

fn cess_testnet_config_genesis() -> serde_json::Value {
	#[rustfmt::skip]
	// stash, controller, session-key
	// generated with secret:
	// for i in 1 2 3 4 ; do for j in stash controller; do subkey inspect "$secret"/fir/$j/$i; done; done
	//
	// and
	//
	// for i in 1 2 3 4 ; do for j in session; do subkey --ed25519 inspect "$secret"//fir//$j//$i; done; done

	let initial_authorities: Vec<AuthorityKeys> = vec![
		(
			// cXfg2SYcq85nyZ1U4ccx6QnAgSeLQB8aXZ2jstbw9CPGSmhXY
			array_bytes::hex_n_into_unchecked("1ec940be673d3613e94c4d44e3f4621422c1a0778a53a34b2b45f3118f823c03"),
			// cXffK7BmstE5rXcK8pxKLufkffp9iASMntxUm6ctpR6xS3icV
			array_bytes::hex_n_into_unchecked("1e3e1c69dfbd27d398e92da4844a9abdc2786ac01588d87a2e1f5ec06ea2a936"),
			// cXiHP2jLVk4iUrJdQVjBc2txoKjHANpomxcfHeCkKcxpesfDy
			array_bytes::hex2array_unchecked("92389e795eef50e9af11c997f82d7bb809b0bb9a76f96b6fb29bd60b7a2cbef7")
				.unchecked_into(),
			// cXfw8G7ThY48EqDMnQdZbwjmu69yELy3dMcuWyacG9snCMdPx
			array_bytes::hex2array_unchecked("2a4d86b50b2d98c3bfb02c00f9731753b01f8151774544f4e78e11ef4bb1eb79")
				.unchecked_into(),
			// cXfw8G7ThY48EqDMnQdZbwjmu69yELy3dMcuWyacG9snCMdPx
			array_bytes::hex2array_unchecked("2a4d86b50b2d98c3bfb02c00f9731753b01f8151774544f4e78e11ef4bb1eb79")
				.unchecked_into(),
			// cXfw8G7ThY48EqDMnQdZbwjmu69yELy3dMcuWyacG9snCMdPx
			array_bytes::hex2array_unchecked("2a4d86b50b2d98c3bfb02c00f9731753b01f8151774544f4e78e11ef4bb1eb79")
				.unchecked_into(),
			array_bytes::hex2array_unchecked("2a4d86b50b2d98c3bfb02c00f9731753b01f8151774544f4e78e11ef4bb1eb79")
				.unchecked_into(),
			array_bytes::hex2array_unchecked("035cea616c14eef7250f8e08a8609b49acdefb2c8dd56ac92f83475b520f4bc673")
				.unchecked_into(),
		),
		(
			// cXiHpiCFn6xkypa2Q8SroghuME1hBtDDrJYEEAXLwkfy6y277
			array_bytes::hex_n_into_unchecked("928f11288d2ba2a6c625fbf34366ea533566e6f6b85661d2ea6df4fb6158b14b"),
			// cXiKthh2dyY1taTydtdxiqQwXY1HKZcXvYGmjS2UmuPi2qNDS
			array_bytes::hex_n_into_unchecked("9422fe68db986721051c6cd9841a6afc2c3edc6de3cd74c7b0e45f50b3d6c671"),
			// cXhWdqfECn1yMd9E6eKT6MD3NqyWGve8F1gLAvM9CAH4RH6LR
			array_bytes::hex2array_unchecked("70186d841754d9f4225f922976682d580e58fce0cfd07696cb6d4324b1ede379")
				.unchecked_into(),
			// cXfvtwoYtMD1Z8nZrK5JptFSdBAVyjQ1t2i6scvMLYK75ChrP
			array_bytes::hex2array_unchecked("2a20b3a025722789f18ca7e459ec21d4f232b1a1d245272f14248d3cfa8b412a")
				.unchecked_into(),
			// cXfvtwoYtMD1Z8nZrK5JptFSdBAVyjQ1t2i6scvMLYK75ChrP
			array_bytes::hex2array_unchecked("2a20b3a025722789f18ca7e459ec21d4f232b1a1d245272f14248d3cfa8b412a")
				.unchecked_into(),
			// cXfvtwoYtMD1Z8nZrK5JptFSdBAVyjQ1t2i6scvMLYK75ChrP
			array_bytes::hex2array_unchecked("2a20b3a025722789f18ca7e459ec21d4f232b1a1d245272f14248d3cfa8b412a")
				.unchecked_into(),
			array_bytes::hex2array_unchecked("2a20b3a025722789f18ca7e459ec21d4f232b1a1d245272f14248d3cfa8b412a")
				.unchecked_into(),
			array_bytes::hex2array_unchecked("026cf46275ef5df7d320cc5c959fec34636a9b8ba51334d54b6c19380e0643dc31")
				.unchecked_into(),
		),
		(
			// cXic3WhctsJ9cExmjE9vog49xaLuVbDLcFi2odeEnvV5Sbq4f
			array_bytes::hex_n_into_unchecked("a0749440bc4322cc700bc3678d986f50d2fe91d9322a4a1b3046c3f1bd944940"),
			// cXj4Z6VmYHkD64xEadQCNgFUDaV4F4ibeupuU4ELJa8uWH7mw
			array_bytes::hex_n_into_unchecked("b4ac8b37a20935b0b390c61cd849ea533b060bb97e32b68801ec3b6220a8e131"),
			// cXjQgQVu9Zuitzkb8oiTYTKt7QUm9jupKbzqVNVwPELwz26rT
			array_bytes::hex2array_unchecked("c4060f93e5bb97ed21491e90ff78268caa67ec84a5d3663ffd65332509fe0dd9")
				.unchecked_into(),
			// cXfwFY2GaMueyyaSrjMRSQM48mLU7Z5JRmdB4FJE7weicJ7pu
			array_bytes::hex2array_unchecked("2a66038471e6a62a2df2195efef9d25263858711337cf8dc31804f196bdb7840")
				.unchecked_into(),
			// cXfwFY2GaMueyyaSrjMRSQM48mLU7Z5JRmdB4FJE7weicJ7pu
			array_bytes::hex2array_unchecked("2a66038471e6a62a2df2195efef9d25263858711337cf8dc31804f196bdb7840")
				.unchecked_into(),
			// cXfwFY2GaMueyyaSrjMRSQM48mLU7Z5JRmdB4FJE7weicJ7pu
			array_bytes::hex2array_unchecked("2a66038471e6a62a2df2195efef9d25263858711337cf8dc31804f196bdb7840")
				.unchecked_into(),
			array_bytes::hex2array_unchecked("2a66038471e6a62a2df2195efef9d25263858711337cf8dc31804f196bdb7840")
				.unchecked_into(),
			array_bytes::hex2array_unchecked("034455f01992d48e2e28ac521930a03865324447ce5ba1e8331a42d17315a27f22")
				.unchecked_into(),
		),
	];

	// generated with secret: subkey inspect "$secret"/fir
	let root_key: AccountId = array_bytes::hex_n_into_unchecked(
		// cXjubUUW33cMzWtA8XLsmZMLMU6yVKzoJzYtGPEjPBdbLacat
		"da1393ab827176a08c6206fd92f82b84ccbac41681f0025b7e8a679ebbea212e",
	);

	let endowed_accounts: Vec<AccountId> = vec![
		root_key.clone(),
		array_bytes::hex_n_into_unchecked("1ec940be673d3613e94c4d44e3f4621422c1a0778a53a34b2b45f3118f823c03"),
		array_bytes::hex_n_into_unchecked("92389e795eef50e9af11c997f82d7bb809b0bb9a76f96b6fb29bd60b7a2cbef7"),
		array_bytes::hex_n_into_unchecked("2a4d86b50b2d98c3bfb02c00f9731753b01f8151774544f4e78e11ef4bb1eb79"),
		array_bytes::hex_n_into_unchecked("928f11288d2ba2a6c625fbf34366ea533566e6f6b85661d2ea6df4fb6158b14b"),
		array_bytes::hex_n_into_unchecked("9422fe68db986721051c6cd9841a6afc2c3edc6de3cd74c7b0e45f50b3d6c671"),
		array_bytes::hex_n_into_unchecked("70186d841754d9f4225f922976682d580e58fce0cfd07696cb6d4324b1ede379"),
		array_bytes::hex_n_into_unchecked("2a20b3a025722789f18ca7e459ec21d4f232b1a1d245272f14248d3cfa8b412a"),
		array_bytes::hex_n_into_unchecked("a0749440bc4322cc700bc3678d986f50d2fe91d9322a4a1b3046c3f1bd944940"),
		array_bytes::hex_n_into_unchecked("b4ac8b37a20935b0b390c61cd849ea533b060bb97e32b68801ec3b6220a8e131"),
		array_bytes::hex_n_into_unchecked("c4060f93e5bb97ed21491e90ff78268caa67ec84a5d3663ffd65332509fe0dd9"),
		array_bytes::hex_n_into_unchecked("2a66038471e6a62a2df2195efef9d25263858711337cf8dc31804f196bdb7840"),
		array_bytes::hex_n_into_unchecked("2ed4a2c67291bf3eaa4de538ab120ba21b302db5704551864226d2fae8f87937"),
		array_bytes::hex_n_into_unchecked("d0a9eef85d7762e89280b8fdfd4ce031530b95421214fcc28c554dbb4d9fe927"),
		array_bytes::hex_n_into_unchecked("5ce2722592557b41c2359fec3367f782703706784f193abc735b937abae71e30"),
	];

	testnet_genesis(initial_authorities, vec![], root_key, Some(endowed_accounts))
}

pub fn cess_testnet_config() -> ChainSpec {
	ChainSpec::from_json_bytes(&include_bytes!("../ccg/cess-testnet-spec-raw.json")[..]).unwrap()
}

pub fn cess_devnet_config() -> ChainSpec {
	ChainSpec::from_json_bytes(&include_bytes!("../ccg/cess-develop-spec-raw.json")[..]).unwrap()
}

pub fn cess_devnet_generate_config() -> ChainSpec {
	ChainSpec::builder(wasm_binary_unwrap(), Default::default())
		.with_name("cess-devnet")
		.with_id("cess-devnet")
		.with_protocol_id("cesdn0")
		.with_chain_type(ChainType::Live)
		.with_properties(properties())
		.with_genesis_config_patch(cess_testnet_config_genesis())
		.with_telemetry_endpoints(
			TelemetryEndpoints::new(vec![(STAGING_TELEMETRY_URL.to_string(), 0)])
				.expect("Staging telemetry url is valid; qed"),
		)
		.build()
}

pub fn cess_testnet() -> ChainSpec {
	ChainSpec::builder(wasm_binary_unwrap(), Default::default())
		.with_name("cess-testnet")
		.with_id("cess-testnet")
		.with_protocol_id("cestn1")
		.with_chain_type(ChainType::Live)
		.with_properties(properties())
		.with_genesis_config_patch(cess_testnet_genesis())
		.with_telemetry_endpoints(
			TelemetryEndpoints::new(vec![(STAGING_TELEMETRY_URL.to_string(), 0)])
				.expect("Staging telemetry url is valid; qed"),
		)
		.build()
}

pub fn development_config() -> ChainSpec {
	ChainSpec::builder(wasm_binary_unwrap(), Default::default())
		.with_name("Development")
		.with_id("dev")
		.with_chain_type(ChainType::Development)
		.with_properties(properties())
		.with_genesis_config_patch(testnet_genesis(
			vec![authority_keys_from_seed("Alice")],
			vec![],
			get_account_id_from_seed::<sr25519::Public>("Alice"),
			None,
		))
		.build()
}

pub fn local_testnet_config() -> ChainSpec {
	ChainSpec::builder(wasm_binary_unwrap(), Default::default())
		.with_name("Local Testnet")
		.with_id("local_testnet")
		.with_chain_type(ChainType::Local)
		.with_properties(properties())
		.with_genesis_config_patch(testnet_genesis(
			vec![authority_keys_from_seed("Alice"), authority_keys_from_seed("Bob")],
			vec![],
			get_account_id_from_seed::<sr25519::Public>("Alice"),
			None,
		))
		.build()
}

/// Configure initial storage state for FRAME modules.
fn testnet_genesis(
	initial_authorities: Vec<AuthorityKeys>,
	initial_nominators: Vec<AccountId>,
	root_key: AccountId,
	endowed_accounts: Option<Vec<AccountId>>,
) -> serde_json::Value {
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
			if !endowed_accounts.contains(x) {
				endowed_accounts.push(x.clone())
			}
		});

	// stakers: all validators and nominators.
	let stakers = initial_authorities
		.iter()
		.map(|x| (x.0.clone(), x.1.clone(), STASH, StakerStatus::Validator))
		.chain(initial_nominators.iter().map(|x| {			
			let nominations = initial_authorities.iter().take(MaxNominations::get() as usize).cloned().collect::<Vec<_>>();
			(x.clone(), x.clone(), STASH, StakerStatus::Nominator(nominations))
		}))
		.collect::<Vec<_>>();

	let num_endowed_accounts = endowed_accounts.len();
	let evm_chain_id = SS58Prefix::get();

	const ENDOWMENT: Balance = 100_000_000 * DOLLARS;
	const STASH: Balance = 3_000_000 * DOLLARS;

	serde_json::json!({
		"sudo": { "key": Some(root_key) },
		"balances": {
			"balances": endowed_accounts
				.iter()
				.cloned()
				.map(|k| (k, ENDOWMENT))
				.collect::<Vec<_>>()
		},
		"session": {
			"keys": initial_authorities
				.iter()
				.map(|x| (x.0.clone(), x.0.clone(), session_keys(x.2.clone(), x.3.clone(), x.4.clone(), x.5.clone(), x.7.clone())))
				.collect::<Vec<_>>()
		},
		"staking": {
			"validatorCount": initial_authorities.len(),
			"minimumValidatorCount": 1,
			"invulnerables": initial_authorities.iter().map(|x| x.0.clone()).collect::<Vec<_>>(),
			"slashRewardFraction": Perbill::from_percent(10),
			"stakers": stakers,
		},
		"technicalCommittee": {
			"members": endowed_accounts.iter().take((num_endowed_accounts + 1) / 2).cloned().collect::<Vec<_>>()
		},
		"babe": {
			"epochConfig": Some(cess_node_runtime::BABE_GENESIS_EPOCH_CONFIG)
		},
		"evmChainId": {
			"chainId": evm_chain_id
		},
		"storageHandler": {
			"price": 30 * DOLLARS
		}
	})
}
