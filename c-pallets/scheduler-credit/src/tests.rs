use super::*;
use crate::mock::*;
use cp_scheduler_credit::SchedulerCreditCounter;
use frame_support::traits::ValidatorCredits;

#[test]
fn scheduler_credit_counter_works() {
	ExtBuilder::default().build_and_execute(|| {
		<Pallet<Test> as SchedulerCreditCounter<AccountId>>::record_proceed_block_size(&1, 100);
		<Pallet<Test> as SchedulerCreditCounter<AccountId>>::record_proceed_block_size(&1, 100);
		assert_eq!(200, CurrentCounters::<Test>::get(1).proceed_block_size);

		<Pallet<Test> as SchedulerCreditCounter<AccountId>>::record_proceed_block_size(&2, 50);
		<Pallet<Test> as SchedulerCreditCounter<AccountId>>::record_proceed_block_size(&3, 150);

        let vc_map = <Pallet<Test> as ValidatorCredits<AccountId>>::credits(1);
		assert_eq!(&500, vc_map.get(&1).unwrap());
		assert_eq!(&125, vc_map.get(&2).unwrap());
		assert_eq!(&375, vc_map.get(&3).unwrap());

		assert!(CurrentCounters::<Test>::contains_key(&1));
		assert!(CurrentCounters::<Test>::contains_key(&2));
		assert!(CurrentCounters::<Test>::contains_key(&3));
	});
}
