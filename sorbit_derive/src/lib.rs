use proc_macro::TokenStream;

#[proc_macro_derive(Serialize, attributes(layout, fallback))]
pub fn derive_serialize(_tokens: TokenStream) -> TokenStream {
    TokenStream::new()
}

#[proc_macro_derive(Deserialize, attributes(layout, fallback))]
pub fn derive_deserialize(_tokens: TokenStream) -> TokenStream {
    TokenStream::new()
}
