
use crate::{mock::*, pallet::Error, *};
use frame_support::{assert_err, assert_ok};
use frame_support::traits::Len;

#[test]
fn registration_scheduler_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        pallet_cess_staking::Bonded::<Test>::insert(1, 2);
        pallet_cess_staking::Bonded::<Test>::insert(3, 1);
        assert_eq!(SchedulerMap::<Test>::get().len(), 0);
        assert_ok!(FileMap::registration_scheduler(Origin::signed(2), 1, Vec::from("127.0.0.1")));
        assert_err!(FileMap::registration_scheduler(Origin::signed(2), 1, Vec::from("127.0.0.1")), Error::<Test>::AlreadyRegistration);
        assert_err!(FileMap::registration_scheduler(Origin::signed(2), 3, Vec::from("192.168.0.1")), Error::<Test>::NotController);
    });
}

#[test]
fn update_scheduler_work() {
    ExtBuilder::default().build_and_execute(|| {
        pallet_cess_staking::Bonded::<Test>::insert(1, 2);

        assert_ok!(FileMap::registration_scheduler(Origin::signed(2), 1, Vec::from("127.0.0.1")));
        let scheduler_info = SchedulerMap::<Test>::get();
        assert_eq!(scheduler_info[0].ip.to_vec(), Vec::from("127.0.0.1"));

        assert_ok!(FileMap::update_scheduler(Origin::signed(2), Vec::from("192.168.0.1")));
        let scheduler_info = SchedulerMap::<Test>::get();
        assert_eq!(scheduler_info[0].ip.to_vec(), Vec::from("192.168.0.1"));
    });
}
