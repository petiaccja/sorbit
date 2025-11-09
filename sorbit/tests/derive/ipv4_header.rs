use sorbit::{Deserialize, Serialize};

use crate::utility::{from_bytes, to_bytes};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[sorbit_layout(round = 4)]
struct IPv4Header {
    #[sorbit_bit_field(_version_ihl, repr(u8), bits(4..8))]
    version: u8,
    #[sorbit_bit_field(_version_ihl, bits(0..4))]
    ihl: u8, // defer: ihl(self)
    #[sorbit_bit_field(_dscp_ecn, repr(u8), bits(2..8))]
    dscp: u8,
    #[sorbit_bit_field(_dscp_ecn, bits(0..2))]
    ecn: u8,
    total_length: u16,
    identification: u16,
    #[sorbit_bit_field(_flags_fo, repr(u16), bits(14))]
    dont_fragment: bool,
    #[sorbit_bit_field(_flags_fo, bits(13))]
    more_fragments: bool,
    #[sorbit_bit_field(_flags_fo, bits(0..13))]
    fragment_offset: u16,
    time_to_live: u8,
    protocol: u8,
    header_checksum: u16, // defer: checksum(self)
    source_address: u32,
    destination_address: u32,
}

const IPV4_VALUE: IPv4Header = IPv4Header {
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
const IPV4_BYTES : [u8; 20] =    [
    0x45, 0x00, 0x06, 0x00,
    0x00, 0x00, 0b0010_0000, 0x00,
    0x0C, 0x11, 0xDE, 0xEE,
    0x73, 0x45, 0x78, 0x23,
    0x88, 0x36, 0x36, 0x60
];

#[test]
fn serialize() {
    assert_eq!(to_bytes(&IPV4_VALUE), Ok(IPV4_BYTES.into()));
}

#[test]
fn deserialize() {
    assert_eq!(from_bytes::<IPv4Header>(&IPV4_BYTES), Ok(IPV4_VALUE));
}
