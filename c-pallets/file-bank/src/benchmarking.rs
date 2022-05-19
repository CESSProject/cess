#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::{
	whitelisted_caller, benchmarks,
};
use frame_system::RawOrigin as SystemOrigin;

benchmarks! {
	upload {
		let caller: T::AccountId = whitelisted_caller();
		let filename: Vec<u8> = "testfilename".as_bytes().to_vec();
		let address: Vec<u8> = "testaddress".as_bytes().to_vec();
        let mut fileid: Vec<u8> = Vec::new();
		fileid.push(1u8);
        let mut filehash: Vec<u8> = Vec::new();
		filehash.push(2u8);
        let mut simihash: Vec<u8> = Vec::new();
		simihash.push(3u8);
		let creator: Vec<u8> = "testcreator".as_bytes().to_vec();
		let keywords: Vec<u8> = "word go lang king upload".as_bytes().to_vec();
		let email: Vec<u8> = "2714857932@qq.com".as_bytes().to_vec();

	}: upload(SystemOrigin::Signed(caller.clone()), 
			filename, 
			address, 
			fileid, 
			filehash, 
			simihash, 
			0u8, 
			8u8, 
			creator, 
			123158u128, 
			keywords, 
			email, 
			0u32.into(), 
			0u32.into(), 
			0
	)
	verify {
		assert_eq!(1, 1);
	}

}
#[cfg(test)]
mod tests {
	use super::*;
	use crate::mock::{new_test_ext, Test};
	use frame_support::assert_ok;

	#[test]
	fn test_benchmarks() {
	new_test_ext().execute_with(|| {
		assert_ok!(Pallet::<Test>::test_benchmark_upload());
	});
	}
}