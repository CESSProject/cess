#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

pub trait WeightInfo {
    fn buy_space() -> Weight;
	fn expansion_space() -> Weight;
	fn renewal_space() -> Weight;
}

pub struct SubstrateWeight<T>(PhantomData<T>);

impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    	// Storage: FileBank UserOwnedSpace (r:1 w:1)
	// Storage: FileBank UnitPrice (r:1 w:0)
	// Storage: Sminer PurchasedSpace (r:1 w:1)
	// Storage: Sminer TotalIdleSpace (r:1 w:0)
	// Storage: Sminer TotalServiceSpace (r:1 w:0)
	// Storage: System Account (r:2 w:2)
	fn buy_space() -> Weight {
		Weight::from_ref_time(297_601_000 as u64)
			.saturating_add(T::DbWeight::get().reads(7 as u64))
			.saturating_add(T::DbWeight::get().writes(4 as u64))
	}
	// Storage: FileBank UserOwnedSpace (r:1 w:1)
	// Storage: FileBank UnitPrice (r:1 w:0)
	// Storage: Sminer PurchasedSpace (r:1 w:1)
	// Storage: Sminer TotalIdleSpace (r:1 w:0)
	// Storage: Sminer TotalServiceSpace (r:1 w:0)
	// Storage: System Account (r:1 w:1)
	fn expansion_space() -> Weight {
		Weight::from_ref_time(3_972_139_000 as u64)
			.saturating_add(T::DbWeight::get().reads(6 as u64))
			.saturating_add(T::DbWeight::get().writes(3 as u64))
	}
	// Storage: FileBank UserOwnedSpace (r:1 w:1)
	// Storage: FileBank UnitPrice (r:1 w:0)
	// Storage: System Account (r:1 w:1)
	fn renewal_space() -> Weight {
		Weight::from_ref_time(313_401_000 as u64)
			.saturating_add(T::DbWeight::get().reads(3 as u64))
			.saturating_add(T::DbWeight::get().writes(2 as u64))
	}
}

impl WeightInfo for () {
    	// Storage: FileBank UserOwnedSpace (r:1 w:1)
	// Storage: FileBank UnitPrice (r:1 w:0)
	// Storage: Sminer PurchasedSpace (r:1 w:1)
	// Storage: Sminer TotalIdleSpace (r:1 w:0)
	// Storage: Sminer TotalServiceSpace (r:1 w:0)
	// Storage: System Account (r:2 w:2)
	fn buy_space() -> Weight {
		Weight::from_ref_time(297_601_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(7 as u64))
			.saturating_add(RocksDbWeight::get().writes(4 as u64))
	}
	// Storage: FileBank UserOwnedSpace (r:1 w:1)
	// Storage: FileBank UnitPrice (r:1 w:0)
	// Storage: Sminer PurchasedSpace (r:1 w:1)
	// Storage: Sminer TotalIdleSpace (r:1 w:0)
	// Storage: Sminer TotalServiceSpace (r:1 w:0)
	// Storage: System Account (r:1 w:1)
	fn expansion_space() -> Weight {
		Weight::from_ref_time(3_972_139_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(6 as u64))
			.saturating_add(RocksDbWeight::get().writes(3 as u64))
	}
	// Storage: FileBank UserOwnedSpace (r:1 w:1)
	// Storage: FileBank UnitPrice (r:1 w:0)
	// Storage: System Account (r:1 w:1)
	fn renewal_space() -> Weight {
		Weight::from_ref_time(313_401_000 as u64)
			.saturating_add(RocksDbWeight::get().reads(3 as u64))
			.saturating_add(RocksDbWeight::get().writes(2 as u64))
	}
}