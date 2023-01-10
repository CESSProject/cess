use super::*;


impl<T: Config> Default for SliceInfo<T> {
    fn default() -> Self {
        SliceInfo::<T> {
            shard_id: [0u8; 68],
            slice_hash: Default::default(),
            shard_size: 0,
            miner_ip: IpAddress::IPV4([0,0,0,0], 15000),
            miner_acc: T::FilbakPalletId::get().into_account(),
        }
    }
}

impl<T: Config> Default for Backup<T> {
    fn default() -> Self {
        Backup::<T> {
            index: 0,
            slices: Default::default(),
        }
    }
}

impl<T: Config> SliceInfo<T> {
    pub fn serde_json_parse(acc: AccountOf<T>, message: Vec<u8>) -> Result<Self, DispatchError> {
        let body: Value = serde_json::from_slice(&message).map_err(|_| Error::<T>::ParseError)?;

        let binding = body["shardId"].as_str().ok_or(Error::<T>::ParseError)?;
        let shard_id = binding.as_bytes();
        let shard_id: [u8; 68] = shard_id.try_into().map_err(|_| Error::<T>::ParseError)?;

        let binding = body["sliceHash"].as_str().ok_or(Error::<T>::ParseError)?;
        let slice_hash = binding.as_bytes();
        let slice_hash = Hash::new(slice_hash).map_err(|_| Error::<T>::ParseError)?;

        let binding = body["minerIp"].as_str().ok_or(Error::<T>::ParseError)?;
        let mut miner_ip_arr = binding.split('/');
        // let miner_ip_arr = miner_ip.split(|num| num == &b'/');
        let ip1 = match miner_ip_arr.next() {
            Some(v) =>  u8::from_str(v).map_err(|_| Error::<T>::ParseError)?,
            None => Err(Error::<T>::ParseError)?,
        };
        let ip2 = match miner_ip_arr.next() {
            Some(v) => u8::from_str(v).map_err(|_| Error::<T>::ParseError)?,
            None => Err(Error::<T>::ParseError)?,
        };
        let ip3 = match miner_ip_arr.next() {
            Some(v) => u8::from_str(v).map_err(|_| Error::<T>::ParseError)?,
            None => Err(Error::<T>::ParseError)?,
        };
        let ip4 = match miner_ip_arr.next() {
            Some(v) => u8::from_str(v).map_err(|_| Error::<T>::ParseError)?,
            None => Err(Error::<T>::ParseError)?,
        };
        let port = match miner_ip_arr.next() {
            Some(v) => u16::from_str(v).map_err(|_| Error::<T>::ParseError)?,
            None => Err(Error::<T>::ParseError)?,
        };

        let miner_ip = IpAddress::IPV4([ip1, ip2, ip3, ip4], port);

        Ok(SliceInfo::<T> {
            shard_id: shard_id,
            slice_hash: slice_hash,
            shard_size: SLICE_DEFAULT_BYTE as u64,
            miner_ip: miner_ip,
            miner_acc: acc,
        })
    }
}

