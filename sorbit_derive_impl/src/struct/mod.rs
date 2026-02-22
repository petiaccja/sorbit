use proc_macro2::TokenStream;
use syn::DeriveInput;

mod ast;
mod parse;

use ast::{ToDeserializeOp as _, ToSerializeOp as _};

use crate::ir::dag::Region;

pub struct Struct {
    inner: ast::Struct,
}

impl Struct {
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
}

impl TryFrom<DeriveInput> for Struct {
    type Error = syn::Error;

    fn try_from(value: DeriveInput) -> Result<Self, Self::Error> {
        let parsed = parse::Struct::try_from(value)?;
        let inner = ast::Struct::try_from(parsed)?;
        Ok(Self { inner })
    }
}
