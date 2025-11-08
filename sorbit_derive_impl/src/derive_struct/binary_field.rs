use crate::derive_struct::bit_field::BitField;
use crate::derive_struct::direct_field::DirectField;
use crate::ir_se;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinaryField {
    Direct(DirectField),
    Bit(BitField),
}

impl BinaryField {
    pub fn lower_se(&self) -> ir_se::Expr {
        match self {
            BinaryField::Direct(direct_field) => direct_field.lower_se(),
            BinaryField::Bit(bit_field) => bit_field.lower_se(),
        }
    }
}
