use sorbit::error::Error;
use sorbit::io::GrowingMemoryStream;
use sorbit::serialize::{Serialize as _, StreamSerializer};
use sorbit::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Empty {}

#[derive(Debug, Serialize, Deserialize)]
struct Unconstrained {
    a: u8,
    b: u8,
}

#[derive(Debug, Serialize, Deserialize)]
struct DirectFields {
    #[sorbit_layout(offset = 2)]
    a: u8,
    #[sorbit_layout(align = 4, round = 2)]
    b: u8,
}

#[derive(Debug, Serialize, Deserialize)]
#[sorbit_bit_field(_b, repr(u16))]
struct BitFields {
    #[sorbit_bit_field(_b, bits(4..10))]
    a: u8,
    #[sorbit_bit_field(_b, bits(14))]
    b: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct BitFieldWithLayout {
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
fn serialize_unconstrained() -> Result<(), Error> {
    let input = Unconstrained { a: 0x03, b: 0x12 };
    let mut serializer = StreamSerializer::new(GrowingMemoryStream::new());
    input.serialize(&mut serializer)?;
    assert_eq!(serializer.take().take(), vec![0x03, 0x12]);
    Ok(())
}

#[test]
fn serialize_direct_fields() -> Result<(), Error> {
    let input = DirectFields { a: 0x03, b: 0x12 };
    let mut serializer = StreamSerializer::new(GrowingMemoryStream::new());
    input.serialize(&mut serializer)?;
    assert_eq!(serializer.take().take(), vec![0x00, 0x00, 0x03, 0x00, 0x12, 0x00]);
    Ok(())
}

#[test]
fn serialize_bit_fields() -> Result<(), Error> {
    let input = BitFields { a: 0b110011, b: true };
    let mut serializer = StreamSerializer::new(GrowingMemoryStream::new());
    input.serialize(&mut serializer)?;
    assert_eq!(serializer.take().take(), 0b0100_0011_0011_0000_u16.to_be_bytes());
    Ok(())
}

#[test]
fn serialize_bit_field_with_layout() -> Result<(), Error> {
    let input = BitFieldWithLayout { a: 0b110011 };
    let mut serializer = StreamSerializer::new(GrowingMemoryStream::new());
    input.serialize(&mut serializer)?;
    assert_eq!(serializer.take().take(), &[0u8, 0u8, 0b0000_0011_u8, 0b0011_0000_u8, 0u8, 0u8]);
    Ok(())
}
