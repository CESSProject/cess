use subxt::{
    backend::StreamOfResults,
    blocks::Block,
    config::{
        substrate::{BlakeTwo256, SubstrateHeader},
        SubstrateExtrinsicParams, HashFor,
    },
    utils::{AccountId32, MultiAddress, MultiSignature},
    Config,
};

include!(concat!(env!("OUT_DIR"), "/genesis_hash.rs"));

// Generated `runtime` mod
include!(concat!(env!("OUT_DIR"), "/runtime_path.rs"));

pub enum CesRuntimeConfig {}

impl Config for CesRuntimeConfig {
    type AccountId = AccountId32;
    type Address = MultiAddress<Self::AccountId, u32>;
    type Signature = MultiSignature;
    type Hasher = BlakeTwo256;
    type Header = SubstrateHeader<u32, BlakeTwo256>;
    type ExtrinsicParams = SubstrateExtrinsicParams<Self>;
    type AssetId = u32;
}

pub type CesChainClient = subxt::client::OnlineClient<CesRuntimeConfig>;
pub type BlockNumber = u32;
pub type AccountId = <CesRuntimeConfig as subxt::Config>::AccountId;
pub type Hash = HashFor<CesRuntimeConfig>;
pub type CesBlock = Block<CesRuntimeConfig, CesChainClient>;
pub type CesBlockStream = StreamOfResults<CesBlock>;

mod runtime_type_converts {
    use super::{AccountId, runtime};
    
    impl From<ces_types::MasterKeyDistributePayload> for runtime::tee_worker::calls::types::distribute_master_key::Payload {
        fn from(value: ces_types::MasterKeyDistributePayload) -> Self {
            Self {
                distributor: value.distributor.0,
                target: value.target.0,
                ecdh_pubkey: value.ecdh_pubkey.0,
                encrypted_master_key: value.encrypted_master_key,
                iv: value.iv,
                signing_time: value.signing_time,
            }
        }
    }

    impl From<ces_types::MasterKeyApplyPayload> for runtime::tee_worker::calls::types::apply_master_key::Payload {
        fn from(value: ces_types::MasterKeyApplyPayload) -> Self {
            Self { pubkey: value.pubkey.0, ecdh_pubkey: value.ecdh_pubkey.0, signing_time: value.signing_time }
        }
    }

    type WorkerRoleSubxtGen = runtime::runtime_types::ces_types::WorkerRole;

    impl From<ces_types::WorkerRole> for WorkerRoleSubxtGen {
        fn from(value: ces_types::WorkerRole) -> Self {
            match value {
                ces_types::WorkerRole::Full => WorkerRoleSubxtGen::Full,
                ces_types::WorkerRole::Verifier => WorkerRoleSubxtGen::Verifier,
                ces_types::WorkerRole::Marker => WorkerRoleSubxtGen::Marker,
            }
        }
    }

    impl From<ces_types::WorkerRegistrationInfo<AccountId>> for runtime::tee_worker::calls::types::register_worker::CesealInfo {
        fn from(value: ces_types::WorkerRegistrationInfo<AccountId>) -> Self {
            Self {
                version: value.version,
                machine_id: value.machine_id,
                pubkey: value.pubkey.0,
                ecdh_pubkey: value.ecdh_pubkey.0,
                stash_account: value.stash_account,
                genesis_block_hash: value.genesis_block_hash,
                features: value.features,
                role: value.role.into(),
                endpoint: value.endpoint,
            }
        }
    }

    type SgxV30QuoteCollateralSubxtGen = runtime::runtime_types::sgx_attestation::types::SgxV30QuoteCollateral;

    impl From<ces_types::attestation::SgxV30QuoteCollateral> for SgxV30QuoteCollateralSubxtGen {
        fn from(value: ces_types::attestation::SgxV30QuoteCollateral) -> Self {
            Self {
                pck_crl_issuer_chain: value.pck_crl_issuer_chain,
                root_ca_crl: value.root_ca_crl,
                pck_crl: value.pck_crl,
                tcb_info_issuer_chain: value.tcb_info_issuer_chain,
                tcb_info: value.tcb_info,
                tcb_info_signature: value.tcb_info_signature,
                qe_identity_issuer_chain: value.qe_identity_issuer_chain,
                qe_identity: value.qe_identity,
                qe_identity_signature: value.qe_identity_signature,
            }
        }
    }

    type CollateralSubxtGen = runtime::runtime_types::sgx_attestation::types::Collateral;

    impl From<ces_types::attestation::Collateral> for CollateralSubxtGen {
        fn from(value: ces_types::attestation::Collateral) -> Self {
            match value {
                ces_types::attestation::Collateral::SgxV30(v) => CollateralSubxtGen::SgxV30(v.into()),
            }
        }
    }    

    type AttestationReportSubxtGen = runtime::runtime_types::sgx_attestation::types::AttestationReport;

    impl From<ces_types::AttestationReport> for AttestationReportSubxtGen {
        fn from(value: ces_types::AttestationReport) -> Self {
            match value {
                ces_types::AttestationReport::SgxDcap { quote, collateral } => AttestationReportSubxtGen::SgxDcap {
                    quote,
                    collateral: collateral.map(|e| e.into()),
                },
                ces_types::AttestationReport::SgxIas {ra_report, signature, raw_signing_cert} => AttestationReportSubxtGen::SgxIas {
                    ra_report, signature, raw_signing_cert
                }
            }
        }
    }
    
}