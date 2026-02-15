use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::DeriveInput;

mod ast;
mod parse;

use ast::{ToDeserializeOp as _, ToSerializeOp as _};

pub struct Struct {
    inner: ast::Struct,
}

impl Struct {
    pub fn derive_serialize(&self) -> TokenStream {
        self.inner.to_serialize_op(()).to_token_stream()
    }

    pub fn derive_deserialize(&self) -> TokenStream {
        self.inner.to_deserialize_op(()).to_token_stream()
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
