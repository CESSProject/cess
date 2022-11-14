use super::*;
use crate::mock::*;
use cp_scheduler_credit::SchedulerCreditCounter;
use frame_support::traits::ValidatorCredits;

#[test]
fn figure_credit_scores_works() {
	ExtBuilder::default().build_and_execute(|| {
		<Pallet<Test> as SchedulerCreditCounter<AccountId>>::record_proceed_block_size(&1, 100);
		<Pallet<Test> as SchedulerCreditCounter<AccountId>>::record_proceed_block_size(&1, 100);
		assert_eq!(200, CurrentCounters::<Test>::get(1).proceed_block_size);

		<Pallet<Test> as SchedulerCreditCounter<AccountId>>::record_proceed_block_size(&2, 50);
		<Pallet<Test> as SchedulerCreditCounter<AccountId>>::record_proceed_block_size(&3, 150);

		// switch period
		let period_duration = PeriodDuration::get();
		System::set_block_number(period_duration);
		Pallet::<Test>::on_initialize(System::block_number());
		
		// figure credit values works
		assert_eq!(HistoryCreditValues::<Test>::get(&0, &1), 500);
		assert_eq!(HistoryCreditValues::<Test>::get(&0, &2), 125);
		assert_eq!(HistoryCreditValues::<Test>::get(&0, &3), 375);

		// clear CurrentCounters works
		assert_eq!(CurrentCounters::<Test>::contains_key(&1), false);
		assert_eq!(CurrentCounters::<Test>::contains_key(&2), false);
		assert_eq!(CurrentCounters::<Test>::contains_key(&3), false);

		// figure credit scores works
        let vc_map = <Pallet<Test> as ValidatorCredits<AccountId>>::credits(0);
		assert_eq!(&250, vc_map.get(&1).unwrap());
		assert_eq!(&62, vc_map.get(&2).unwrap());
		assert_eq!(&187, vc_map.get(&3).unwrap());
	});
}
