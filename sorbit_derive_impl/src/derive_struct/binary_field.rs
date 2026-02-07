use crate::derive_struct::bit_field::BitField;
use crate::derive_struct::direct_field::DirectField;
use crate::ir::dag::{Operation, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinaryField {
    Direct(DirectField),
    Bit(BitField),
}

impl BinaryField {
    pub fn lower_se(&self, serializer: Value) -> Operation {
        match self {
            BinaryField::Direct(direct_field) => direct_field.lower_se(serializer),
            BinaryField::Bit(bit_field) => bit_field.lower_se(serializer),
        }
    }

    pub fn lower_de(&self, serializer: Value) -> Operation {
        match self {
            BinaryField::Direct(direct_field) => direct_field.lower_de(serializer),
            BinaryField::Bit(bit_field) => bit_field.lower_de(serializer),
        }
    }
}
