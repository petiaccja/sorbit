use sorbit::{Deserialize, Serialize, ser_de::ToBytes};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct ImplicitMultiPass {
    #[sorbit(value=byte_count(c))]
    a: u8,
    c: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct MultiPass {
    #[sorbit(multi_pass)]
    inner: ImplicitMultiPass,
}

const MULTI_PASS_VALUE: MultiPass = MultiPass { inner: ImplicitMultiPass { a: 0, c: vec![] } };
const MULTI_PASS_BYTES: [u8; 1] = [0];

#[test]
fn serialize_multi_pass() {
    assert_eq!(MULTI_PASS_VALUE.to_bytes(), Ok(MULTI_PASS_BYTES.into()));
}
