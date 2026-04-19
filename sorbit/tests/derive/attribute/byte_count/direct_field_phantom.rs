use std::marker::PhantomData;

use sorbit::{
    Deserialize, Serialize,
    ser_de::{FromBytes, ToBytes},
};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Test {
    #[sorbit(value = byte_count(collection))]
    length: PhantomData<u16>,
    collection: Vec<u8>,
}

#[test]
fn roundtrip() {
    let value = Test { length: PhantomData, collection: vec![0xCC; 2] };
    let bytes = vec![0x00, 0x02, 0xCC, 0xCC];
    assert_eq!(value.to_be_bytes().as_ref(), Ok(&bytes));
    assert_eq!(Test::from_be_bytes(&bytes), Ok(value));
}

#[test]
fn serialize_mismatched_length() {
    let value = Test { length: PhantomData, collection: vec![0xCC; 2] };
    let bytes = vec![0x00, 0x02, 0xCC, 0xCC];
    assert_eq!(value.to_be_bytes().as_ref(), Ok(&bytes));
}

#[test]
fn serialize_loss_of_precision() {
    let value = Test { length: PhantomData, collection: vec![0xCC; 65536] };
    assert!(value.to_be_bytes().is_err());
}

#[test]
fn deserialize_eof() {
    let bytes = vec![0x00, 0x02, 0xCC];
    assert!(Test::from_be_bytes(&bytes).is_err());
}
