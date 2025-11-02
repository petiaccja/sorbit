use proc_macro2::TokenStream;
use syn::{Expr, Type};

use crate::derive_struct::bit_field::BitField;
use crate::derive_struct::direct_field::DirectField;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinaryField {
    Direct(DirectField),
    Bit(BitField),
}

impl BinaryField {
    pub fn derive_serialize(&self, parent: &Expr, serializer: &Expr, serializer_ty: &Type) -> TokenStream {
        match self {
            BinaryField::Direct(field) => field.derive_serialize(parent, serializer),
            BinaryField::Bit(field) => field.derive_serialize(parent, serializer, serializer_ty),
        }
    }
}
