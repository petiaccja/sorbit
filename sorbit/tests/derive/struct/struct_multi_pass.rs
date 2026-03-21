use sorbit::{Deserialize, Serialize, ser_de::ToBytes};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct ImplicitMultiPass {
    #[sorbit(value=byte_count(c))]
    a: u8,
    c: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct MultiPassSingle {
    #[sorbit(multi_pass)]
    inner: ImplicitMultiPass,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct MultiPassCollection {
    #[sorbit(value=len(collection))]
    length: u8,
    #[sorbit(multi_pass)]
    collection: Vec<ImplicitMultiPass>,
}

const MULTI_PASS_SINGLE_VALUE: MultiPassSingle = MultiPassSingle { inner: ImplicitMultiPass { a: 0, c: vec![] } };
const MULTI_PASS_SINGLE_BYTES: [u8; 1] = [0];

const MULTI_PASS_COLLECTION_VALUE: MultiPassCollection = MultiPassCollection { length: 0, collection: vec![] };
const MULTI_PASS_COLLECTION_BYTES: [u8; 1] = [0];

#[test]
fn serialize_multi_pass_single() {
    assert_eq!(MULTI_PASS_SINGLE_VALUE.to_bytes(), Ok(MULTI_PASS_SINGLE_BYTES.into()));
}

#[test]
fn serialize_multi_pass_collection() {
    assert_eq!(MULTI_PASS_COLLECTION_VALUE.to_bytes(), Ok(MULTI_PASS_COLLECTION_BYTES.into()));
}
