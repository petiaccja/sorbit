#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ByteOrder {
    LittleEndian,
    BigEndian,
}

impl ByteOrder {
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
