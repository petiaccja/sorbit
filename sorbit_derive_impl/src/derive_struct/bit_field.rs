use syn::Ident;

use crate::derive_struct::{bit_field_attribute::BitFieldAttribute, packed_field::PackedField};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitField {
    pub name: Ident,
    pub attribute: BitFieldAttribute,
    pub members: Vec<PackedField>,
}
