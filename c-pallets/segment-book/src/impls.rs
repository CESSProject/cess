use super::*;

use sp_std::str::FromStr;

impl<T: pallet::Config> ChallengeReport<T> {
    pub fn parse(&self) -> Result<ReportMessage<T>, DispatchError> {
        log::info!("start!");
        let body: Value = serde_json::from_slice(&self.message).map_err(|_| Error::<T>::ParseError)?;
        log::info!("---1---");
        let binding = body["idle_bloom_filter"].as_array().ok_or(Error::<T>::ParseError)?;
        let idle_filter_array = Self::parse_array(binding)?;
        let idle_filter = BloomFilter(idle_filter_array);
        log::info!("---2---");
        let binding = body["service_bloom_filter"].as_array().ok_or(Error::<T>::ParseError)?;
        let service_filter_array = Self::parse_array(binding)?;
        let service_filter = BloomFilter(service_filter_array);
        log::info!("---2---");
        let binding = body["autonomous_bloom_filter"].as_array().ok_or(Error::<T>::ParseError)?;
        let autonomy_filter_array = Self::parse_array(binding)?;
        let autonomy_filter = BloomFilter(autonomy_filter_array);
        log::info!("---4---");
        let binding = body["chal_id"].as_str().ok_or(Error::<T>::ParseError)?;
        let random_bytes: Vec<u8> = match base64::decode(binding) {
            Ok(value) => value,
            Err(_) => Err(Error::<T>::ParseError)?,
        };
        let random: [u8; 20] = random_bytes.try_into().map_err(|_| Error::<T>::ParseError)?;
        log::info!("---5---");
        let mut failed_idle_file: Vec<Hash> = Default::default();
        let binding = body["idle_failed_file_hashes"].as_str().ok_or(Error::<T>::ParseError)?;
        let failed_bytes = binding.as_bytes();
        if failed_bytes.len() != 0 {
            let failed_arr = failed_bytes.split(|elem| elem == &b'|');
            for elem in failed_arr {
                let hash_temp = Hash::new(elem).map_err(|_| Error::<T>::ParseError)?;
                failed_idle_file.push(hash_temp);
            }
        }
        log::info!("---6---");
        let mut failed_service_file: Vec<[u8; 68]> = Default::default();
        let binding = body["service_failed_file_hashes"].as_str().ok_or(Error::<T>::ParseError)?;
        let failed_bytes = binding.as_bytes();
        if failed_bytes.len() != 0 {
            let failed_arr = failed_bytes.split(|elem| elem == &b'|');
            for elem in failed_arr {
                let share_id_temp: [u8; 68] = elem.try_into().map_err(|_| Error::<T>::ParseError)?;
                failed_service_file.push(share_id_temp);
            }
        }
        log::info!("---7---");
        let mut failed_autonomy_file: Vec<Hash> = Default::default();
        let binding = body["autonomous_failed_file_hashes"].as_str().ok_or(Error::<T>::ParseError)?;
        let failed_bytes = binding.as_bytes();
        if failed_bytes.len() != 0 {
            let failed_arr = failed_bytes.split(|elem| elem == &b'|');
            for elem in failed_arr {
                let hash_temp = Hash::new(elem).map_err(|_| Error::<T>::ParseError)?;
                failed_autonomy_file.push(hash_temp);
            }
        }
        log::info!("---8---");

        let report = ReportMessage::<T> {
            idle_filter: idle_filter,
            service_filter: service_filter,
            autonomy_filter: autonomy_filter,
            random: random,
            failed_idle_file: failed_idle_file.try_into().map_err(|_| Error::<T>::BoundedVecError)?,
            failed_service_file: failed_service_file.try_into().map_err(|_| Error::<T>::BoundedVecError)?,
            failed_autonomy_file: failed_autonomy_file.try_into().map_err(|_| Error::<T>::BoundedVecError)?,
        };
        log::info!("---9---");
        Ok(report)
    }

    pub fn parse_array(binding: &Vec<Value>) -> Result<[u64; 256], DispatchError> {
		log::info!("comp length");
		ensure!(binding.len() == 256, Error::<T>::ParseError);
        log::info!("---comp success---");
		let mut array = [0u64; 256];
		let mut index = 0;
		for elem in binding {
			let temp = match elem {
				Value::Number(v) => v,
				_ => Err(Error::<T>::ParseError)?,
			};
			array[index] = temp.as_u64().ok_or(Error::<T>::ParseError)?;
            index = index + 1;
		}
		log::info!("array = {:?}",array);
	
		Ok(array)
	}
}