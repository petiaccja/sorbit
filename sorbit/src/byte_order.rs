//! Specifying and querying byte ordering.

/// The ordering of bytes within a primitive type's memory layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ByteOrder {
    /// The type's least significant byte is at the lowest memory address.
    LittleEndian,
    /// The type's most significant byte is at the lowest memory address.
    BigEndian,
}

impl ByteOrder {
    /// Determine the system's native byte ordering.
    ///
    /// For example, little endian for x86 and amd64 computers.
    pub fn native() -> Self {
        match 0x00FFu16.to_ne_bytes()[0] {
            0xFF => Self::LittleEndian,
            _ => Self::BigEndian,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::byte_order::ByteOrder;

    #[test]
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    fn native_byte_order() {
        assert_eq!(ByteOrder::native(), ByteOrder::LittleEndian);
    }
}
