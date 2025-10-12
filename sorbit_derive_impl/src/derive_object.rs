use proc_macro2::TokenStream;
use syn::DeriveInput;
use syn::spanned::Spanned;

use crate::derive_enum::Enum;
use crate::derive_struct::Struct;

pub enum DeriveObject {
    Struct(Struct),
    Enum(Enum),
}

impl DeriveObject {
    pub fn parse(input: &DeriveInput) -> Result<Self, syn::Error> {
        match &input.data {
            syn::Data::Struct(_) => Ok(Self::Struct(Struct::parse(input)?)),
            syn::Data::Enum(_) => todo!(),
            syn::Data::Union(_) => Err(syn::Error::new(input.span(), "unions are not supported")),
        }
    }

    pub fn derive_serialize(&self) -> TokenStream {
        match self {
            DeriveObject::Struct(item) => item.derive_serialize(),
            DeriveObject::Enum(item) => item.derive_serialize(),
        }
    }

    pub fn derive_deserialize(&self) -> TokenStream {
        match self {
            DeriveObject::Struct(item) => item.derive_deserialize(),
            DeriveObject::Enum(item) => item.derive_deserialize(),
        }
    }
}
