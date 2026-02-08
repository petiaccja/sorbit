use crate::derive_struct::bit_field::BitField;
use crate::derive_struct::direct_field::DirectField;
use crate::ir::dag::{Operation, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinaryField {
    Direct(DirectField),
    Bit(BitField),
}

impl BinaryField {
    pub fn to_serialize_op(&self, serializer: Value) -> Operation {
        match self {
            BinaryField::Direct(direct_field) => direct_field.to_serialize_op(serializer),
            BinaryField::Bit(bit_field) => bit_field.to_serialize_op(serializer),
        }
    }

    pub fn to_deserialize_op(&self, serializer: Value) -> Operation {
        match self {
            BinaryField::Direct(direct_field) => direct_field.to_deserialize_op(serializer),
            BinaryField::Bit(bit_field) => bit_field.to_deserialize_op(serializer),
        }
    }
}
