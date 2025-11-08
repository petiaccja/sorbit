use proc_macro2::TokenStream;
use quote::ToTokens;

use crate::derive_struct::bit_field::BitField;
use crate::derive_struct::direct_field::DirectField;
use crate::hir;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinaryField {
    Direct(DirectField),
    Bit(BitField),
}

impl BinaryField {
    pub fn to_hir(&self) -> hir::ast::Expr {
        match self {
            BinaryField::Direct(direct_field) => direct_field.to_hir(),
            BinaryField::Bit(bit_field) => bit_field.to_hir(),
        }
    }

    pub fn derive_serialize(&self) -> TokenStream {
        self.to_hir().to_token_stream()
    }
}
