use serde_binfmt::{
    serialize::Serialize,
    serializer::{BitFieldSerializer as _, Serializer, StructSerializer as _},
};

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
    header_checksum: u16,
    source_address: u32,
    destination_address: u32,
}

impl Serialize for IPv4Header {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<(), S::Error> {
        serializer.serialize_struct(|serializer| {
            serializer.serialize_bit_field(|bf| {
                bf.serialize_member(self.version, 0..4, Some("version"))?;
                bf.serialize_member(self.ihl, 4..8, Some("ihl"))
            })?;
            serializer.serialize_bit_field(|bf| {
                bf.serialize_member(self.dscp, 0..4, Some("dscp"))?;
                bf.serialize_member(self.ecn, 4..8, Some("ecn"))
            })?;
            serializer.serialize_member(self.total_length, Some("total_length"))?;
            serializer.serialize_member(self.identification, Some("identification"))?;
            serializer.serialize_bit_field(|bf| {
                bf.serialize_member(self.flags, 0..3, Some("flags"))?;
                bf.serialize_member(self.fragment_offset, 3..13, Some("fragment_offset"))
            })
        })
    }
}
