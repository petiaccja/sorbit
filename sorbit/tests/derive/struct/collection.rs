use crate::utility::to_bytes;
use sorbit::Serialize;

#[derive(Debug, Serialize, PartialEq)]
#[sorbit(byte_order=big_endian)]
struct ByLength {
    #[sorbit(value=len(collection))]
    len: u16,
    collection: [u8; 4],
}

#[derive(Debug, Serialize, PartialEq)]
#[sorbit(byte_order=big_endian)]
struct ByLengthBit {
    #[sorbit(bit_field=_0, repr=u8, bits=0..=4)]
    #[sorbit(value=len(collection))]
    len: u8,
    collection: [u8; 4],
}

#[derive(Debug, Serialize, PartialEq)]
#[sorbit(byte_order=big_endian)]
struct ByLengthOverflow {
    #[sorbit(bit_field=_0, repr=u8, bits=0..=1)]
    #[sorbit(value=len(collection))]
    len: u8,
    collection: [u8; 4],
}

const BY_LENGTH_VALUE: ByLength = ByLength { len: 0, collection: [1, 2, 3, 4] };
const BY_LENGTH_BYTES: [u8; 6] = [0, 4, 1, 2, 3, 4];

const BY_LENGTH_BIT_VALUE: ByLengthBit = ByLengthBit { len: 0, collection: [1, 2, 3, 4] };
const BY_LENGTH_BIT_BYTES: [u8; 5] = [4, 1, 2, 3, 4];

const BY_LENGTH_OVERFLOW_VALUE: ByLengthOverflow = ByLengthOverflow { len: 0, collection: [1, 2, 3, 4] };

#[test]
fn serialize() {
    assert_eq!(to_bytes(&BY_LENGTH_VALUE), Ok(BY_LENGTH_BYTES.into()));
}

#[test]
fn serialize_bit() {
    assert_eq!(to_bytes(&BY_LENGTH_BIT_VALUE), Ok(BY_LENGTH_BIT_BYTES.into()));
}

#[test]
fn serialize_overflow() {
    assert!(to_bytes(&BY_LENGTH_OVERFLOW_VALUE).is_err());
}
