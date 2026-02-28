use proc_macro2::TokenStream;

mod ast;
mod parse;

pub struct Enum {}

impl Enum {
    pub fn derive_serialize(&self) -> TokenStream {
        TokenStream::new()
    }

    pub fn derive_deserialize(&self) -> TokenStream {
        TokenStream::new()
    }
}
