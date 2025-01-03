use scale_encode::EncodeAsType;

use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use subxt::config::polkadot::{PolkadotExtrinsicParams, PolkadotExtrinsicParamsBuilder};

mod chain_api;
pub mod dynamic;
pub mod rpc;

#[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, PartialOrd, Ord, Debug, EncodeAsType)]
pub struct ParaId(pub u32);

pub type StorageProof = Vec<Vec<u8>>;
pub type StorageState = Vec<(Vec<u8>, Vec<u8>)>;
pub type ExtrinsicParams = PolkadotExtrinsicParams<Config>;
pub type ExtrinsicParamsBuilder = PolkadotExtrinsicParamsBuilder<Config>;
pub use subxt::PolkadotConfig as Config;
pub type SubxtOnlineClient = subxt::OnlineClient<Config>;

pub use chain_api::{connect, ChainApi};
pub type ParachainApi = ChainApi;
pub type RelaychainApi = ChainApi;

pub use subxt;
pub type BlockNumber = u32;
pub type Hash = primitive_types::H256;
pub type AccountId = <Config as subxt::Config>::AccountId;

pub mod sp_core {
    pub use sp_core::*;
}

pub use pair_signer::PairSigner;

/// A concrete PairSigner implementation which relies on `sr25519::Pair` for signing
/// and that PolkadotConfig is the runtime configuration.
mod pair_signer {
    use sp_core::{sr25519, Pair as _};
    use subxt::{Config, PolkadotConfig};

    use sp_runtime::{
        traits::{IdentifyAccount, Verify},
        MultiSignature as SpMultiSignature,
    };
    use subxt::{
        config::substrate::{AccountId32, MultiSignature},
        tx::Signer,
    };

    /// A [`Signer`] implementation for [`polkadot_sdk::sp_core::sr25519::Pair`].
    #[derive(Clone)]
    pub struct PairSigner {
        account_id: <PolkadotConfig as Config>::AccountId,
        signer: sr25519::Pair,
        nonce: u64,
    }

    impl PairSigner {
        /// Creates a new [`Signer`] from an [`sp_core::sr25519::Pair`].
        pub fn new(signer: sr25519::Pair) -> Self {
            let account_id =
                <SpMultiSignature as Verify>::Signer::from(signer.public()).into_account();
            Self {
                // Convert `sp_core::AccountId32` to `subxt::config::substrate::AccountId32`.
                //
                // This is necessary because we use `subxt::config::substrate::AccountId32` and no
                // From/Into impls are provided between `sp_core::AccountId32` because `polkadot-sdk` isn't a direct
                // dependency in subxt.
                //
                // This can also be done by provided a wrapper type around `subxt::config::substrate::AccountId32` to implement
                // such conversions but that also most likely requires a custom `Config` with a separate `AccountId` type to work
                // properly without additional hacks.
                account_id: AccountId32(account_id.into()),
                signer,
                nonce: 0,
            }
        }

        /// Returns the [`sp_core::sr25519::Pair`] implementation used to construct this.
        pub fn signer(&self) -> &sr25519::Pair {
            &self.signer
        }

        /// Return the account ID.
        pub fn account_id(&self) -> &AccountId32 {
            &self.account_id
        }

        pub fn increment_nonce(&mut self) {
            self.nonce += 1;
        }

        pub fn nonce(&self) -> u64 {
            self.nonce
        }

        pub fn set_nonce(&mut self, nonce: u64) {
            self.nonce = nonce;
        }
    }

    impl Signer<PolkadotConfig> for PairSigner {
        fn account_id(&self) -> <PolkadotConfig as Config>::AccountId {
            self.account_id.clone()
        }

        fn address(&self) -> <PolkadotConfig as Config>::Address {
            self.account_id.clone().into()
        }

        fn sign(&self, signer_payload: &[u8]) -> <PolkadotConfig as Config>::Signature {
            let signature = self.signer.sign(signer_payload);
            MultiSignature::Sr25519(signature.0)
        }
    }
}
