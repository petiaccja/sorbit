use proc_macro2::TokenStream;

use crate::derive_struct::bit_field::BitField;
use crate::derive_struct::direct_field::DirectField;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinaryField {
    Direct(DirectField),
    Bit(BitField),
}

impl BinaryField {
    pub fn derive_serialize(&self) -> TokenStream {
        match self {
            BinaryField::Direct(field) => field.derive_serialize(),
            BinaryField::Bit(field) => field.derive_serialize(),
        }
    }
}
