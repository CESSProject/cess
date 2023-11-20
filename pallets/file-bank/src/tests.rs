//! This file is part of CESS.
//!
//! Tests for the module.

use super::*;
use crate::{mock::*, Event};
use mock::System as Sys;
use frame_support::{assert_ok, assert_noop};
use cp_cess_common::{IpAddress, Hash};
use pallet_sminer::MinerControl;


#[derive(Debug, Clone)]
pub struct MockingFileBankInfo {
    file_hash: Hash,
    file_size: u64,
	  slice_info: Vec<SliceInfo<Test>>,
}

type Balance = u64;
const UNIT_PRICE: Balance = 30;

impl Default for MockingFileBankInfo {
    fn default() -> Self {
			let file_hash = Hash([5u8; 64]);
			let mut file_hash1: Vec<u8> = file_hash.0.to_vec();
			file_hash1.append("-001".as_bytes().to_vec().as_mut());
			let file_hash1: [u8; 68] = file_hash1.as_slice().try_into().unwrap();

			let mut file_hash2: Vec<u8> = file_hash.0.to_vec();
			file_hash2.append("-002".as_bytes().to_vec().as_mut());
			let file_hash2: [u8; 68] = file_hash1.as_slice().try_into().unwrap();

			let mut file_hash3: Vec<u8> = file_hash.0.to_vec();
			file_hash3.append("-002".as_bytes().to_vec().as_mut());
			let file_hash3: [u8; 68] = file_hash1.as_slice().try_into().unwrap();
        MockingFileBankInfo {
            file_hash: file_hash,
            file_size: 12,
            slice_info: vec![SliceInfo::<Test>{
                miner_id: 1,
                shard_size: 111,
                block_num: 8,
                shard_id: file_hash1,
                miner_ip: IpAddress::IPV4([127,0,0,1], 15000),
                miner_acc: mock::miner1(),
            },
            SliceInfo::<Test>{
                miner_id: 1,
                shard_size: 111,
                block_num: 8,
                shard_id: file_hash2,
                miner_ip: IpAddress::IPV4([127,0,0,1], 15000),
                miner_acc: mock::miner1(),
            },
            SliceInfo::<Test>{
                miner_id: 1,
                shard_size: 111,
                block_num: 8,
                shard_id: file_hash3,
                miner_ip: IpAddress::IPV4([127,0,0,1], 15000),
                miner_acc: mock::miner1(),
            },
            ]
        }
    }
}

fn upload_declaration_alias(account: AccountId, file_name: Vec<u8>, file_hash: Hash, bucket_name: Vec<u8>) -> DispatchResult {
		let user_brief = UserBrief::<Test>{
			user: account.clone(),
			file_name: file_name.try_into().unwrap(),
			bucket_name: bucket_name.try_into().unwrap(),
		};
    FileBank::upload_declaration(
        RuntimeOrigin::signed(account.clone()),
        file_hash,
        user_brief,
    )
}

fn upload_file_alias(_account: AccountId, controller: AccountId, file_info: &MockingFileBankInfo) -> DispatchResult {
    let MockingFileBankInfo { file_hash, file_size, slice_info} = file_info.clone();
    FileBank::upload(
        RuntimeOrigin::signed(controller),
        file_hash,
        file_size,
        slice_info,
    )
}

fn create_new_bucket(acc: AccountId, bucket_name: Vec<u8>) -> DispatchResult {
	FileBank::create_bucket(
		RuntimeOrigin::signed(acc.clone()),
		acc,
		bucket_name.try_into().unwrap(),
	)
}

fn register_scheduler(stash: AccountId, controller: AccountId) -> DispatchResult {
    pallet_cess_staking::Bonded::<Test>::insert(&stash, controller.clone());
    TeeWorker::registration_scheduler(
        RuntimeOrigin::signed(controller),
        stash,
        IpAddress::IPV4([127,0,0,1], 15000),
    )

}

fn register_miner(miner: AccountId) -> DispatchResult {
    Sminer::regnstk(
        RuntimeOrigin::signed(miner),
        miner.clone(),
				IpAddress::IPV4([127,0,0,1], 15000),
        2_000u128.try_into().unwrap(),
    )
}

fn add_power_for_miner(controller: AccountId, miner: AccountId) -> DispatchResult {
    let max: u8 = 10;
    let mut filler_list: Vec<FillerInfo<Test>> = Vec::new();
    for i in 0 .. max {
        let filler_hash: Hash = Hash([i; 64]);
        filler_list.push(
            FillerInfo::<Test> {
                index: 1,
                filler_size: 8 * 1_048_576,
                block_num: 8,
                segment_size: 1_048_576,
                scan_size: 1_048_576,
                miner_address: miner.clone(),
                filler_hash: filler_hash.clone(),
            }
        )
    }
    FileBank::upload_filler(RuntimeOrigin::signed(controller), miner, filler_list)?;

    Ok(())
}

#[test]
fn buy_space_works() {
    new_test_ext().execute_with(|| {
        let acc1 = mock::account1();
        let _bal_before = Balances::free_balance(acc1);
        let miner1 = mock::miner1();
        let space_gb = 10_u128;
        let bn = Sys::block_number();

        assert_ok!(register_miner(miner1.clone()));
        assert_ok!(Sminer::add_power(&miner1, 1_048_576 * 1024 * space_gb));  // GB => MB

        assert_ok!(FileBank::buy_space(RuntimeOrigin::signed(acc1), 10));

        let uhsd = UserOwnedSpace::<Test>::try_get(acc1).unwrap();
        assert_eq!(space_gb * 1024 * 1_048_576, uhsd.total_space); // MB unit
        assert_eq!(14400 * 30 + bn as u128, uhsd.deadline as u128);

        assert_eq!(uhsd.total_space, uhsd.remaining_space);
				let price = UNIT_PRICE * space_gb as u64;

        let event = Sys::events().pop().expect("Expected at least one BuySpace to be found").event;
        assert_eq!(mock::RuntimeEvent::from(Event::BuySpace { acc: acc1, storage_capacity: space_gb * 1024 * 1_048_576, spend: price }), event);
    });
}

#[test]
fn expansion_space_work() {
    new_test_ext().execute_with(|| {
        let acc1 = mock::account1();
        let miner1 = mock::miner1();
        let space_gb = 1000_u128;
        assert_ok!(register_miner(miner1.clone()));
        assert_ok!(Sminer::add_power(&miner1, 1_048_576 * 1024 * space_gb));  // GB => MB

        assert_ok!(FileBank::buy_space(RuntimeOrigin::signed(acc1), 1));
        let uhsd = UserOwnedSpace::<Test>::try_get(acc1).unwrap();
        assert_eq!(1 * 1024 * 1_048_576, uhsd.total_space); // MB unit

        assert_ok!(FileBank::expansion_space(RuntimeOrigin::signed(acc1), 1));
				let price = UNIT_PRICE * 1;

        let event = Sys::events().pop().expect("Expected at least one BuySpace to be found").event;
        assert_eq!(mock::RuntimeEvent::from(Event::ExpansionSpace { acc: acc1, expansion_space: 1024 * 1024 * 1024, fee: price,}), event);

        let uhsd = UserOwnedSpace::<Test>::try_get(acc1).unwrap();
        assert_eq!(2 * 1024 * 1_048_576, uhsd.total_space); // MB unit
    });
}

#[test]
fn renewal_space_work() {
    new_test_ext().execute_with(|| {
			let acc1 = mock::account1();
			let miner1 = mock::miner1();
			let space_gb = 1000_u128;
			assert_ok!(register_miner(miner1.clone()));
			assert_ok!(Sminer::add_power(&miner1, 1_048_576 * 1024 * space_gb));  // GB => MB

			let gib_count = 2;
			assert_ok!(FileBank::buy_space(RuntimeOrigin::signed(acc1), gib_count));
			let uhsd = UserOwnedSpace::<Test>::try_get(acc1).unwrap();
			assert_eq!(2 * 1024 * 1_048_576, uhsd.total_space); // MB unit

			let renewal_days = 32;
			let price = UNIT_PRICE / 30 * (renewal_days as u64) * (gib_count as u64);

			assert_ok!(FileBank::renewal_space(RuntimeOrigin::signed(acc1), renewal_days));
			let event = Sys::events().pop().expect("Expected at least one BuySpace to be found").event;
			assert_eq!(mock::RuntimeEvent::from(Event::RenewalSpace { acc: acc1, renewal_days, fee: price}), event);

			let uhsd = UserOwnedSpace::<Test>::try_get(acc1).unwrap();
			assert_eq!(2 * 1024 * 1_048_576, uhsd.total_space);
			assert_eq!(1, uhsd.start);
			assert_eq!(62 * 14400 + 1, uhsd.deadline);
    });
}

#[test]
fn upload_declaration() {
    new_test_ext().execute_with(|| {
			let acc1 = mock::account1();
			let file_name = "cess-book".as_bytes().to_vec();
			let file_hash = Hash([5u8; 64]);
			let bucket_name: Vec<u8> = "cess-bucket".as_bytes().to_vec();

			assert_noop!(upload_declaration_alias(acc1.clone(), file_name.clone(), file_hash.clone(), bucket_name.clone()), Error::<Test>::NonExistent);
			assert_ok!(create_new_bucket(acc1.clone(), bucket_name.clone()));

			assert_ok!(upload_declaration_alias(acc1, file_name, file_hash, bucket_name));
    });
}

#[test]
fn upload_works() {
    new_test_ext().execute_with(|| {
        let acc1 = mock::account1();
        let miner1 = mock::miner1();
        let stash1 = mock::stash1();
        let controller1 = mock::controller1();
        let mfi = MockingFileBankInfo::default();
				let bucket_name = "cess-bucket".as_bytes().to_vec();
				let file_name = "cess-book".as_bytes().to_vec();

				assert_ok!(create_new_bucket(acc1.clone(), bucket_name.clone()));
        assert_ok!(upload_declaration_alias(acc1, file_name.clone(), mfi.file_hash.clone(), bucket_name.clone()));
        assert_noop!(upload_file_alias(acc1, controller1, &mfi), Error::<Test>::ScheduleNonExistent);
        assert_ok!(register_miner(miner1));
        assert_ok!(register_scheduler(stash1.clone(), controller1.clone()));
        //upload() file not work have not buy space


        let space_gb = 20_u128;
        assert_ok!(Sminer::add_power(&miner1, 1_048_576 * 1024 * space_gb));
        assert_ok!(FileBank::buy_space(RuntimeOrigin::signed(acc1), 2));
        assert_ok!(add_power_for_miner(controller1, miner1));

        assert_ok!(upload_file_alias(acc1, controller1, &mfi));

        let file_hash = mfi.file_hash;
        let file_size = mfi.file_size;
        let file_slice_info = UserFileSliceInfo{
            file_hash: file_hash.clone(),
            file_size: file_size,
        };
        assert!(File::<Test>::contains_key(&file_hash));
        let t = UserOwnedSpace::<Test>::try_get(acc1).unwrap();
        assert_eq!(mfi.file_size as u128, t.used_space);
        assert_eq!(t.total_space - mfi.file_size as u128, t.remaining_space);
        assert!(UserHoldFileList::<Test>::try_get(acc1).unwrap().contains(&file_slice_info));

        let event = Sys::events().pop().expect("Expected at least one FileUpload to be found").event;
        assert_eq!(mock::RuntimeEvent::from(Event::FileUpload { acc: controller1 }), event);
    });
}

#[test]
fn upload_should_not_work_when_insufficient_storage() {
    new_test_ext().execute_with(|| {
        let acc1 = mock::account1();
        let stash1 = mock::stash1();
        let miner1 = mock::miner1();
        let controller1 = mock::controller1();
        let mut mfi = MockingFileBankInfo::default();
        let space_gb = 20_u128;
				let bucket_name = "cess-bucket".as_bytes().to_vec();
        assert_ok!(register_miner(miner1));
        assert_ok!(register_scheduler(stash1.clone(), controller1.clone()));
        assert_ok!(add_power_for_miner(controller1, miner1));
        assert_ok!(Sminer::add_power(&miner1, 1_048_576 * 1024 * space_gb));
        assert_ok!(FileBank::buy_space(RuntimeOrigin::signed(acc1), 2));
        mfi.file_size = 3 * 1024 * 1024 * 1024;  // large file

				assert_ok!(create_new_bucket(acc1.clone(), bucket_name.clone()));
				assert_ok!(upload_declaration_alias(acc1, "cess-book".as_bytes().to_vec(), mfi.file_hash, bucket_name.clone()));
        assert_noop!(upload_file_alias(acc1.clone(), controller1.clone(), &mfi), Error::<Test>::InsufficientStorage);

        if let Err(e) = upload_file_alias(acc1, controller1, &mfi) {
            if let DispatchError::Module(m) = e {
                assert_eq!("InsufficientStorage", m.message.unwrap());
            }
        }
    })
}

#[test]
fn delete_file_works() {
    new_test_ext().execute_with(|| {
        let acc1 = mock::account1();
        let stash1 = mock::stash1();
        let miner1 = mock::miner1();
        let controller1 = mock::controller1();
        let mfi = MockingFileBankInfo::default();
				let bucket_name = "cess-bucket".as_bytes().to_vec();
        assert_noop!(FileBank::delete_file(RuntimeOrigin::signed(acc1.clone()), acc1.clone(), mfi.file_hash.clone()), Error::<Test>::FileNonExistent);

        let space_gb = 20_u128;
        assert_ok!(register_miner(miner1.clone()));
        assert_ok!(Sminer::add_power(&miner1, 1_048_576 * 1024 * space_gb));
        assert_ok!(FileBank::buy_space(RuntimeOrigin::signed(acc1), 2));
        // acc1 upload file
        assert_ok!(register_scheduler(stash1.clone(), controller1.clone()));
				assert_ok!(create_new_bucket(acc1.clone(), bucket_name.clone()));
        assert_ok!(upload_declaration_alias(acc1, "cess-book".as_bytes().to_vec(), mfi.file_hash, bucket_name.clone()));
        assert_ok!(add_power_for_miner(controller1, miner1));
        assert_ok!(upload_file_alias(acc1, controller1, &mfi));
        assert_noop!(FileBank::delete_file(RuntimeOrigin::signed(mock::account2()), mock::account2(), mfi.file_hash.clone()), Error::<Test>::NotOwner);
        let ss_before = UserOwnedSpace::<Test>::try_get(acc1).unwrap();

        assert_ok!(FileBank::delete_file(RuntimeOrigin::signed(acc1.clone()), acc1.clone(), mfi.file_hash.clone()));

        let ss_after = UserOwnedSpace::<Test>::try_get(acc1).unwrap();
        assert!(!File::<Test>::contains_key(&mfi.file_hash));
        assert_ne!(ss_before.remaining_space, ss_after.remaining_space);

        let event = Sys::events().pop().expect("Expected at least one DeleteFile to be found").event;
        assert_eq!(mock::RuntimeEvent::from(Event::DeleteFile { operator: acc1.clone(), owner: acc1.clone(), file_hash: mfi.file_hash }), event);
    });
}

#[test]
fn upload_filler_work() {
    new_test_ext().execute_with(|| {
        let stash1 = mock::stash1();
        let miner1 = mock::miner1();
        let controller1 = mock::controller1();
        assert_noop!(add_power_for_miner(controller1.clone(), miner1.clone()), Error::<Test>::ScheduleNonExistent);
        assert_ok!(register_scheduler(stash1.clone(), controller1.clone()));
        assert_noop!(add_power_for_miner(controller1.clone(), miner1.clone()), pallet_sminer::Error::<Test>::NotMiner);
        assert_ok!(register_miner(miner1));
        assert_ok!(add_power_for_miner(controller1.clone(), miner1.clone()));

        let (power, _) = Sminer::get_power_and_space(miner1.clone()).unwrap();
        assert_eq!(1_048_576 * 8 * 10, power);
    });
}

#[test]
fn clear_invalid_file_work() {
    new_test_ext().execute_with(|| {
        let acc1 = mock::account1();
        let stash1 = mock::stash1();
        let miner1 = mock::miner1();
        let controller1 = mock::controller1();
        let mfi = MockingFileBankInfo::default();
				let bucket_name = "cess-bucket".as_bytes().to_vec();

        assert_ok!(register_miner(miner1.clone()));
        assert_ok!(Sminer::add_power(&miner1 ,1_048_576 * 1024 * 20));
        assert_ok!(register_scheduler(stash1.clone(), controller1.clone()));

				assert_ok!(create_new_bucket(acc1.clone(), bucket_name.clone()));
        assert_ok!(upload_declaration_alias(acc1, "cess-book".as_bytes().to_vec(), mfi.file_hash, bucket_name));
        assert_ok!(add_power_for_miner(controller1, miner1));
        assert_ok!(FileBank::buy_space(RuntimeOrigin::signed(acc1), 2));
        assert_ok!(upload_file_alias(acc1, controller1, &mfi));

        let mut file_hash_list = InvalidFile::<Test>::get(miner1.clone());
        assert_ok!(FileBank::clear_invalid_file(RuntimeOrigin::signed(miner1.clone()), file_hash_list[0]));
        file_hash_list.remove(0);
        assert_eq!(file_hash_list, InvalidFile::<Test>::get(miner1.clone()));
    });
}

#[test]
fn create_bucket_works() {
	new_test_ext().execute_with(|| {
		let acc1 = mock::account1();
		let bucket_name = "cess-bucket".as_bytes().to_vec();
		let bound_bucket_name: BoundedVec<u8, NameStrLimit> = bucket_name.try_into().unwrap();
		assert_ok!(FileBank::create_bucket(RuntimeOrigin::signed(acc1.clone()), acc1.clone(), bound_bucket_name.clone()));
		let bucket = Bucket::<Test>::get(&acc1, bound_bucket_name).unwrap();
		assert_eq!(bucket.authority.len(), 1);
		assert_eq!(bucket.authority[0], acc1);
	});
}

#[test]
fn delete_bucket_works() {
	new_test_ext().execute_with(|| {
		let acc1 = mock::account1();
		let bucket_name = "cess-bucket".as_bytes().to_vec();
		let bound_bucket_name: BoundedVec<u8, NameStrLimit> = bucket_name.try_into().unwrap();
		assert_ok!(FileBank::create_bucket(RuntimeOrigin::signed(acc1.clone()), acc1.clone(), bound_bucket_name.clone()));
		assert!(<Bucket<Test>>::contains_key(&acc1, bound_bucket_name.clone()));

		assert_ok!(FileBank::delete_bucket(RuntimeOrigin::signed(acc1.clone()), acc1.clone(), bound_bucket_name.clone()));
		assert!(!<Bucket<Test>>::contains_key(&acc1, bound_bucket_name.clone()));
	});
}

#[test]
fn transfer_ownership_works() {
	new_test_ext().execute_with(|| {
		let acc1 = account1();
		let acc2 = account2();
		let mfi = MockingFileBankInfo::default();
		let bucket_name = "cess-bucket".as_bytes().to_vec();
		let bound_bucket_name: BoundedVec<u8, NameStrLimit> = bucket_name.clone().try_into().unwrap();

		let stash1 = mock::stash1();
		let miner1 = mock::miner1();
		let controller1 = mock::controller1();

		assert_ok!(register_miner(miner1.clone()));
		assert_ok!(Sminer::add_power(&miner1 ,1_048_576 * 1024 * 20));
		assert_ok!(register_scheduler(stash1.clone(), controller1.clone()));
		assert_ok!(add_power_for_miner(controller1, miner1));

		assert_ok!(FileBank::buy_space(RuntimeOrigin::signed(acc1), 2));
		assert_ok!(FileBank::buy_space(RuntimeOrigin::signed(acc2), 2));

		assert_ok!(create_new_bucket(acc1.clone(), bucket_name.clone()));
		assert_ok!(create_new_bucket(acc2.clone(), bucket_name.clone()));
		assert_ok!(upload_declaration_alias(acc1, "cess-book".as_bytes().to_vec(), mfi.file_hash.clone(), bucket_name.clone()));
		assert_ok!(upload_file_alias(acc1, controller1, &mfi));

		let file = <File<Test>>::get(&mfi.file_hash).unwrap();
		assert_eq!(file.user_brief_list[0].user, acc1.clone());

		let target_brief = UserBrief::<Test> {
			user: acc2.clone(),
			file_name: "test-file2".as_bytes().to_vec().try_into().unwrap(),
			bucket_name: bound_bucket_name.clone(),
		};

		assert_ok!(FileBank::ownership_transfer(RuntimeOrigin::signed(acc1.clone()), bound_bucket_name, target_brief, mfi.file_hash.clone()));

		let file = <File<Test>>::get(&mfi.file_hash).unwrap();
		assert_eq!(file.user_brief_list[0].user, acc2.clone());
		assert!(!FileBank::check_is_file_owner(&acc1, &mfi.file_hash));
	})
}

#[test]
fn transfer_ownership_exception() {
	new_test_ext().execute_with(|| {
		let acc1 = account1();
		let acc2 = account2();
		let mfi = MockingFileBankInfo::default();
		let bucket_name = "cess-bucket".as_bytes().to_vec();
		let bound_bucket_name: BoundedVec<u8, NameStrLimit> = bucket_name.clone().try_into().unwrap();

		let stash1 = mock::stash1();
		let miner1 = mock::miner1();
		let controller1 = mock::controller1();

		assert_ok!(register_miner(miner1.clone()));
		assert_ok!(Sminer::add_power(&miner1 ,1_048_576 * 1024 * 20));
		assert_ok!(register_scheduler(stash1.clone(), controller1.clone()));
		assert_ok!(add_power_for_miner(controller1, miner1));

		assert_ok!(FileBank::buy_space(RuntimeOrigin::signed(acc1), 2));

		assert_ok!(create_new_bucket(acc1.clone(), bucket_name.clone()));
		assert_ok!(upload_declaration_alias(acc1, "cess-book".as_bytes().to_vec(), mfi.file_hash.clone(), bucket_name.clone()));
		assert_ok!(upload_file_alias(acc1, controller1, &mfi));

		let target_brief = UserBrief::<Test> {
			user: acc2.clone(),
			file_name: "test-file2".as_bytes().to_vec().try_into().unwrap(),
			bucket_name: bound_bucket_name.clone(),
		};

		assert_noop!(FileBank::ownership_transfer(RuntimeOrigin::signed(acc2.clone()), bound_bucket_name.clone(), target_brief.clone(), mfi.file_hash.clone()), Error::<Test>::NotOwner);
		let file_hash = Hash([8u8; 64]);
		assert_noop!(FileBank::ownership_transfer(RuntimeOrigin::signed(acc1.clone()), bound_bucket_name.clone(), target_brief.clone(), file_hash.clone()), Error::<Test>::FileNonExistent);
		assert_noop!(FileBank::ownership_transfer(RuntimeOrigin::signed(acc1.clone()), bound_bucket_name.clone(), target_brief.clone(), mfi.file_hash.clone()), Error::<Test>::NonExistent);

		assert_ok!(create_new_bucket(acc2.clone(), bucket_name.clone()));
		assert_noop!(FileBank::ownership_transfer(RuntimeOrigin::signed(acc1.clone()), bound_bucket_name.clone(), target_brief.clone(), mfi.file_hash.clone()), Error::<Test>::NotPurchasedSpace);
	})
}
// #[test]
// fn update_price_works() {
//     new_test_ext().execute_with(|| {
//         let acc1 = mock::account1();
//         assert_ok!(FileBank::update_price(Origin::signed(acc1), Vec::from("1000")));
//         assert_eq!(1000 / 3, UnitPrice::<Test>::try_get().unwrap());
//     });
// }
