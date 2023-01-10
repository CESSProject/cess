use super::*;

impl<AccountId, Balance, BoundedString> MinerInfo<AccountId, Balance, BoundedString> 
    where Balance: Zero, 
          BoundedString: TryFrom<Vec<u8>>,
{
    pub fn calculate_power(&self) -> u128 {
        let autonomy_power = AUTONOMY_MUTI.mul_floor(self.autonomy_space);

        let service_power = SERVICE_MUTI.mul_floor(self.service_space);

        let idle_power = IDLE_MUTI.mul_floor(self.idle_space);

        log::info!("idle_power: {:?}", idle_power);

        let power: u128 = autonomy_power + service_power + idle_power;

        power
    }
    
    pub fn new(beneficiary: AccountId, collaterals: Balance, ip: IpAddress, puk: Public, ias_cert: BoundedString) -> Self {
        let bloom = BloomCollect::default();

        MinerInfo::<AccountId, Balance, BoundedString> {
            beneficiary: beneficiary,
            ip: ip,
            collaterals: collaterals,
            debt: Zero::zero(),
            state: STATE_POSITIVE.as_bytes().to_vec().try_into().ok().unwrap(),
            idle_space: u128::MIN,
            service_space: u128::MIN,
            autonomy_space: u128::MIN,
            puk: puk,
            ias_cert: ias_cert,
            bloom_filter: bloom,
        }
    }
}

// impl<T: Config> MinerInfo<
//         <T as frame_system::Config>::AccountId,
//         <<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance,
//         BoundedVec<u8, T::ItemLimit>,
//     >
// {
//     pub fn calculate_power(&self) -> u128 {
//         let power: u128 = 
//             self.autonomy_space * AUTONOMY_MUTI +
//             self.idle_sapce * IDLE_MUTI +
//             self.service_space * SERVICE_MUTI
//             .into();
//         power
//     }

//     pub fn new(beneficiary: AccountOf<T>, collaterals: BalanceOf<T>, ip: IpAddress, puk: Public, ias_cert: IasCert) -> Self {
//         let bloom = BloomCollect::default();

//         MinerInfo::<AccountOf<T>, BalanceOf<T>, BoundedVec<u8, T::ItemLimit>> {
//             beneficiary: beneficiary,
//             ip: ip,
//             collaterals: collaterals,
//             debt: Zero::zero(),
//             state: STATE_POSITIVE.as_bytes().to_vec().into(),
//             idle_space: u128::MIN,
//             service_space: u128::MIN,
//             autonomy_space: u128::MIN,
//             puk: puk,
//             ias_cert: ias_cert,
//             bloom_filter: bloom,
//         }
//     }
// }