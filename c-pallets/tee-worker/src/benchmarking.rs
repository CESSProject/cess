use super::*;
use crate::{Pallet as TeeWorker, *};
use codec::{alloc::string::ToString, Decode};
pub use frame_benchmarking::{
	account, benchmarks, impl_benchmark_test_suite, whitelist_account, whitelisted_caller,
};
use frame_support::{
	dispatch::UnfilteredDispatchable,
	pallet_prelude::*,
	traits::{Currency, CurrencyToVote, Get, Imbalance},
};
use pallet_cess_staking::{
	testing_utils, Config as StakingConfig, Pallet as Staking, RewardDestination,
};
use frame_system::RawOrigin;

pub struct Pallet<T: Config>(TeeWorker<T>);
pub trait Config:
	crate::Config + pallet_cess_staking::Config
{
}

const USER_SEED: u32 = 999666;
const PODR2_PBK: [u8; 270] = [48, 130, 1, 10, 2, 130, 1, 1, 0, 232, 164, 71, 61, 156, 19, 143, 219, 24, 119, 196, 115, 203, 33, 130, 151, 75, 193, 108, 45, 181, 28, 191, 217, 102, 16, 251, 69, 156, 138, 34, 69, 68, 239, 167, 157, 121, 34, 146, 221, 122, 71, 183, 180, 190, 53, 5, 94, 154, 224, 178, 215, 143, 210, 96, 209, 51, 13, 153, 186, 209, 42, 30, 184, 117, 100, 112, 165, 234, 132, 48, 11, 137, 160, 143, 171, 209, 37, 93, 58, 237, 199, 4, 65, 231, 156, 171, 238, 184, 196, 182, 185, 5, 15, 216, 174, 194, 238, 247, 101, 25, 167, 108, 61, 236, 131, 208, 221, 104, 49, 198, 233, 98, 40, 30, 35, 0, 99, 93, 169, 190, 225, 76, 106, 55, 179, 135, 252, 71, 124, 215, 70, 189, 104, 167, 157, 31, 169, 7, 65, 147, 103, 47, 238, 62, 44, 136, 49, 31, 68, 176, 103, 77, 230, 83, 205, 162, 237, 154, 196, 193, 246, 79, 40, 206, 156, 87, 70, 178, 11, 64, 59, 174, 248, 210, 233, 140, 144, 93, 197, 115, 32, 133, 44, 157, 125, 226, 159, 221, 4, 19, 26, 247, 234, 54, 49, 216, 114, 142, 130, 13, 163, 250, 178, 72, 32, 187, 175, 59, 189, 53, 174, 19, 252, 169, 83, 235, 175, 38, 76, 241, 124, 131, 86, 46, 38, 87, 119, 45, 101, 51, 6, 133, 36, 178, 123, 212, 137, 57, 14, 110, 20, 164, 219, 79, 134, 82, 98, 213, 246, 115, 68, 119, 104, 157, 209, 2, 3, 1, 0, 1];

const NODE_PUBLIC_KEY: NodePublicKey = sp_core::ed25519::Public([0u8; 32]);

pub fn get_report() -> SgxAttestationReport {
    SgxAttestationReport {
        report_json_raw: "{\"id\":\"184395600112234849453795347768280755165\",\"timestamp\":\"2023-04-14T02:09:47.119891\",\"version\":4,\"epidPseudonym\":\"7+Jpi5RSDua6q1rmZHMpKdtPhfgoQj+74Ujo11SCqvYk/U1/s08H2mCekn5wkR/5rG7B1zrhJ1kyveR/kHxb/Dwu0jk79okEkT5vCIb0z2UXL2qcCmgU6faXkeuO2j6RqVjP/sxud0xUdZRfLQgh6RdCPbphnQGYitMV38G1MMA=\",\"advisoryURL\":\"https://security-center.intel.com\",\"advisoryIDs\":[\"INTEL-SA-00161\",\"INTEL-SA-00219\",\"INTEL-SA-00289\",\"INTEL-SA-00334\",\"INTEL-SA-00614\",\"INTEL-SA-00615\",\"INTEL-SA-00617\"],\"isvEnclaveQuoteStatus\":\"GROUP_OUT_OF_DATE\",\"platformInfoBlob\":\"150200650400090000131302040180070000000000000000000D00000C000000020000000000000C14356D7E6BD49FCE309AAC5007C3460F25CFD98D34DB2AD00836B4BCEF207509ADB761A9C3387BE53F6CAD756FE3024B91699F19D452294DEFC724873FAA232BF8\",\"isvEnclaveQuoteBody\":\"AgABABQMAAANAA0AAAAAAB6Xh0EMRY+We+07AgcxosUAAAAAAAAAAAAAAAAAAAAAERMCBv+AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABwAAAAAAAAAHAAAAAAAAAPQprxuAacfFpkMnlSNkZWrjSB7QpI7Bi9y7NYu6P3AmAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAJaXDwjRmUJaezDwOBzT4u9xntaxv/9C345MfmeBnIpAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAwKjAFBgMrZXADIQA6A6w8225D6fhgT3+5qpgxlYooSXnulLDOxPJcFiMr3AAAAAAAAAAAAAAAAAAAAAAAAAAA\"}".as_bytes().to_vec().try_into().unwrap(),
        sign: "STJ5RTIyg/NLcFN0528CGqQ08GfIu/czhUS1EJ1+gZhEqzEn9deC/j4nELvoCUiEZ2Xk5DSZ6eRIw9UYyWf5VUNFP95gHgpgz+/Ly35830fh3kjQGB078io43L8Cg3T/Y8e1yXV4MbFKGeLGRhR4ETAvVPfR+kVwapDvQAZ6AX+RcOfJVSsd16DYzIzlZLviq77soWVEW1n2Sqaf2zYXjasuW4YlLwbwKfQT7aZLAP8HWkpGaj7j9GnRlj3tiNZs7nZs7mE7voir+3bHGg/BDsxE0MPbMrZdqyiwejTjgyn4eNdjzYgD2ngYMVZnBJxJ3akxfVTnKDwn4rUeaxnh0g==".as_bytes().to_vec().try_into().unwrap(),
        cert_der: "MIIEoTCCAwmgAwIBAgIJANEHdl0yo7CWMA0GCSqGSIb3DQEBCwUAMH4xCzAJBgNVBAYTAlVTMQswCQYDVQQIDAJDQTEUMBIGA1UEBwwLU2FudGEgQ2xhcmExGjAYBgNVBAoMEUludGVsIENvcnBvcmF0aW9uMTAwLgYDVQQDDCdJbnRlbCBTR1ggQXR0ZXN0YXRpb24gUmVwb3J0IFNpZ25pbmcgQ0EwHhcNMTYxMTIyMDkzNjU4WhcNMjYxMTIwMDkzNjU4WjB7MQswCQYDVQQGEwJVUzELMAkGA1UECAwCQ0ExFDASBgNVBAcMC1NhbnRhIENsYXJhMRowGAYDVQQKDBFJbnRlbCBDb3Jwb3JhdGlvbjEtMCsGA1UEAwwkSW50ZWwgU0dYIEF0dGVzdGF0aW9uIFJlcG9ydCBTaWduaW5nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAqXot4OZuphR8nudFrAFiaGxxkgma/Es/BA+tbeCTUR106AL1ENcWA4FX3K+E9BBL0/7X5rj5nIgX/R/1ubhkKWw9gfqPG3KeAtIdcv/uTO1yXv50vqaPvE1CRChvzdS/ZEBqQ5oVvLTPZ3VEicQjlytKgN9cLnxbwtuvLUK7eyRPfJW/ksddOzP8VBBniolYnRCD2jrMRZ8nBM2ZWYwnXnwYeOAHV+W9tOhAImwRwKF/95yAsVwd21ryHMJBcGH70qLagZ7Ttyt++qO/6+KAXJuKwZqjRlEtSEz8gZQeFfVYgcwSfo96oSMAzVr7V0L6HSDLRnpb6xxmbPdqNol4tQIDAQABo4GkMIGhMB8GA1UdIwQYMBaAFHhDe3amfrzQr35CN+s1fDuHAVE8MA4GA1UdDwEB/wQEAwIGwDAMBgNVHRMBAf8EAjAAMGAGA1UdHwRZMFcwVaBToFGGT2h0dHA6Ly90cnVzdGVkc2VydmljZXMuaW50ZWwuY29tL2NvbnRlbnQvQ1JML1NHWC9BdHRlc3RhdGlvblJlcG9ydFNpZ25pbmdDQS5jcmwwDQYJKoZIhvcNAQELBQADggGBAGcIthtcK9IVRz4rRq+ZKE+7k50/OxUsmW8aavOzKb0iCx07YQ9rzi5nU73tME2yGRLzhSViFs/LpFa9lpQL6JL1aQwmDR74TxYGBAIi5f4I5TJoCCEqRHz91kpG6Uvyn2tLmnIdJbPE4vYvWLrtXXfFBSSPD4Afn7+3/XUggAlc7oCTizOfbbtOFlYA4g5KcYgS1J2ZAeMQqbUdZseZCcaZZZn65tdqee8UXZlDvx0+NdO0LR+5pFy+juM0wWbu59MvzcmTXbjsi7HY6zd53Yq5K244fwFHRQ8eOB0IWB+4PfM7FeAApZvlfqlKOlLcZL2uyVmzRkyR5yW72uo9mehX44CiPJ2fse9Y6eQtcfEhMPkmHXI01sN+KwPbpA39+xOsStjhP9N1Y1a2tQAVo+yVgLgV2Hws73Fc0o3wC78qPEA+v2aRs/Be3ZFDgDyghc/1fgU+7C+P6kbqd4poyb6IW8KCJbxfMJvkordNOgOUUxndPHEi/tb/U7uLjLOgPA==".as_bytes().to_vec().try_into().unwrap(),
    }
}

// pub fn register_tee_worker() {
//     let caller: T::AccountId = whitelisted_caller();
//     let (stash, controller) = pallet_cess_staking::testing_utils::create_stash_controller::<T>(USER_SEED, 100, Default::default())?;


// }

benchmarks! {
    register {
        let caller: T::AccountId = whitelisted_caller();
        let (stash, controller) = pallet_cess_staking::testing_utils::create_stash_controller::<T>(USER_SEED, 100, Default::default())?;
        let sgx_att_report = get_report();
    }: _(RawOrigin::Signed(controller.clone()), stash.clone(), NODE_PUBLIC_KEY, [0u8; 38], PODR2_PBK, sgx_att_report)
    verify {
		assert!(TeeWorkerMap::<T>::contains_key(&controller))
    }
}
