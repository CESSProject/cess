use crate::*;

impl<T: Config> Pallet<T> {
    pub fn check_time_unix(signing_time: &u64) -> bool {
        let expiration = 4 * 60 * 60 * 1000; // 4 hours
        let now = T::UnixTime::now().as_millis().saturated_into::<u64>();
        if signing_time < &now && now <= signing_time + expiration {
            return true;
        } else {
            return false;
        }
    }
}