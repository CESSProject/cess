
use crate::{mock::*, pallet::Error, *};
use frame_support::{assert_err, assert_ok};
use frame_support::traits::Len;

#[test]
fn registration_scheduler_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        pallet_cess_staking::Bonded::<Test>::insert(1, 2);
        pallet_cess_staking::Bonded::<Test>::insert(3, 1);
        assert_eq!(SchedulerMap::<Test>::get().len(), 0);
				let ip = IpAddress::IPV4([127,0,0,1], 15000);
        assert_ok!(FileMap::registration_scheduler(Origin::signed(2), 1, ip.clone()));
        assert_err!(FileMap::registration_scheduler(Origin::signed(2), 1, ip.clone()), Error::<Test>::AlreadyRegistration);
        assert_err!(FileMap::registration_scheduler(Origin::signed(2), 3, ip), Error::<Test>::NotController);
    });
}

#[test]
fn update_scheduler_work() {
    ExtBuilder::default().build_and_execute(|| {
        pallet_cess_staking::Bonded::<Test>::insert(1, 2);
				let ip = IpAddress::IPV4([127,0,0,1], 15000);
        assert_ok!(FileMap::registration_scheduler(Origin::signed(2), 1, ip.clone()));
        let scheduler_info = SchedulerMap::<Test>::get();
        assert_eq!(scheduler_info[0].ip, ip.clone());
				let new_ip = IpAddress::IPV4([127,0,0,1], 15001);
        assert_ok!(FileMap::update_scheduler(Origin::signed(2), new_ip.clone()));
        let scheduler_info = SchedulerMap::<Test>::get();
        assert_eq!(scheduler_info[0].ip, new_ip.clone());
    });
}
