// use crate::{Pallet as OssPallet, *};
// use frame_benchmarking::{
// 	account, benchmarks, impl_benchmark_test_suite, whitelist_account, whitelisted_caller,
// };
// use frame_support::dispatch::RawOrigin;

// const SEED: u32 = 2190502;

// benchmarks! {
// 	authorize {
// 		let owner: AccountOf<T> = account("owner", 100, SEED);
// 		let operator: AccountOf<T> = account("operator", 100, SEED);
// 	}: _(RawOrigin::Signed(owner.clone()), operator.clone())
// 	verify {
// 		assert!(<AuthorityList<T>>::contains_key(&owner));
// 		let acc = <AuthorityList<T>>::get(&owner).unwrap();
// 		assert_eq!(acc, operator);
// 	}

// 	cancel_authorize {
// 		let owner: AccountOf<T> = account("owner", 100, SEED);
// 		let operator: AccountOf<T> = account("operator", 100, SEED);
// 		<AuthorityList<T>>::insert(&owner, operator.clone());
// 	}: _(RawOrigin::Signed(owner.clone()))
// 	verify {
// 		assert!(!<AuthorityList<T>>::contains_key(&owner));
// 	}

// 	register {
// 		let oss: AccountOf<T> = account("oss", 100, SEED);
// 		let ip: IpAddress = IpAddress::IPV4([127,0,0,1], 15000);
// 	}: _(RawOrigin::Signed(oss.clone()), ip.clone())
// 	verify {
// 		assert!(<Oss<T>>::contains_key(&oss));
// 		let oss_ip = <Oss<T>>::get(&oss).unwrap();
// 		assert_eq!(ip, oss_ip);
// 	}

// 	update {
// 		let oss: AccountOf<T> = account("oss", 100, SEED);
// 		let ip: IpAddress = IpAddress::IPV4([127,0,0,1], 15000);
// 		<Oss<T>>::insert(&oss, ip);
// 		let new_ip: IpAddress = IpAddress::IPV4([127,0,0,1], 15001);
// 	}: _(RawOrigin::Signed(oss.clone()), new_ip.clone())
// 	verify {
// 		assert!(<Oss<T>>::contains_key(&oss));
// 		let oss_ip = <Oss<T>>::get(&oss).unwrap();
// 		assert_eq!(new_ip, oss_ip);
// 	}

// 	destroy {
// 		let oss: AccountOf<T> = account("oss", 100, SEED);
// 		let ip: IpAddress = IpAddress::IPV4([127,0,0,1], 15000);
// 		<Oss<T>>::insert(&oss, ip.clone());
// 	}: _(RawOrigin::Signed(oss.clone()))
// 	verify {
// 		assert!(!<Oss<T>>::contains_key(&oss));
// 	}
// }
