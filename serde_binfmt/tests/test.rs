use serde_binfmt::{bit_field, serialize::Serialize, serializer::Serializer};

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
    // #[bits = 3]
    flags: u8,
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
        serializer.composite(|serializer| {
            serializer.serialize_u8(bit_field!(u8 => {(self.version, 0..4), (self.ihl, 4..8)}).unwrap())?;
            serializer.serialize_u8(bit_field!(u8 => {(self.dscp, 0..4), (self.ecn, 4..8)}).unwrap())?;
            serializer.serialize_u16(self.total_length)?;
            serializer.serialize_u16(self.identification)?;
            serializer
                .serialize_u16(bit_field!(u16 => {(self.flags, 0..3), (self.fragment_offset, 3..13)}).unwrap())?;
            Ok(())
        })
    }
}
