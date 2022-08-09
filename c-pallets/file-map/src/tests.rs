#![cfg(test)]

use crate::{mock::*, pallet::Error, *};
use frame_support::{assert_err, assert_ok};
use frame_support::traits::{Hooks, Len};

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

// #[test]
// fn scheduler_exception_report_should_work() {
//     ExtBuilder::default().build_and_execute(|| {
//         assert_eq!(SchedulerException::<Test>::get(1), None);
//         assert_ok!(FileMap::scheduler_exception_report(Origin::signed(1), 1));
//         assert_err!(FileMap::scheduler_exception_report(Origin::signed(1), 1), Error::<Test>::AlreadyReport);
//         assert_eq!(SchedulerException::<Test>::get(1).len(), 1);
//         assert_ok!(FileMap::scheduler_exception_report(Origin::signed(2), 1));
//     });
// }

// #[test]
// fn must_apply_slash_for_hooks_on_initialize() {
//     ExtBuilder::default().build_and_execute(|| {
//         pallet_cess_staking::Bonded::<Test>::insert(1, 2);
//         pallet_cess_staking::Bonded::<Test>::insert(2, 3);
//         pallet_cess_staking::Bonded::<Test>::insert(3, 4);
//         assert_ok!(FileMap::registration_scheduler(Origin::signed(2), 1, Vec::from("127.0.0.1")));
//         assert_ok!(FileMap::registration_scheduler(Origin::signed(3), 2, Vec::from("127.0.0.1")));
//         assert_ok!(FileMap::registration_scheduler(Origin::signed(4), 3, Vec::from("127.0.0.1")));
//         assert_ok!(FileMap::scheduler_exception_report(Origin::signed(2), 2));
//         assert_ok!(FileMap::scheduler_exception_report(Origin::signed(3), 2));
//         assert_ok!(FileMap::scheduler_exception_report(Origin::signed(4), 2));
//         <FileMap as Hooks<u64>>::on_initialize(1200);
//         //TODO: to refactor pallet_cess_staking::slashing::slash_scheduler() dependency
//         for event_entry in &System::events() {
//             println!("{:?}", event_entry.event);
//         }
//     });
// }

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