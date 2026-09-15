use deku::prelude::*;

#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
#[deku(ctx = "file_count: i32")]
pub struct Buckets {
    pub hash_offset: u64,
    pub bucket_count: i32,
    #[deku(assert_eq = "0x10")]
    pub bucket_header_size: u8,
    #[deku(assert_eq = "8")]
    pub bucket_size: u8,
    #[deku(assert_eq = "8")]
    pub hash_size: u8,
    #[deku(assert_eq = "0")]
    unk0xf: u8,
    #[deku(count = "*bucket_count")]
    pub buckets: Vec<(u32, u32)>,
    #[deku(count = "file_count")]
    pub hashes: Vec<(u32, u32)>,
}
