use super::*;

pub fn add_scheduler<T: Config>(
    controller: AccountOf<T>, 
    stash: AccountOf<T>, 
    ip: Vec<u8>
) -> DispatchResult {
    let ip_bound: BoundedVec<u8, <T as Config>::StringLimit> = ip.try_into().map_err(|_| "file-map testing_utils convert err!")?;
    let scheduler = SchedulerInfo::<T> {
        ip: ip_bound,
        stash_user: stash.clone(),
        controller_user: controller.clone(),
    };
    let mut s_vec = SchedulerMap::<T>::get();
    s_vec.try_push(scheduler).map_err(|_e| "file-map testing_utils convert push err!")?;
    SchedulerMap::<T>::put(s_vec);

    Ok(())
}