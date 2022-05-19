#![cfg(test)]

use crate::{mock::*, pallet::Error, *};
use frame_support::{assert_err, assert_ok};
use frame_support::traits::{Hooks, Len};

// #[test]
// fn registration_scheduler_should_work() {
//     ExtBuilder::default().build_and_execute(|| {
//         assert_eq!(SchedulerMap::<Test>::get().len(), 0);
//         assert_ok!(FileMap::registration_scheduler(Origin::signed(1), 1, Vec::from("127.0.0.1")));
//         assert_err!(FileMap::registration_scheduler(Origin::signed(1), 1, Vec::from("127.0.0.1")), Error::<Test>::AlreadyRegistration);
//         assert_ok!(FileMap::registration_scheduler(Origin::signed(1), 2, Vec::from("192.168.0.1")));
//         assert_ok!(FileMap::registration_scheduler(Origin::signed(1), 3, Vec::from("192.168.0.1")));
//     });
// }

#[test]
fn scheduler_exception_report_should_work() {
    ExtBuilder::default().build_and_execute(|| {
        assert_eq!(SchedulerException::<Test>::get(1), None);
        assert_ok!(FileMap::scheduler_exception_report(Origin::signed(1), 1));
        assert_err!(FileMap::scheduler_exception_report(Origin::signed(1), 1), Error::<Test>::AlreadyReport);
        assert_eq!(SchedulerException::<Test>::get(1).len(), 1);
        assert_ok!(FileMap::scheduler_exception_report(Origin::signed(2), 1));
    });
}

// #[test]
// fn must_apply_slash_for_hooks_on_initialize() {
//     ExtBuilder::default().build_and_execute(|| {
//         assert_ok!(FileMap::registration_scheduler(Origin::signed(2), 1, Vec::from("127.0.0.1")));
//         assert_ok!(FileMap::registration_scheduler(Origin::signed(3), 1, Vec::from("127.0.0.1")));
//         assert_ok!(FileMap::registration_scheduler(Origin::signed(4), 1, Vec::from("127.0.0.1")));
//         assert_ok!(FileMap::scheduler_exception_report(Origin::signed(2), 1));
//         assert_ok!(FileMap::scheduler_exception_report(Origin::signed(3), 1));
//         assert_ok!(FileMap::scheduler_exception_report(Origin::signed(4), 1));
//         <FileMap as Hooks<u64>>::on_initialize(1200);
//         //TODO: to refactor pallet_cess_staking::slashing::slash_scheduler() dependency
//         for event_entry in &System::events() {
//             println!("{:?}", event_entry.event);
//         }
//     });
// }