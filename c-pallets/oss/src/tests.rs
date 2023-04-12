use super::*;
use crate::{
	mock::{Oss, *},
	Oss as OssList,
};
use frame_support::{assert_err, assert_ok};

#[test]
fn authorize_work() {
	ExtBuilder::default().build_and_execute(|| {
		let owner = account1();
		let operator = account2();

		assert_ok!(Oss::authorize(RuntimeOrigin::signed(owner.clone()), operator.clone()));
		let verify_operator = AuthorityList::<Test>::get(&owner).unwrap();
		assert_eq!(verify_operator, operator);
	});
}

#[test]
fn cancel_authorize_work() {
	ExtBuilder::default().build_and_execute(|| {
		let owner = account1();
		let operator = account2();
		assert_ok!(Oss::authorize(RuntimeOrigin::signed(owner.clone()), operator.clone()));
		assert!(AuthorityList::<Test>::contains_key(&owner));

		assert_ok!(Oss::cancel_authorize(RuntimeOrigin::signed(owner.clone())));
		assert!(!AuthorityList::<Test>::contains_key(&owner));
	});
}

#[test]
fn register_work() {
	ExtBuilder::default().build_and_execute(|| {
		let oss = account1();
		let ip = IpAddress::IPV4([127, 0, 0, 1], 15000);
		assert_ok!(Oss::register(RuntimeOrigin::signed(oss.clone()), ip.clone()));

		let result_ip = OssList::<Test>::get(&oss).unwrap();
		assert_eq!(result_ip, ip);
	});
}

#[test]
fn register_err_registered() {
	ExtBuilder::default().build_and_execute(|| {
		let oss = account1();
		let ip = IpAddress::IPV4([127, 0, 0, 1], 15000);
		assert_ok!(Oss::register(RuntimeOrigin::signed(oss.clone()), ip.clone()));
		assert_err!(
			Oss::register(RuntimeOrigin::signed(oss.clone()), ip.clone()),
			Error::<Test>::Registered
		);
	});
}

#[test]
fn update_work() {
	ExtBuilder::default().build_and_execute(|| {
		let oss = account1();
		let ip = IpAddress::IPV4([127, 0, 0, 1], 15000);
		assert_ok!(Oss::register(RuntimeOrigin::signed(oss.clone()), ip.clone()));

		let result_ip = OssList::<Test>::get(&oss).unwrap();
		assert_eq!(result_ip, ip);

		let new_ip = IpAddress::IPV4([127, 0, 0, 1], 15001);
		assert_ok!(Oss::update(RuntimeOrigin::signed(oss.clone()), new_ip.clone()));

		let result_ip = OssList::<Test>::get(&oss).unwrap();
		assert_eq!(result_ip, new_ip);
	});
}
