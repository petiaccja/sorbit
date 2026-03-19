use sorbit::{
    Deserialize, Serialize,
    ser_de::{FromBytes, ToBytes},
};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[sorbit(byte_order=big_endian)]
struct ByByteCount {
    #[sorbit(value=byte_count(collection))]
    byte_count: u16,
    collection: Vec<u16>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[sorbit(byte_order=big_endian)]
struct ByByteCountBit {
    #[sorbit(bit_field=b, repr=u16, bits=0..12)]
    #[sorbit(value=byte_count(collection_1))]
    byte_count_1: u16,
    #[sorbit(bit_field=b, bits=12..16)]
    #[sorbit(value=byte_count(collection_2))]
    byte_count_2: u8,

    collection_1: Vec<u16>,
    collection_2: Vec<u16>,
}

fn by_byte_count_value(synchronize_len: bool) -> ByByteCount {
    ByByteCount { byte_count: if synchronize_len { 4 } else { 0 }, collection: vec![1, 2] }
}
const BY_BYTE_COUNT_BYTES: [u8; 6] = [0, 4, 0, 1, 0, 2];

fn by_byte_count_value_bit(synchronize_len: bool) -> ByByteCountBit {
    ByByteCountBit {
        byte_count_1: if synchronize_len { 4 } else { 0 },
        byte_count_2: if synchronize_len { 6 } else { 0 },
        collection_1: vec![1, 2],
        collection_2: vec![3, 4, 5],
    }
}
const BY_BYTE_COUNT_BIT_BYTES: [u8; 12] = [0b0110_0000, 0b0000_0100, 0, 1, 0, 2, 0, 3, 0, 4, 0, 5];

#[test]
fn serialize() {
    assert_eq!(by_byte_count_value(false).to_bytes(), Ok(BY_BYTE_COUNT_BYTES.into()));
}

#[test]
fn deserialize() {
    assert_eq!(ByByteCount::from_bytes(&BY_BYTE_COUNT_BYTES), Ok(by_byte_count_value(true)));
}

#[test]
fn serialize_bit() {
    assert_eq!(by_byte_count_value_bit(false).to_bytes(), Ok(BY_BYTE_COUNT_BIT_BYTES.into()));
}

#[test]
fn deserialize_bit() {
    assert_eq!(ByByteCountBit::from_bytes(&BY_BYTE_COUNT_BIT_BYTES), Ok(by_byte_count_value_bit(true)));
}
