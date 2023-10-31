//! Tests for the module.

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::{new_test_ext, Cacher, RuntimeOrigin, Test};
use pallet_balances::Error as BalancesError;
use sp_runtime::traits::Hash;

#[test]
fn register_works() {
	new_test_ext().execute_with(|| {
		let info = CacherInfo::<AccountOf<Test>, BalanceOf<Test>> {
			payee: 1,
			ip: IpAddress::IPV4([127, 0, 0, 1], 8080),
			byte_price: 100u32.into(),
		};
		// Register works.
		assert_ok!(Cacher::register(RuntimeOrigin::signed(1), info.clone()));

		let result_info = Cachers::<Test>::get(&1).unwrap();
		assert_eq!(result_info, info);

		// Register again fails.
		assert_noop!(
			Cacher::register(RuntimeOrigin::signed(1), info.clone()),
			Error::<Test>::Registered
		);
	});
}

#[test]
fn update_works() {
	new_test_ext().execute_with(|| {
		let info = CacherInfo::<AccountOf<Test>, BalanceOf<Test>> {
			payee: 1,
			ip: IpAddress::IPV4([127, 0, 0, 1], 8080),
			byte_price: 100u32.into(),
		};
		assert_ok!(Cacher::register(RuntimeOrigin::signed(1), info.clone()));

		let new_info = CacherInfo::<AccountOf<Test>, BalanceOf<Test>> {
			payee: 1,
			ip: IpAddress::IPV4([127, 0, 0, 1], 80),
			byte_price: 200u32.into(),
		};
		// Wrong accout update fails.

		assert_noop!(
			Cacher::update(RuntimeOrigin::signed(2), new_info.clone()),
			Error::<Test>::UnRegister
		);
		// Update works.
		assert_ok!(Cacher::update(RuntimeOrigin::signed(1), new_info.clone()));

		let result_info = Cachers::<Test>::get(&1).unwrap();
		assert_eq!(result_info, new_info);
	});
}

#[test]
fn logout_works() {
	new_test_ext().execute_with(|| {
		let info = CacherInfo::<AccountOf<Test>, BalanceOf<Test>> {
			payee: 1,
			ip: IpAddress::IPV4([127, 0, 0, 1], 8080),
			byte_price: 100u32.into(),
		};
		assert_ok!(Cacher::register(RuntimeOrigin::signed(1), info.clone()));

		// Wrong accout logout fails.
		assert_noop!(Cacher::logout(RuntimeOrigin::signed(2)), Error::<Test>::UnRegister);

		// Logout works.
		assert_ok!(Cacher::logout(RuntimeOrigin::signed(1)));
	});
}

#[test]
fn pay_works() {
	new_test_ext().execute_with(|| {
		let n = 10;
		let amount: BalanceOf<Test> = 10000;
		let s_file = String::from("file");
		let s_slice = String::from("slice");
		let mut bill_vec = Vec::new();
		for i in 0..n {
			let bill =
				Bill::<AccountOf<Test>, BalanceOf<Test>, <Test as frame_system::Config>::Hash> {
					id: [i as u8; 16],
					to: 2,
					amount,
					file_hash: <Test as frame_system::Config>::Hashing::hash_of(&format!(
						"{}{}",
						s_file, i
					)),
					slice_hash: <Test as frame_system::Config>::Hashing::hash_of(&format!(
						"{}{}",
						s_slice, i
					)),
					expiration_time: 1675900800u64,
				};
			bill_vec.push(bill);
		}
		let bills: BoundedVec<_, <Test as Config>::BillsLimit> = bill_vec.try_into().unwrap();

		// Pay fails.
		assert_noop!(
			Cacher::pay(RuntimeOrigin::signed(1), bills.clone()),
			Error::<Test>::InsufficientBalance
		);

		<Test as Config>::Currency::make_free_balance_be(&1, BalanceOf::<Test>::max_value());
		let balance_befor_1 = <Test as Config>::Currency::free_balance(&1);
		let balance_before_2 = <Test as Config>::Currency::free_balance(&2);
		// Pay works.
		assert_ok!(Cacher::pay(RuntimeOrigin::signed(1), bills));
		let balance_after_1 = <Test as Config>::Currency::free_balance(&1);
		let balance_after_2 = <Test as Config>::Currency::free_balance(&2);
		assert_eq!(balance_befor_1 - balance_after_1, amount * n);
		assert_eq!(balance_after_2 - balance_before_2, amount * n);
	});
}
