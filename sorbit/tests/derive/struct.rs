use sorbit::deserialize::Deserialize;
use sorbit::error::Error;
use sorbit::io::GrowingMemoryStream;
use sorbit::serialize::{Serialize, StreamSerializer};
use sorbit::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Empty {}

#[derive(Debug, Serialize, Deserialize)]
struct Generic<T: Serialize + Deserialize> {
    value: T,
}

#[derive(Debug, Serialize, Deserialize)]
struct WithoutLayout {
    a: u8,
    b: u8,
}

#[derive(Debug, Serialize, Deserialize)]
#[sorbit_layout(len = 12)]
struct WithLen {
    a: u8,
    b: u8,
}

#[derive(Debug, Serialize, Deserialize)]
#[sorbit_layout(round = 8)]
struct WithRounding {
    a: [u8; 10],
}

#[derive(Debug, Serialize, Deserialize)]
struct WithDirectFields {
    #[sorbit_layout(offset = 2)]
    a: u8,
    #[sorbit_layout(align = 4, round = 2)]
    b: u8,
}

#[derive(Debug, Serialize, Deserialize)]
#[sorbit_bit_field(_b, repr(u16))]
struct WithBitFields {
    #[sorbit_bit_field(_b, bits(4..10))]
    a: u8,
    #[sorbit_bit_field(_b, bits(14))]
    b: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct WithBitFieldWithLayout {
    #[sorbit_bit_field(_b, repr(u16), offset = 2, round = 4, bits(4..10))]
    a: u8,
}

#[test]
fn serialize_empty() -> Result<(), Error> {
    let input = Empty {};
    let mut serializer = StreamSerializer::new(GrowingMemoryStream::new());
    input.serialize(&mut serializer)?;
    assert_eq!(serializer.take().take(), vec![]);
    Ok(())
}

#[test]
fn serialize_generic() -> Result<(), Error> {
    let input = Generic { value: -72i32 };
    let mut serializer = StreamSerializer::new(GrowingMemoryStream::new());
    input.serialize(&mut serializer)?;
    assert_eq!(serializer.take().take(), (-72i32).cast_unsigned().to_be_bytes());
    Ok(())
}

#[test]
fn serialize_with_len() -> Result<(), Error> {
    let input = WithLen { a: 83, b: 8 };
    let mut serializer = StreamSerializer::new(GrowingMemoryStream::new());
    input.serialize(&mut serializer)?;
    assert_eq!(serializer.take().take(), vec![83, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    Ok(())
}

#[test]
fn serialize_with_rounding() -> Result<(), Error> {
    let input = WithRounding { a: [3; 10] };
    let mut serializer = StreamSerializer::new(GrowingMemoryStream::new());
    input.serialize(&mut serializer)?;
    assert_eq!(serializer.take().take(), vec![3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 0, 0, 0, 0, 0, 0]);
    Ok(())
}

#[test]
fn serialize_without_layout() -> Result<(), Error> {
    let input = WithoutLayout { a: 0x03, b: 0x12 };
    let mut serializer = StreamSerializer::new(GrowingMemoryStream::new());
    input.serialize(&mut serializer)?;
    assert_eq!(serializer.take().take(), vec![0x03, 0x12]);
    Ok(())
}

#[test]
fn serialize_direct_fields() -> Result<(), Error> {
    let input = WithDirectFields { a: 0x03, b: 0x12 };
    let mut serializer = StreamSerializer::new(GrowingMemoryStream::new());
    input.serialize(&mut serializer)?;
    assert_eq!(serializer.take().take(), vec![0x00, 0x00, 0x03, 0x00, 0x12, 0x00]);
    Ok(())
}

#[test]
fn serialize_bit_fields() -> Result<(), Error> {
    let input = WithBitFields { a: 0b110011, b: true };
    let mut serializer = StreamSerializer::new(GrowingMemoryStream::new());
    input.serialize(&mut serializer)?;
    assert_eq!(serializer.take().take(), 0b0100_0011_0011_0000_u16.to_be_bytes());
    Ok(())
}

#[test]
fn serialize_bit_field_with_layout() -> Result<(), Error> {
    let input = WithBitFieldWithLayout { a: 0b110011 };
    let mut serializer = StreamSerializer::new(GrowingMemoryStream::new());
    input.serialize(&mut serializer)?;
    assert_eq!(serializer.take().take(), &[0u8, 0u8, 0b0000_0011_u8, 0b0011_0000_u8, 0u8, 0u8]);
    Ok(())
}
