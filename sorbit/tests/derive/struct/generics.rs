use crate::utility::{from_bytes, to_bytes};
use sorbit::ser_de::{Deserialize, Serialize};
use sorbit::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[sorbit(byte_order = big_endian)]
struct Generic<T: Serialize + Deserialize + PartialEq> {
    value: T,
}

const VALUE: Generic<i32> = Generic::<i32> { value: -72 };
const BYTES: [u8; 4] = (-72i32).cast_unsigned().to_be_bytes();

#[test]
fn serialize() {
    assert_eq!(to_bytes(&VALUE), Ok(BYTES.into()));
}

#[test]
fn deserialize() {
    assert_eq!(from_bytes::<Generic<i32>>(&BYTES), Ok(VALUE));
}
