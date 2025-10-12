use crate::derive_struct::bit_field::BitField;
use crate::derive_struct::direct_field::DirectField;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinaryField {
    Direct(DirectField),
    Bit(BitField),
}
