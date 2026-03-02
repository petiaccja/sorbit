mod attribute;
mod r#enum;
mod ir;
mod r#struct;

use proc_macro2::{Span, TokenStream};
use syn::DeriveInput;
use syn::spanned::Spanned;

use r#enum::Enum;
use r#struct::Struct;

pub enum DeriveObject {
    Struct(Struct),
    Enum(Enum),
}

impl DeriveObject {
    pub fn parse(input: DeriveInput) -> Result<Self, syn::Error> {
        match &input.data {
            syn::Data::Struct(_) => Ok(Self::Struct(Struct::try_from(input)?)),
            syn::Data::Enum(_) => Ok(Self::Enum(Enum::try_from(input)?)),
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

    pub fn derive_pack_into(&self) -> TokenStream {
        match self {
            DeriveObject::Struct(_) => {
                syn::Error::new(Span::call_site(), "PackInto can only be derived for enums").into_compile_error()
            }
            DeriveObject::Enum(item) => item.derive_pack_into(),
        }
    }

    pub fn derive_unpack_from(&self) -> TokenStream {
        match self {
            DeriveObject::Struct(_) => {
                syn::Error::new(Span::call_site(), "UnpackFrom can only be derived for enums").into_compile_error()
            }
            DeriveObject::Enum(item) => item.derive_unpack_from(),
        }
    }
}
