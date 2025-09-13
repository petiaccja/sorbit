use serde_binfmt::{
    bit_field,
    byte_order::ByteOrder,
    error::Error,
    io::GrowingMemoryStream,
    serialize::{Serialize, Serializer, StreamSerializer},
};

#[allow(unused)]
struct IPv4Header {
    // #[bits = 4]
    version: u8,
    // #[bits = 4]
    ihl: u8,
    // #[bits = 6]
    dscp: u8,
    // #[bits = 2]
    ecn: u8,
    total_length: u16,
    identification: u16,
    // #[bits = 1]
    dont_fragment: bool,
    // #[bits = 1]
    more_fragments: bool,
    // #[bits = 13]
    fragment_offset: u16,
    time_to_live: u8,
    protocol: u8,
    header_checksum: u16, // Checksum calculation?
    source_address: u32,
    destination_address: u32,
}

impl Serialize for IPv4Header {
    fn serialize<S: Serializer>(&self, serializer: &mut S) -> Result<(), S::Error> {
        serializer.change_byte_order(ByteOrder::BigEndian, |s| {
            s.serialize_composite(|s| {
                s.serialize_u8(bit_field!(u8 => {(self.version, 4..8), (self.ihl, 0..4)}).unwrap())?;
                s.serialize_u8(bit_field!(u8 => {(self.dscp, 0..4), (self.ecn, 4..8)}).unwrap())?;
                s.serialize_u16(self.total_length)?;
                s.serialize_u16(self.identification)?;
                s.serialize_u16(
                    bit_field!(u16 => {
                        (self.dont_fragment, 14..=14),
                        (self.more_fragments, 13..=13),
                        (self.fragment_offset, 0..13)
                    })
                    .unwrap(),
                )?;
                s.serialize_u8(self.time_to_live)?;
                s.serialize_u8(self.protocol)?;
                s.serialize_u16(self.header_checksum)?;
                s.serialize_u32(self.source_address)?;
                s.serialize_u32(self.destination_address)?;
                Ok(())
            })
        })
    }
}

#[test]
fn serialize_ipv4_header() -> Result<(), Error> {
    let header = IPv4Header {
        version: 4,
        ihl: 5,
        dscp: 0,
        ecn: 0,
        total_length: 1536,
        identification: 0,
        dont_fragment: false,
        more_fragments: true,
        fragment_offset: 0,
        time_to_live: 12,
        protocol: 17,
        header_checksum: 0x00,
        source_address: 0x73457823,
        destination_address: 0x88363660,
    };

    #[rustfmt::skip]
    let expected = [
        0x45, 0x00, 0x06, 0x00,
        0x00, 0x00, 0b0010_0000, 0x00,
        0x0C, 0x11, 0x00, 0x00,
        0x73, 0x45, 0x78, 0x23,
        0x88, 0x36, 0x36, 0x60
    ];

    let mut s = StreamSerializer::new(GrowingMemoryStream::new());
    header.serialize(&mut s)?;
    assert_eq!(&s.take().take(), &expected);
    Ok(())
}
