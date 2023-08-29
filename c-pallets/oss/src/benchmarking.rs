use crate::{Pallet as OssPallet, *};
use frame_benchmarking::{
	account, benchmarks, impl_benchmark_test_suite, whitelist_account, whitelisted_caller,
};
use frame_support::dispatch::RawOrigin;

const SEED: u32 = 2190502;

benchmarks! {
	authorize {
		let owner: AccountOf<T> = account("owner", 100, SEED);
		let operator: AccountOf<T> = account("operator", 100, SEED);
	}: _(RawOrigin::Signed(owner.clone()), operator.clone())
	verify {
		assert!(<AuthorityList<T>>::contains_key(&owner));
		let acc_list = <AuthorityList<T>>::get(&owner);
		assert!(acc_list.contains(&operator));
	}

	cancel_authorize {
		let owner: AccountOf<T> = account("owner", 100, SEED);
        let operator: AccountOf<T> = account("operator", 100, SEED);
		let mut operator_list: BoundedVec<AccountOf<T>, T::AuthorLimit> = Default::default();
        operator_list.try_push(operator.clone()).unwrap();
		<AuthorityList<T>>::insert(&owner, operator_list);
	}: _(RawOrigin::Signed(owner.clone()), operator.clone())
	verify {
        let operator_list = <AuthorityList<T>>::get(&owner);
		assert!(!operator_list.contains(&operator));
	}

	register {
		let oss: AccountOf<T> = account("oss", 100, SEED);
		let peer_id = [5u8; 38];
	}: _(RawOrigin::Signed(oss.clone()), peer_id.clone())
	verify {
		assert!(<Oss<T>>::contains_key(&oss));
		let oss_ip = <Oss<T>>::get(&oss).unwrap();
		assert_eq!(oss_ip, peer_id);
	}

	update {
		let oss: AccountOf<T> = account("oss", 100, SEED);
		let peer_id = [5u8; 38];
		<Oss<T>>::insert(&oss, peer_id);
		let new_peer_id = [6u8; 38];
	}: _(RawOrigin::Signed(oss.clone()), new_peer_id.clone())
	verify {
		assert!(<Oss<T>>::contains_key(&oss));
		let oss_peer_id = <Oss<T>>::get(&oss).unwrap();
		assert_eq!(new_peer_id, oss_peer_id);
	}

	destroy {
		let oss: AccountOf<T> = account("oss", 100, SEED);
        let peer_id = [5u8; 38];
		<Oss<T>>::insert(&oss, peer_id.clone());
	}: _(RawOrigin::Signed(oss.clone()))
	verify {
		assert!(!<Oss<T>>::contains_key(&oss));
	}
}
