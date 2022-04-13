use super::*;

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct FileInfo<BoundedString, BoundedStringList> {
    pub(super) filename: BoundedString,
    pub(super) filesize: u128,
    pub(super) keywords: BoundedStringList,
}

impl<BoundedString, BoundedStringList> FileInfo<BoundedString, BoundedStringList> {
    pub fn new(pfilename: BoundedString, filesize: u128, pkeywords: BoundedStringList) -> FileInfo<BoundedString, BoundedStringList> {
        let file = FileInfo::<BoundedString, BoundedStringList> {
            filename: pfilename,
            filesize: filesize,
            keywords: pkeywords,
        };
        file
    }
}

#[derive(PartialEq, Eq, Copy, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct TestInfo<AccountId>{
    pub(super) owner: AccountId,
    pub(super) filename: [u8; 32],
    pub(super) filesize: u32,
}


