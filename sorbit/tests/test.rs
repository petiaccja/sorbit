use sorbit::byte_order::ByteOrder;
use sorbit::deserialize::{Deserialize, Deserializer, StreamDeserializer};
use sorbit::error::Error;
use sorbit::io::{FixedMemoryStream, GrowingMemoryStream, Read};
use sorbit::pack_bit_field;
use sorbit::serialize::{DeferredSerialize, DeferredSerializer, Section, Serialize, Serializer, StreamSerializer};
use sorbit::unpack_bit_field;

#[derive(Debug, PartialEq, Eq)]
// #[snap(4)]
struct IPv4Header {
    // #![declare_bit_field(b1: u8)]
    // #![declare_bit_field(b2: u8)]
    // #[bit_field(b1, 4..8)]
    version: u8,
    // #[bit_field(b1, 0..4)]
    // #[deferred(ihl(self))]
    ihl: u8,
    // #[bit_field(b2, 2..8)]
    dscp: u8,
    // #[bit_field(b2, 0..2)]
    ecn: u8,
    total_length: u16,
    identification: u16,
    // #[bit_field(b3: u16, 14)]
    dont_fragment: bool,
    // #[bit_field(b3, 13)]
    more_fragments: bool,
    // #[bit_field(b3, 0..13)]
    fragment_offset: u16,
    time_to_live: u8,
    protocol: u8,
    // #[deferred(checksum(self))]
    header_checksum: u16,
    source_address: u32,
    destination_address: u32,
}

impl IPv4Header {
    fn ihl(section: &impl Section) -> u8 {
        core::cmp::min(u8::MAX as u64, section.len()) as u8 / 4
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
}

impl DeferredSerialize for IPv4Header {
    fn serialize<S: DeferredSerializer>(&self, serializer: &mut S) -> Result<S::Success, S::Error> {
        let mut b1_section = None;
        let mut checksum_section = None;
        let self_section = serializer.with_byte_order(ByteOrder::BigEndian, |s| {
            s.serialize_composite(|s| {
                b1_section = 0u8.serialize(s)?.into(); // Version and IHL.
                pack_bit_field!(u8 => {(self.dscp, 2..8), (self.ecn, 0..2)}).unwrap().serialize(s)?;
                self.total_length.serialize(s)?;
                self.identification.serialize(s)?;
                pack_bit_field!(u16 => {
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
                self.destination_address.serialize(s)?;
                s.align(4)
            })
        })?;
        // Update IHL.
        {
            let ihl = Self::ihl(&self_section);
            serializer.update_section(&b1_section.as_ref().unwrap(), |s| {
                pack_bit_field!(u8 => {(self.version, 4..8), (ihl, 0..4)}).unwrap().serialize(s)
            })?;
        }
        // Update checksum.
        {
            let checksum = serializer.analyze_section(&self_section, |reader| Self::checksum(reader))?;
            serializer.update_section(&checksum_section.as_ref().unwrap(), |s| {
                s.with_byte_order(ByteOrder::BigEndian, |s| checksum.serialize(s))
            })?;
        }
        Ok(self_section)
    }
}

impl Deserialize for IPv4Header {
    fn deserialize<D: Deserializer>(deserializer: &mut D) -> Result<Self, D::Error> {
        deserializer.with_byte_order(ByteOrder::BigEndian, |s| {
            s.deserialize_composite(|s| {
                let (version, ihl) = unpack_bit_field!(u8::deserialize(s)? => { (u8,  4..8), (u8, 0..4) }).unwrap();
                let (dscp, ecn) = unpack_bit_field!(u8::deserialize(s)? => {(u8, 2..8), (u8, 0..2)}).unwrap();
                let total_length = u16::deserialize(s)?;
                let identification = u16::deserialize(s)?;
                let (dont_fragment, more_fragments, fragment_offset) =
                    unpack_bit_field!(u16::deserialize(s)? => { (bool, 14..=14), (bool, 13..=13), (u16, 0..13)})
                        .unwrap();
                let time_to_live = u8::deserialize(s)?;
                let protocol = u8::deserialize(s)?;
                let header_checksum = u16::deserialize(s)?;
                let source_address = u32::deserialize(s)?;
                let destination_address = u32::deserialize(s)?;
                s.align(4)?;
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

const EXAMPLE_IPV4_HEADER: IPv4Header = IPv4Header {
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
const EXAMPLE_IPV4_BYTES : [u8; 20] =    [
    0x45, 0x00, 0x06, 0x00,
    0x00, 0x00, 0b0010_0000, 0x00,
    0x0C, 0x11, 0xDE, 0xEE,
    0x73, 0x45, 0x78, 0x23,
    0x88, 0x36, 0x36, 0x60
];

#[test]
fn serialize_ipv4_header() -> Result<(), Error> {
    let mut s = StreamSerializer::new(GrowingMemoryStream::new());
    EXAMPLE_IPV4_HEADER.serialize(&mut s)?;
    assert_eq!(&s.take().take(), &EXAMPLE_IPV4_BYTES);
    Ok(())
}

#[test]
fn deserialize_ipv4_header() {
    let mut s = StreamDeserializer::new(FixedMemoryStream::new(&EXAMPLE_IPV4_BYTES));
    assert_eq!(IPv4Header::deserialize(&mut s), Ok(EXAMPLE_IPV4_HEADER));
}
