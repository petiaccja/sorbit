use proc_macro2::TokenStream;

pub struct Enum {}

impl Enum {
    pub fn derive_serialize(&self) -> TokenStream {
        TokenStream::new()
    }

    pub fn derive_deserialize(&self) -> TokenStream {
        TokenStream::new()
    }
}
