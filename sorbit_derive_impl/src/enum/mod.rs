use proc_macro2::TokenStream;
use syn::DeriveInput;

use crate::ir::{Region, ToDeserializeOp as _, ToSerializeOp as _};

mod ast;
mod parse;

pub struct Enum {
    inner: ast::Enum,
}

impl Enum {
    pub fn derive_serialize(&self) -> TokenStream {
        let mut region = Region::new(0);
        self.inner.to_serialize_op(&mut region, ());
        region.to_token_stream_formatted(false)
    }

    pub fn derive_deserialize(&self) -> TokenStream {
        let mut region = Region::new(0);
        self.inner.to_deserialize_op(&mut region, ());
        region.to_token_stream_formatted(false)
    }

    pub fn derive_pack_into(&self) -> TokenStream {
        self.inner.to_pack_into_tokens()
    }

    pub fn derive_unpack_from(&self) -> TokenStream {
        self.inner.to_unpack_from_tokens()
    }
}

impl TryFrom<DeriveInput> for Enum {
    type Error = syn::Error;

    fn try_from(value: DeriveInput) -> Result<Self, Self::Error> {
        let parsed = parse::Enum::try_from(value)?;
        let inner = ast::Enum::try_from(parsed)?;
        Ok(Self { inner })
    }
}
