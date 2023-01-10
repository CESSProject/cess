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

    let ias_sig: IasSig = "FW/DAt2bbHxlrAUqMeiOWB/MDoXXaWeid/DPdgcjXgQXV5PY1C+2ZrE8B1gDaK746XWXfA/8XhHvlTVFTicTeNLThr4GcdKN/r2gky20clpbO9oRYY9dFsRXSbVaElbjDfXtzrx9owpAX+rJ0rQ3IWhCQrONtMusxMJuxUI2JofsNYBxw3u6rfQqst2cjB7O1VdssBpJZceX5+qlNIc0JV87JuFrOZZAD32AaNFJ1ek1+h5aHocr2V3qYcDZ995wX3GlJZGwhKSMvjCvu2lwayiVGm2IuoNXJ+NOTO0ag8D0Awj6RYUlBSVV6dATiNZ32cYftzd64lLjamh4WUYmpA=="
            .as_bytes().to_vec();

    let quote_body: QuoteBody = "{\"id\":\"163015725754598646541909052603894961873\",\"timestamp\":\"2023-01-04T08:54:17.294919\",\"version\":4,\"epidPseudonym\":\"9B7Ac4onoHExqmjOIg0ldoYTF1jtI7wUlotfHyOqRTX36eZElWcxfxlEIeZy5RRMEeyEjMzl5q6H7fMUyTpDi3FJ9pIkskiHmnaXxSxMiR1Cx9czGmT6I+X5mrdDsprhY18ZqHITQ1eL5AeT2qVU0r2JpmekHzxdwgnE68GTb2o=\",\"advisoryURL\":\"https://security-center.intel.com\",\"advisoryIDs\":[\"INTEL-SA-00161\",\"INTEL-SA-00220\",\"INTEL-SA-00270\",\"INTEL-SA-00293\",\"INTEL-SA-00320\",\"INTEL-SA-00329\",\"INTEL-SA-00334\",\"INTEL-SA-00381\",\"INTEL-SA-00389\",\"INTEL-SA-00477\",\"INTEL-SA-00614\",\"INTEL-SA-00615\",\"INTEL-SA-00617\"],\"isvEnclaveQuoteStatus\":\"GROUP_OUT_OF_DATE\",\"platformInfoBlob\":\"1502006504000F0000131302040101070000000000000000000D00000C000000020000000000000B601FB8D8B50FBAA12E8F1A8913032BFDA84219CF7A8AEE4FCEE4C90303DBE31B8284D5BBB3E626810F11719B84545FFAFD842443FDB9787AF2B322757CF1D3DC5B\",\"isvEnclaveQuoteBody\":\"AgABAGALAAAMAAwAAAAAANmMYLGtWKhiobXgZ0023bgAAAAAAAAAAAAAAAAAAAAABhMC//8CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABwAAAAAAAAAHAAAAAAAAAMfPZTOU+wq2a3TEZdayodYZPkLkQypNozUdorSLlbRVAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACD1xnnferKFHD2uvYqTXdDA8iZ22kCD5xw7h38CMfOngAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAC9X3WOzjS60PChQQayLn72+ORCpfqAaLsPH0FLhvDT9kAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\"}"
            .as_bytes().to_vec();

    let sig: Signature = Signature([
        0x90,0xe5,0x64,0x93,0xad,0x3e,0x12,0x6d,0x3d,0xdd,
        0xc2,0x3c,0xf9,0xc8,0xa9,0xc0,0x4c,0xe1,0xa8,0x5f,
        0x18,0xca,0x12,0x97,0xab,0x02,0x7e,0x17,0xe8,0x42,
        0xfa,0xa5,0x36,0x4d,0x71,0x1a,0x0b,0x60,0x5b,0xc1,
        0x91,0xf3,0x18,0xd0,0x8b,0x89,0xe3,0xc1,0x61,0x31,
        0xff,0xd8,0x89,0x60,0xed,0xc7,0xe1,0x93,0x3b,0x84,
        0x53,0x1a,0x4a,0xc9,0x00,
    ]);

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
    let code: Mrenclave = [0xc7,0xcf,0x65,0x33,0x94,0xfb,0x0a,0xb6,0x6b,0x74,0xc4,0x65,0xd6,0xb2,0xa1,0xd6,0x19,0x3e,0x42,0xe4,0x43,0x2a,0x4d,0xa3,0x35,0x1d,0xa2,0xb4,0x8b,0x95,0xb4,0x55];

    let mut list = MrenclaveCodes::<T>::get();
    list.try_push(code).unwrap();
    MrenclaveCodes::<T>::put(list);
}

benchmarks! {
    regnstk {
        let ias_cert: IasCert = "MIIEoTCCAwmgAwIBAgIJANEHdl0yo7CWMA0GCSqGSIb3DQEBCwUAMH4xCzAJBgNVBAYTAlVTMQswCQYDVQQIDAJDQTEUMBIGA1UEBwwLU2FudGEgQ2xhcmExGjAYBgNVBAoMEUludGVsIENvcnBvcmF0aW9uMTAwLgYDVQQDDCdJbnRlbCBTR1ggQXR0ZXN0YXRpb24gUmVwb3J0IFNpZ25pbmcgQ0EwHhcNMTYxMTIyMDkzNjU4WhcNMjYxMTIwMDkzNjU4WjB7MQswCQYDVQQGEwJVUzELMAkGA1UECAwCQ0ExFDASBgNVBAcMC1NhbnRhIENsYXJhMRowGAYDVQQKDBFJbnRlbCBDb3Jwb3JhdGlvbjEtMCsGA1UEAwwkSW50ZWwgU0dYIEF0dGVzdGF0aW9uIFJlcG9ydCBTaWduaW5nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAqXot4OZuphR8nudFrAFiaGxxkgma/Es/BA+tbeCTUR106AL1ENcWA4FX3K+E9BBL0/7X5rj5nIgX/R/1ubhkKWw9gfqPG3KeAtIdcv/uTO1yXv50vqaPvE1CRChvzdS/ZEBqQ5oVvLTPZ3VEicQjlytKgN9cLnxbwtuvLUK7eyRPfJW/ksddOzP8VBBniolYnRCD2jrMRZ8nBM2ZWYwnXnwYeOAHV+W9tOhAImwRwKF/95yAsVwd21ryHMJBcGH70qLagZ7Ttyt++qO/6+KAXJuKwZqjRlEtSEz8gZQeFfVYgcwSfo96oSMAzVr7V0L6HSDLRnpb6xxmbPdqNol4tQIDAQABo4GkMIGhMB8GA1UdIwQYMBaAFHhDe3amfrzQr35CN+s1fDuHAVE8MA4GA1UdDwEB/wQEAwIGwDAMBgNVHRMBAf8EAjAAMGAGA1UdHwRZMFcwVaBToFGGT2h0dHA6Ly90cnVzdGVkc2VydmljZXMuaW50ZWwuY29tL2NvbnRlbnQvQ1JML1NHWC9BdHRlc3RhdGlvblJlcG9ydFNpZ25pbmdDQS5jcmwwDQYJKoZIhvcNAQELBQADggGBAGcIthtcK9IVRz4rRq+ZKE+7k50/OxUsmW8aavOzKb0iCx07YQ9rzi5nU73tME2yGRLzhSViFs/LpFa9lpQL6JL1aQwmDR74TxYGBAIi5f4I5TJoCCEqRHz91kpG6Uvyn2tLmnIdJbPE4vYvWLrtXXfFBSSPD4Afn7+3/XUggAlc7oCTizOfbbtOFlYA4g5KcYgS1J2ZAeMQqbUdZseZCcaZZZn65tdqee8UXZlDvx0+NdO0LR+5pFy+juM0wWbu59MvzcmTXbjsi7HY6zd53Yq5K244fwFHRQ8eOB0IWB+4PfM7FeAApZvlfqlKOlLcZL2uyVmzRkyR5yW72uo9mehX44CiPJ2fse9Y6eQtcfEhMPkmHXI01sN+KwPbpA39+xOsStjhP9N1Y1a2tQAVo+yVgLgV2Hws73Fc0o3wC78qPEA+v2aRs/Be3ZFDgDyghc/1fgU+7C+P6kbqd4poyb6IW8KCJbxfMJvkordNOgOUUxndPHEi/tb/U7uLjLOgPA=="
         .as_bytes().to_vec();

        let ias_sig: IasSig = "l/zrYQh/QYCHhs6LV2wapSpq/xmGNyH26WktW1A2UgxIosfMpIqdEDUKU69GOI66zCSJbE4frqq3Pxk1VBzRqTPANg3kdUXYROwiPPf47+qmlX6b5/AtncgstthmE5kJYk1ZmlX2W9C/GCpdIN3hak70o9MXq5WFWtqiTBae9G4koUhL5LmP48soP+oUxeGxofYajT5L/jPSySe/P9y/xGJet1Hdba0XDLauAHbp+TiQLV4mFXC20Eq+NYRFSAMFNs71o+e5WQZNrZIYBaul2C5tUG00ev2UivzDhsoZRRnWt3Iu3hRRZX3+au/y/Tqvnb7Epp7KscjhVVBJAxxcZQ=="
            .as_bytes().to_vec();

        let quote_body: QuoteBody = "{\"id\":\"14045970018994034909234870769014222941\",\"timestamp\":\"2023-01-03T08:55:25.143595\",\"version\":4,\"epidPseudonym\":\"9B7Ac4onoHExqmjOIg0ldoYTF1jtI7wUlotfHyOqRTX36eZElWcxfxlEIeZy5RRMEeyEjMzl5q6H7fMUyTpDi3FJ9pIkskiHmnaXxSxMiR1Cx9czGmT6I+X5mrdDsprhY18ZqHITQ1eL5AeT2qVU0r2JpmekHzxdwgnE68GTb2o=\",\"advisoryURL\":\"https://security-center.intel.com\",\"advisoryIDs\":[\"INTEL-SA-00161\",\"INTEL-SA-00220\",\"INTEL-SA-00270\",\"INTEL-SA-00293\",\"INTEL-SA-00320\",\"INTEL-SA-00329\",\"INTEL-SA-00334\",\"INTEL-SA-00381\",\"INTEL-SA-00389\",\"INTEL-SA-00477\",\"INTEL-SA-00614\",\"INTEL-SA-00615\",\"INTEL-SA-00617\"],\"isvEnclaveQuoteStatus\":\"GROUP_OUT_OF_DATE\",\"platformInfoBlob\":\"1502006504000F0000131302040101070000000000000000000D00000C000000020000000000000B6040F02500802F5B58133DBE85BF506527CE0280F56E4E2A338BB1E1AA4E59EDBB33159DF6B9B1F0F9F8AAF9E5A847465810C70F830628C1DA5566CE324613DA58\",\"isvEnclaveQuoteBody\":\"AgABAGALAAAMAAwAAAAAANmMYLGtWKhiobXgZ0023bgAAAAAAAAAAAAAAAAAAAAABhMC//8CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABwAAAAAAAAAHAAAAAAAAAH08cOGQof5p/YyNlXpS+LCA80pKmo750ebeZ7SEN1CsAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACD1xnnferKFHD2uvYqTXdDA8iZ22kCD5xw7h38CMfOngAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAADeC73vqhK0geGGRCNUVk/tMEq7nuZUbJdCtB9UWpijHYAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\"}"
            .as_bytes().to_vec();

        let sig: Signature = Signature([
            0x45,0x41,0xcc,0x38,0x3f,0x2a,0xdf,0x37,0x3d,0xcd,0xcb,0xd2,0xdf,0x5d,
            0x53,0x5b,0xa1,0x0b,0x62,0x21,0x11,0xc0,0x49,0x5a,0x3f,0xe6,0x76,0x32,
            0x27,0x55,0x35,0xc4,0x58,0xa3,0x07,0x5b,0x68,0x2e,0xd2,0xef,0x4f,0x93,
            0x7d,0xd4,0x60,0xf8,0x91,0x6e,0xff,0x10,0x9d,0xed,0x73,0x0b,0xd2,0x15,
            0x3f,0xa9,0x76,0x60,0xf0,0x38,0x17,0x06,0x01
        ]);

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
