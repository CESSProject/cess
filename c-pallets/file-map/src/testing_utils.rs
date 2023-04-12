use super::*;

pub fn add_scheduler<T: Config>(
	controller: AccountOf<T>,
	stash: AccountOf<T>,
	ip: IpAddress,
) -> DispatchResult {
	let scheduler =
		SchedulerInfo::<T> { ip, stash_user: stash.clone(), controller_user: controller.clone() };
	let mut s_vec = SchedulerMap::<T>::get();
	s_vec
		.try_push(scheduler)
		.map_err(|_e| "file-map testing_utils convert push err!")?;
	SchedulerMap::<T>::put(s_vec);

	Ok(())
}
