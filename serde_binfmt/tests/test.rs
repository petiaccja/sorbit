use serde_binfmt::{
    bit_field,
    byte_order::ByteOrder,
    deserialize::{Deserialize, Deserializer, StreamDeserializer},
    error::Error,
    io::{FixedMemoryStream, GrowingMemoryStream, Read},
    serialize::{DeferredSerialize, DeferredSerializer, Serialize, Serializer, StreamSerializer},
    unpack,
};

#[derive(Debug, PartialEq, Eq)]
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
    // #[deferred(checksum(self))]
    header_checksum: u16,
    source_address: u32,
    destination_address: u32,
}

fn checksum(mut reader: impl Read) -> u16 {
    let mut checksum = 0u32;
    let mut bytes = [0u8; 2];
    while let Ok(_) = reader.read(bytes.as_mut_slice()) {
        let word = u16::from_be_bytes(bytes);
        checksum += word as u32;
        checksum = (checksum >> 16) + (checksum & 0xFFFF);
    }
    !(checksum as u16)
}

impl DeferredSerialize for IPv4Header {
    fn serialize<S: DeferredSerializer>(&self, serializer: &mut S) -> Result<S::Success, S::Error> {
        let mut checksum_section = None;
        let composite_section = serializer.with_byte_order(ByteOrder::BigEndian, |s| {
            s.serialize_composite(|s| {
                bit_field!(u8 => {(self.version, 4..8), (self.ihl, 0..4)}).unwrap().serialize(s)?;
                bit_field!(u8 => {(self.dscp, 4..8), (self.ecn, 0..4)}).unwrap().serialize(s)?;
                self.total_length.serialize(s)?;
                self.identification.serialize(s)?;
                bit_field!(u16 => {
                    (self.dont_fragment, 14..=14),
                    (self.more_fragments, 13..=13),
                    (self.fragment_offset, 0..13)
                })
                .unwrap()
                .serialize(s)?;
                self.time_to_live.serialize(s)?;
                self.protocol.serialize(s)?;
                checksum_section = 0u16.serialize(s)?.into();
                self.source_address.serialize(s)?;
                self.destination_address.serialize(s)
            })
        })?;
        let checksum = serializer.analyze_section(&composite_section, |reader| checksum(reader))?;
        serializer.update_section(&checksum_section.as_ref().unwrap(), |s| {
            s.with_byte_order(ByteOrder::BigEndian, |s| checksum.serialize(s))
        })?;
        Ok(composite_section)
    }
}

impl Deserialize for IPv4Header {
    fn deserialize<D: Deserializer>(deserializer: &mut D) -> Result<Self, D::Error> {
        deserializer.with_byte_order(ByteOrder::BigEndian, |s| {
            s.deserialize_composite(|s| {
                let (version, ihl) = unpack!(u8::deserialize(s)? => { (u8,  4..8), (u8, 0..4) }).unwrap();
                let (dscp, ecn) = unpack!(u8::deserialize(s)? => {(u8, 4..8), (u8, 0..4)}).unwrap();
                let total_length = u16::deserialize(s)?;
                let identification = u16::deserialize(s)?;
                let (dont_fragment, more_fragments, fragment_offset) =
                    unpack!(u16::deserialize(s)? => { (bool, 14..=14), (bool, 13..=13), (u16, 0..13)}).unwrap();
                let time_to_live = u8::deserialize(s)?;
                let protocol = u8::deserialize(s)?;
                let header_checksum = u16::deserialize(s)?;
                let source_address = u32::deserialize(s)?;
                let destination_address = u32::deserialize(s)?;
                Ok(IPv4Header {
                    version,
                    ihl,
                    dscp,
                    ecn,
                    total_length,
                    identification,
                    dont_fragment,
                    more_fragments,
                    fragment_offset,
                    time_to_live,
                    protocol,
                    header_checksum,
                    source_address,
                    destination_address,
                })
            })
        })
    }
}

#[test]
fn serialize_ipv4_header() -> Result<(), Error> {
    let value = IPv4Header {
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
        header_checksum: 0xDEEE,
        source_address: 0x73457823,
        destination_address: 0x88363660,
    };

    #[rustfmt::skip]
    let bytes = [
        0x45, 0x00, 0x06, 0x00,
        0x00, 0x00, 0b0010_0000, 0x00,
        0x0C, 0x11, 0xDE, 0xEE,
        0x73, 0x45, 0x78, 0x23,
        0x88, 0x36, 0x36, 0x60
    ];
    {
        let mut s = StreamSerializer::new(GrowingMemoryStream::new());
        value.serialize(&mut s)?;
        assert_eq!(&s.take().take(), &bytes);
    }
    {
        let mut s = StreamDeserializer::new(FixedMemoryStream::new(&bytes));
        assert_eq!(IPv4Header::deserialize(&mut s), Ok(value));
    }
    Ok(())
}
