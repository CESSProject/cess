use super::*;
use serde_json::Value;
use sp_std::str::FromStr;

impl<T: pallet::Config> ChallengeReport<T> {
    pub fn parse(&self) -> Result<ReportMessage<T>, DispatchError> {
        let body: Value = serde_json::from_slice(&self.message).map_err(|_| Error::<T>::ParseError)?;

        let idle_str = body["idleFilter"].as_str().ok_or(Error::<T>::ParseError)?;
        let idle_str = idle_str.split(|elem| elem == ',');
        let mut index = 0;
        let mut idle_filter = BloomFilter::default();
        for elem in idle_str {
            idle_filter.0[index] = u64::from_str(elem).map_err(|_| Error::<T>::ParseError)?;
            index = index + 1;
        }

        let binding = body["serviceFilter"].as_str().ok_or(Error::<T>::ParseError)?;
        let service_str = binding.split(|elem| elem == ',');
        let mut index = 0;
        let mut service_filter = BloomFilter::default();
        for elem in service_str {
            service_filter.0[index] = u64::from_str(elem).map_err(|_| Error::<T>::ParseError)?;
            index = index + 1;
        }

        let binding = body["autonomyFilter"].as_str().ok_or(Error::<T>::ParseError)?;
        let autonomy_str = binding.split(|elem| elem == ',');
        let mut index = 0;
        let mut autonomy_filter = BloomFilter::default();
        for elem in autonomy_str {
            autonomy_filter.0[index] = u64::from_str(elem).map_err(|_| Error::<T>::ParseError)?;
            index = index + 1;
        }

        let binding = body["random"].as_str().ok_or(Error::<T>::ParseError)?;
        let random_bytes = binding.as_bytes();
        let random: [u8; 20] = random_bytes.try_into().map_err(|_| Error::<T>::ParseError)?;

        let mut failed_idle_file: Vec<Hash> = Default::default();
        let binding = body["failedIdleFile"].as_str().ok_or(Error::<T>::ParseError)?;
        let failed_bytes = binding.as_bytes();
        if failed_bytes.len() != 0 {
            let failed_arr = failed_bytes.split(|elem| elem == &b'/');
            for elem in failed_arr {
                let hash_temp = Hash::new(elem).map_err(|_| Error::<T>::ParseError)?;
                failed_idle_file.push(hash_temp);
            }
        }
        
        let mut failed_service_file: Vec<[u8; 68]> = Default::default();
        let binding = body["failedServiceFile"].as_str().ok_or(Error::<T>::ParseError)?;
        let failed_bytes = binding.as_bytes();
        if failed_bytes.len() != 0 {
            let failed_arr = failed_bytes.split(|elem| elem == &b'/');
            for elem in failed_arr {
                let share_id_temp: [u8; 68] = elem.try_into().map_err(|_| Error::<T>::ParseError)?;
                failed_service_file.push(share_id_temp);
            }
        }

        let mut failed_autonomy_file: Vec<Hash> = Default::default();
        let binding = body["failedAutonomyFile"].as_str().ok_or(Error::<T>::ParseError)?;
        let failed_bytes = binding.as_bytes();
        if failed_bytes.len() != 0 {
            let failed_arr = failed_bytes.split(|elem| elem == &b'/');
            for elem in failed_arr {
                let hash_temp = Hash::new(elem).map_err(|_| Error::<T>::ParseError)?;
                failed_autonomy_file.push(hash_temp);
            }
        }


        let report = ReportMessage::<T> {
            idle_filter: idle_filter,
            service_filter: service_filter,
            autonomy_filter: autonomy_filter,
            random: random,
            failed_idle_file: failed_idle_file.try_into().map_err(|_| Error::<T>::BoundedVecError)?,
            failed_service_file: failed_service_file.try_into().map_err(|_| Error::<T>::BoundedVecError)?,
            failed_autonomy_file: failed_autonomy_file.try_into().map_err(|_| Error::<T>::BoundedVecError)?,
        };

        Ok(report)
    }
}