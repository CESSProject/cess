use crate::{mock::*, types::*};
use frame_support::{assert_ok};
use frame_support::storage::bounded_vec::BoundedVec;

#[test]
fn store_work() {
	new_test_ext().execute_with(|| {
		let pfileid: Vec<u8> = vec![0,1,2,3,4,5,6,7,8];
		let pfilename: Vec<u8> = vec![0,1,2,3,4,5,6,7,8];
		let pkeywords: Vec<Vec<u8>> = vec![vec![0,1,2,3,4,5,6,7,8,9]];
		let fileid: BoundedVec<u8, crate::mock::StringLimit> = pfileid.clone().try_into().expect("fileid too long");
		let filename: BoundedVec<u8, crate::mock::StringLimit> = pfilename.clone().try_into().expect("filename too long");
		let keywords = DataStore::vec_to_bounde(pkeywords.clone()).unwrap();
		let file = FileInfo::<BoundedVec<u8, crate::mock::StringLimit>, crate::StringList<Test>>::new(filename, 200, keywords);
		// Dispatch a signed extrinsic.
		assert_ok!(DataStore::store(Origin::signed(1), pfileid, pfilename, 200, pkeywords));
		// Read pallet storage and assert an expected result.
		let result = DataStore::file_storage(1, fileid).unwrap();
		assert_eq!(result, file);
	});
}

#[test]
fn retrieve_when_owner() {
	new_test_ext().execute_with(|| {
		// Ensure the expected error is thrown when no value is present.
		let pfileid: Vec<u8> = vec![0,1,2,3,4,5,6,7,8];
		let pfilename: Vec<u8> = vec![0,1,2,3,4,5,6,7,8];
		let pkeywords: Vec<Vec<u8>> = vec![vec![0,1,2,3,4,5,6,7,8,9]];
		
		// Dispatch a signed extrinsic.
		assert_ok!(DataStore::store(Origin::signed(1), pfileid.clone(), pfilename, 200, pkeywords));
	});
}

#[test]
fn replace_work() {
	new_test_ext().execute_with(|| {
		let pfileid: Vec<u8> = vec![0,1,2,3,4,5,6,7,8];
		let pfilename: Vec<u8> = vec![0,1,2,3,4,5,6,7,8];
		let pkeywords: Vec<Vec<u8>> = vec![vec![0,1,2,3,4,5,6,7,8,9]];
		assert_ok!(DataStore::store(Origin::signed(1), pfileid.clone(), pfilename, 200, pkeywords));

		let new_fileid: Vec<u8> = vec![14,15,28,37,9,85,6,79,158];
		let new_filename: Vec<u8> = vec![0,1,2,3,4,5,6,7,8];
		let new_keywords: Vec<Vec<u8>> = vec![vec![0,1,2,3,4,5,6,7,8,9]];
		assert_ok!(DataStore::replace(Origin::signed(1), pfileid, new_fileid.clone(), new_filename.clone(), 200, new_keywords.clone()));

		let fileid: BoundedVec<u8, crate::mock::StringLimit> = new_fileid.clone().try_into().expect("fileid too long");
		let filename: BoundedVec<u8, crate::mock::StringLimit> = new_filename.clone().try_into().expect("filename too long");
		let keywords = DataStore::vec_to_bounde(new_keywords.clone()).unwrap();
		let file = FileInfo::<BoundedVec<u8, crate::mock::StringLimit>, crate::StringList<Test>>::new(filename, 200, keywords);

		let result = DataStore::file_storage(1, fileid).unwrap();
		assert_eq!(result, file);
	});
}

#[test]
fn delete_work() {
	new_test_ext().execute_with(|| {
		let pfileid: Vec<u8> = vec![0,1,2,3,4,5,6,7,8];
		let pfilename: Vec<u8> = vec![0,1,2,3,4,5,6,7,8];
		let pkeywords: Vec<Vec<u8>> = vec![vec![0,1,2,3,4,5,6,7,8,9]];
		assert_ok!(DataStore::store(Origin::signed(1), pfileid.clone(), pfilename, 200, pkeywords));

		assert_ok!(DataStore::delete(Origin::signed(1), pfileid));
	});
}

#[test]
fn edit_work() {
	new_test_ext().execute_with(|| {
		let pfileid: Vec<u8> = vec![0,1,2,3,4,5,6,7,8];
		let pfilename: Vec<u8> = vec![0,1,2,3,4,5,6,7,8];
		let pkeywords: Vec<Vec<u8>> = vec![vec![0,1,2,3,4,5,6,7,8,9]];
		assert_ok!(DataStore::store(Origin::signed(1), pfileid.clone(), pfilename, 200, pkeywords));

		assert_ok!(DataStore::delete(Origin::signed(1), pfileid.clone()));

		let new_filename: Vec<u8> = vec![0,1,2,3,4,5,6,7,8];
		let new_keywords: Vec<Vec<u8>> = vec![vec![0,1,2,3,4,5,6,7,8,9]];
		assert_ok!(DataStore::edit(Origin::signed(1), pfileid.clone(), new_filename.clone(), new_keywords.clone()));

		let fileid: BoundedVec<u8, crate::mock::StringLimit> = pfileid.clone().try_into().expect("fileid too long");
		let filename: BoundedVec<u8, crate::mock::StringLimit> = new_filename.clone().try_into().expect("filename too long");
		let keywords = DataStore::vec_to_bounde(new_keywords.clone()).unwrap();
		let file = FileInfo::<BoundedVec<u8, crate::mock::StringLimit>, crate::StringList<Test>>::new(filename, 200, keywords);

		let result = DataStore::file_storage(1, fileid).unwrap();
		assert_eq!(result, file);
	});
}