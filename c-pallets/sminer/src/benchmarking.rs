use super::*;
use crate::{Pallet as Sminer};
use codec::{alloc::string::ToString, Decode};
pub use frame_benchmarking::{
	account, benchmarks, impl_benchmark_test_suite, whitelist_account, whitelisted_caller,
};
use frame_support::{
	dispatch::UnfilteredDispatchable,
	pallet_prelude::*,
	traits::{Currency, CurrencyToVote, Get, Imbalance},
};

use sp_runtime::{
	traits::{Bounded, One, StaticLookup, TrailingZeroInput, Zero, CheckedMul},
	Perbill, Percent,
};
use frame_system::RawOrigin;

const SEED: u32 = 2190502;
const MAX_SPANS: u32 = 100;

pub fn add_miner<T: Config>(name: &'static str) -> T::AccountId {
    let ias_cert: IasCert = "MIIEoTCCAwmgAwIBAgIJANEHdl0yo7CWMA0GCSqGSIb3DQEBCwUAMH4xCzAJBgNVBAYTAlVTMQswCQYDVQQIDAJDQTEUMBIGA1UEBwwLU2FudGEgQ2xhcmExGjAYBgNVBAoMEUludGVsIENvcnBvcmF0aW9uMTAwLgYDVQQDDCdJbnRlbCBTR1ggQXR0ZXN0YXRpb24gUmVwb3J0IFNpZ25pbmcgQ0EwHhcNMTYxMTIyMDkzNjU4WhcNMjYxMTIwMDkzNjU4WjB7MQswCQYDVQQGEwJVUzELMAkGA1UECAwCQ0ExFDASBgNVBAcMC1NhbnRhIENsYXJhMRowGAYDVQQKDBFJbnRlbCBDb3Jwb3JhdGlvbjEtMCsGA1UEAwwkSW50ZWwgU0dYIEF0dGVzdGF0aW9uIFJlcG9ydCBTaWduaW5nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAqXot4OZuphR8nudFrAFiaGxxkgma/Es/BA+tbeCTUR106AL1ENcWA4FX3K+E9BBL0/7X5rj5nIgX/R/1ubhkKWw9gfqPG3KeAtIdcv/uTO1yXv50vqaPvE1CRChvzdS/ZEBqQ5oVvLTPZ3VEicQjlytKgN9cLnxbwtuvLUK7eyRPfJW/ksddOzP8VBBniolYnRCD2jrMRZ8nBM2ZWYwnXnwYeOAHV+W9tOhAImwRwKF/95yAsVwd21ryHMJBcGH70qLagZ7Ttyt++qO/6+KAXJuKwZqjRlEtSEz8gZQeFfVYgcwSfo96oSMAzVr7V0L6HSDLRnpb6xxmbPdqNol4tQIDAQABo4GkMIGhMB8GA1UdIwQYMBaAFHhDe3amfrzQr35CN+s1fDuHAVE8MA4GA1UdDwEB/wQEAwIGwDAMBgNVHRMBAf8EAjAAMGAGA1UdHwRZMFcwVaBToFGGT2h0dHA6Ly90cnVzdGVkc2VydmljZXMuaW50ZWwuY29tL2NvbnRlbnQvQ1JML1NHWC9BdHRlc3RhdGlvblJlcG9ydFNpZ25pbmdDQS5jcmwwDQYJKoZIhvcNAQELBQADggGBAGcIthtcK9IVRz4rRq+ZKE+7k50/OxUsmW8aavOzKb0iCx07YQ9rzi5nU73tME2yGRLzhSViFs/LpFa9lpQL6JL1aQwmDR74TxYGBAIi5f4I5TJoCCEqRHz91kpG6Uvyn2tLmnIdJbPE4vYvWLrtXXfFBSSPD4Afn7+3/XUggAlc7oCTizOfbbtOFlYA4g5KcYgS1J2ZAeMQqbUdZseZCcaZZZn65tdqee8UXZlDvx0+NdO0LR+5pFy+juM0wWbu59MvzcmTXbjsi7HY6zd53Yq5K244fwFHRQ8eOB0IWB+4PfM7FeAApZvlfqlKOlLcZL2uyVmzRkyR5yW72uo9mehX44CiPJ2fse9Y6eQtcfEhMPkmHXI01sN+KwPbpA39+xOsStjhP9N1Y1a2tQAVo+yVgLgV2Hws73Fc0o3wC78qPEA+v2aRs/Be3ZFDgDyghc/1fgU+7C+P6kbqd4poyb6IW8KCJbxfMJvkordNOgOUUxndPHEi/tb/U7uLjLOgPA=="
            .as_bytes().to_vec();

    let ias_sig: IasSig = "K2x0WBV6hIv9t/aHJrK3xIAtTT3L56gh+Aegp02pt673068EfrY/p6yzE3CjgLE82Rcocz2PQg8vHBZOZmj6mAL6kIi1yB0g8Zw0itssCpHP5EutFub7+qhdLkOoD5S8nQelhpmh/vZeNlKIotPQX4UitE+Y4XE++YG4oAJdBjQz/DdHgLNOXRrtZ8Em7M6dVhq5CVKQh9hCiDzHxUuw79nc0zjYY3vVOZIpNSblrjsVIQ+wUv+B7kAWjQv5q1/1LqQ+NLFXiAvhuhHBAhh/sGFCiuoWL2wKEJWyGXhiXVsFLtz7cFxmAmCHVfTB81UDqdFvRtfyssrA0XTTe8MbDw=="
            .as_bytes().to_vec();

    let quote_body: QuoteBody = "{\"id\":\"94430613512883727687118974192602172838\",\"timestamp\":\"2022-12-28T03:33:41.967203\",\"version\":4,\"epidPseudonym\":\"9B7Ac4onoHExqmjOIg0ldoYTF1jtI7wUlotfHyOqRTX36eZElWcxfxlEIeZy5RRMEeyEjMzl5q6H7fMUyTpDi3FJ9pIkskiHmnaXxSxMiR1Cx9czGmT6I+X5mrdDsprhY18ZqHITQ1eL5AeT2qVU0r2JpmekHzxdwgnE68GTb2o=\",\"advisoryURL\":\"https://security-center.intel.com\",\"advisoryIDs\":[\"INTEL-SA-00161\",\"INTEL-SA-00220\",\"INTEL-SA-00270\",\"INTEL-SA-00293\",\"INTEL-SA-00320\",\"INTEL-SA-00329\",\"INTEL-SA-00334\",\"INTEL-SA-00381\",\"INTEL-SA-00389\",\"INTEL-SA-00477\",\"INTEL-SA-00614\",\"INTEL-SA-00615\",\"INTEL-SA-00617\"],\"isvEnclaveQuoteStatus\":\"GROUP_OUT_OF_DATE\",\"platformInfoBlob\":\"1502006504000F0000131302040101070000000000000000000D00000C000000020000000000000B608AA1EECB8FFE3401066352CAE64592A72D3A79DA26BA867C02EF734B00A3FFD25D2281841A3F547279B9FD23CD87121EE48493559B131A79156070CDFDEBAAB9\",\"isvEnclaveQuoteBody\":\"AgABAGALAAAMAAwAAAAAANmMYLGtWKhiobXgZ0023bgAAAAAAAAAAAAAAAAAAAAABhMC//8CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABwAAAAAAAAAHAAAAAAAAAM1+FwG22OSX4XyShK7tfIrie1r9ON3xlgyjWd962YGZAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACD1xnnferKFHD2uvYqTXdDA8iZ22kCD5xw7h38CMfOngAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAADuc+kLRIl1VUXMyirr8d0fkwENw9NLPuxKNtPslYhQXoAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\"}"
            .as_bytes().to_vec();

    let sig: Signature = Signature([
        0xe6,0x29,0xc4,0xc0,0xd2,0x97,0xe9,0x48,0x9d,0x1b,0x30,
        0xdd,0x54,0x51,0x46,0xa9,0xf0,0xcc,0x39,0x20,0xf1,0x31,
        0xec,0x54,0x7d,0xcc,0x0d,0xaa,0x55,0x9a,0xde,0xc9,0x1e,
        0xf2,0x04,0x13,0x0f,0x3e,0xaf,0x20,0x24,0xbb,0x79,0x04,
        0x2a,0xab,0x90,0x1d,0xa8,0x7e,0xec,0x05,0x02,0x8c,0xfa,
        0xed,0x05,0xf7,0x37,0x12,0xab,0xd4,0xb5,0x10,0x01]);

	let miner: T::AccountId = account(name.clone(), 100, SEED);
	let ip = IpAddress::IPV4([127,0,0,1], 15001);
	T::Currency::make_free_balance_be(
		&miner,
		BalanceOf::<T>::max_value(),
	);
	whitelist_account!(miner);
	let _ = Sminer::<T>::regnstk(
		RawOrigin::Signed(miner.clone()).into(),
		miner.clone(),
		ip,
		2_000u32.into(),
        ias_cert, 
        ias_sig, 
        quote_body, 
        sig,
	);
	miner.clone()
}

pub fn set_mrenclave_code<T: Config>() {
    let code: Mrenclave = [0xcd,0x7e,0x17,0x01,0xb6,0xd8,0xe4,0x97,0xe1,0x7c,0x92,0x84,0xae,0xed,0x7c,0x8a,0xe2,0x7b,0x5a,0xfd,0x38,0xdd,0xf1,0x96,0x0c,0xa3,0x59,0xdf,0x7a,0xd9,0x81,0x99];

    let mut list = MrenclaveCodes::<T>::get();
    list.try_push(code).unwrap();
    MrenclaveCodes::<T>::put(list);
}

benchmarks! {
    regnstk {
        let ias_cert: IasCert = "MIIEoTCCAwmgAwIBAgIJANEHdl0yo7CWMA0GCSqGSIb3DQEBCwUAMH4xCzAJBgNVBAYTAlVTMQswCQYDVQQIDAJDQTEUMBIGA1UEBwwLU2FudGEgQ2xhcmExGjAYBgNVBAoMEUludGVsIENvcnBvcmF0aW9uMTAwLgYDVQQDDCdJbnRlbCBTR1ggQXR0ZXN0YXRpb24gUmVwb3J0IFNpZ25pbmcgQ0EwHhcNMTYxMTIyMDkzNjU4WhcNMjYxMTIwMDkzNjU4WjB7MQswCQYDVQQGEwJVUzELMAkGA1UECAwCQ0ExFDASBgNVBAcMC1NhbnRhIENsYXJhMRowGAYDVQQKDBFJbnRlbCBDb3Jwb3JhdGlvbjEtMCsGA1UEAwwkSW50ZWwgU0dYIEF0dGVzdGF0aW9uIFJlcG9ydCBTaWduaW5nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAqXot4OZuphR8nudFrAFiaGxxkgma/Es/BA+tbeCTUR106AL1ENcWA4FX3K+E9BBL0/7X5rj5nIgX/R/1ubhkKWw9gfqPG3KeAtIdcv/uTO1yXv50vqaPvE1CRChvzdS/ZEBqQ5oVvLTPZ3VEicQjlytKgN9cLnxbwtuvLUK7eyRPfJW/ksddOzP8VBBniolYnRCD2jrMRZ8nBM2ZWYwnXnwYeOAHV+W9tOhAImwRwKF/95yAsVwd21ryHMJBcGH70qLagZ7Ttyt++qO/6+KAXJuKwZqjRlEtSEz8gZQeFfVYgcwSfo96oSMAzVr7V0L6HSDLRnpb6xxmbPdqNol4tQIDAQABo4GkMIGhMB8GA1UdIwQYMBaAFHhDe3amfrzQr35CN+s1fDuHAVE8MA4GA1UdDwEB/wQEAwIGwDAMBgNVHRMBAf8EAjAAMGAGA1UdHwRZMFcwVaBToFGGT2h0dHA6Ly90cnVzdGVkc2VydmljZXMuaW50ZWwuY29tL2NvbnRlbnQvQ1JML1NHWC9BdHRlc3RhdGlvblJlcG9ydFNpZ25pbmdDQS5jcmwwDQYJKoZIhvcNAQELBQADggGBAGcIthtcK9IVRz4rRq+ZKE+7k50/OxUsmW8aavOzKb0iCx07YQ9rzi5nU73tME2yGRLzhSViFs/LpFa9lpQL6JL1aQwmDR74TxYGBAIi5f4I5TJoCCEqRHz91kpG6Uvyn2tLmnIdJbPE4vYvWLrtXXfFBSSPD4Afn7+3/XUggAlc7oCTizOfbbtOFlYA4g5KcYgS1J2ZAeMQqbUdZseZCcaZZZn65tdqee8UXZlDvx0+NdO0LR+5pFy+juM0wWbu59MvzcmTXbjsi7HY6zd53Yq5K244fwFHRQ8eOB0IWB+4PfM7FeAApZvlfqlKOlLcZL2uyVmzRkyR5yW72uo9mehX44CiPJ2fse9Y6eQtcfEhMPkmHXI01sN+KwPbpA39+xOsStjhP9N1Y1a2tQAVo+yVgLgV2Hws73Fc0o3wC78qPEA+v2aRs/Be3ZFDgDyghc/1fgU+7C+P6kbqd4poyb6IW8KCJbxfMJvkordNOgOUUxndPHEi/tb/U7uLjLOgPA=="
            .as_bytes().to_vec();

        let ias_sig: IasSig = "K2x0WBV6hIv9t/aHJrK3xIAtTT3L56gh+Aegp02pt673068EfrY/p6yzE3CjgLE82Rcocz2PQg8vHBZOZmj6mAL6kIi1yB0g8Zw0itssCpHP5EutFub7+qhdLkOoD5S8nQelhpmh/vZeNlKIotPQX4UitE+Y4XE++YG4oAJdBjQz/DdHgLNOXRrtZ8Em7M6dVhq5CVKQh9hCiDzHxUuw79nc0zjYY3vVOZIpNSblrjsVIQ+wUv+B7kAWjQv5q1/1LqQ+NLFXiAvhuhHBAhh/sGFCiuoWL2wKEJWyGXhiXVsFLtz7cFxmAmCHVfTB81UDqdFvRtfyssrA0XTTe8MbDw=="
            .as_bytes().to_vec();

        let quote_body: QuoteBody = "{\"id\":\"94430613512883727687118974192602172838\",\"timestamp\":\"2022-12-28T03:33:41.967203\",\"version\":4,\"epidPseudonym\":\"9B7Ac4onoHExqmjOIg0ldoYTF1jtI7wUlotfHyOqRTX36eZElWcxfxlEIeZy5RRMEeyEjMzl5q6H7fMUyTpDi3FJ9pIkskiHmnaXxSxMiR1Cx9czGmT6I+X5mrdDsprhY18ZqHITQ1eL5AeT2qVU0r2JpmekHzxdwgnE68GTb2o=\",\"advisoryURL\":\"https://security-center.intel.com\",\"advisoryIDs\":[\"INTEL-SA-00161\",\"INTEL-SA-00220\",\"INTEL-SA-00270\",\"INTEL-SA-00293\",\"INTEL-SA-00320\",\"INTEL-SA-00329\",\"INTEL-SA-00334\",\"INTEL-SA-00381\",\"INTEL-SA-00389\",\"INTEL-SA-00477\",\"INTEL-SA-00614\",\"INTEL-SA-00615\",\"INTEL-SA-00617\"],\"isvEnclaveQuoteStatus\":\"GROUP_OUT_OF_DATE\",\"platformInfoBlob\":\"1502006504000F0000131302040101070000000000000000000D00000C000000020000000000000B608AA1EECB8FFE3401066352CAE64592A72D3A79DA26BA867C02EF734B00A3FFD25D2281841A3F547279B9FD23CD87121EE48493559B131A79156070CDFDEBAAB9\",\"isvEnclaveQuoteBody\":\"AgABAGALAAAMAAwAAAAAANmMYLGtWKhiobXgZ0023bgAAAAAAAAAAAAAAAAAAAAABhMC//8CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABwAAAAAAAAAHAAAAAAAAAM1+FwG22OSX4XyShK7tfIrie1r9ON3xlgyjWd962YGZAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACD1xnnferKFHD2uvYqTXdDA8iZ22kCD5xw7h38CMfOngAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAADuc+kLRIl1VUXMyirr8d0fkwENw9NLPuxKNtPslYhQXoAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\"}"
            .as_bytes().to_vec();

        let sig: Signature = Signature([
        0xe6,0x29,0xc4,0xc0,0xd2,0x97,0xe9,0x48,0x9d,0x1b,0x30,
        0xdd,0x54,0x51,0x46,0xa9,0xf0,0xcc,0x39,0x20,0xf1,0x31,
        0xec,0x54,0x7d,0xcc,0x0d,0xaa,0x55,0x9a,0xde,0xc9,0x1e,
        0xf2,0x04,0x13,0x0f,0x3e,0xaf,0x20,0x24,0xbb,0x79,0x04,
        0x2a,0xab,0x90,0x1d,0xa8,0x7e,0xec,0x05,0x02,0x8c,0xfa,
        0xed,0x05,0xf7,0x37,0x12,0xab,0xd4,0xb5,0x10,0x01]);

        let miner: T::AccountId = account("miner1", 100, SEED);
	    let ip = IpAddress::IPV4([127,0,0,1], 15001);
        whitelist_account!(miner);
        <T as crate::Config>::Currency::make_free_balance_be(
            &miner,
            BalanceOf::<T>::max_value(),
        );
        set_mrenclave_code::<T>();
    }: _(RawOrigin::Signed(miner.clone()), miner.clone(), ip, 2_000u32.into(), ias_cert, ias_sig, quote_body, sig)
    verify {
        assert!(<MinerItems<T>>::contains_key(&miner));
    }

    increase_collateral {
        set_mrenclave_code::<T>();
        let miner = add_miner::<T>("miner");
        MinerItems::<T>::mutate(&miner, |m_opt| {
            let m = m_opt.as_mut().unwrap();
            m.state = STATE_DEBT.as_bytes().to_vec().try_into().unwrap();
            m.debt = 3000u32.into();
        });
        let acc: AccountOf<T> = T::PalletId::get().into_account();
		let balance: BalanceOf<T> = <T as crate::Config>::Currency::minimum_balance().checked_mul(&2u32.saturated_into()).ok_or("over flow")?;
		<T as crate::Config>::Currency::make_free_balance_be(
			&acc,
			balance,
		);
    }: _(RawOrigin::Signed(miner.clone()), 2000u32.into())
    verify {
        let miner_info = MinerItems::<T>::get(&miner).unwrap();
        assert_eq!(miner_info.collaterals, 1000u32.into());
    }

    update_beneficiary {
        set_mrenclave_code::<T>();
        let miner = add_miner::<T>("miner");

        let new_acc: T::AccountId = account("acc1", 100, SEED);

        let miner_info = <MinerItems<T>>::get(&miner).unwrap();
        assert_eq!(miner_info.beneficiary, miner.clone());
    }: _(RawOrigin::Signed(miner.clone()), new_acc.clone())
    verify {
        let miner_info = <MinerItems<T>>::get(&miner).unwrap();
        assert_eq!(miner_info.beneficiary, new_acc);
    }

    update_ip {
        set_mrenclave_code::<T>();
        let miner = add_miner::<T>("miner");

        let new_ip = IpAddress::IPV4([111,1,1,1], 15001);

        let miner_info = <MinerItems<T>>::get(&miner).unwrap();
        assert_eq!(miner_info.ip, IpAddress::IPV4([127,0,0,1], 15001));
    }: _(RawOrigin::Signed(miner.clone()), new_ip.clone())
    verify {
        let miner_info = <MinerItems<T>>::get(&miner).unwrap();
        assert_eq!(miner_info.ip, new_ip);
    }

    receive_reward {
        set_mrenclave_code::<T>();
        let miner = add_miner::<T>("miner");

        <RewardMap<T>>::mutate(&miner, |m_opt| {
            let m = m_opt.as_mut().unwrap();

            m.total_reward = 3000u32.into();
            m.currently_available_reward = 1000u32.into();
        });

        let acc: AccountOf<T> = T::PalletId::get().into_account();
		let balance: BalanceOf<T> = <T as crate::Config>::Currency::minimum_balance().checked_mul(&2u32.saturated_into()).ok_or("over flow")?;
		<T as crate::Config>::Currency::make_free_balance_be(
			&acc,
			balance,
		);

    }: _(RawOrigin::Signed(miner.clone()))
    verify {
        let miner_reward = <RewardMap<T>>::get(&miner).unwrap();
        assert_eq!(miner_reward.currently_available_reward, 0u32.into());
        assert_eq!(miner_reward.reward_issued, 1000u32.into());

    }
}
