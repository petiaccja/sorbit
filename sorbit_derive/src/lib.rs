use proc_macro::TokenStream;
use syn::DeriveInput;

use sorbit_derive_impl::DeriveObject;

#[proc_macro_derive(Serialize, attributes(sorbit))]
pub fn derive_serialize(tokens: TokenStream) -> TokenStream {
    let input: DeriveInput = match syn::parse(tokens) {
        Ok(input) => input,
        Err(err) => return err.into_compile_error().into(),
    };
    let object = match DeriveObject::parse(input) {
        Ok(object) => object,
        Err(err) => return err.into_compile_error().into(),
    };
    object.derive_serialize().into()
}

#[proc_macro_derive(Deserialize, attributes(sorbit))]
pub fn derive_deserialize(tokens: TokenStream) -> TokenStream {
    let input: DeriveInput = match syn::parse(tokens) {
        Ok(input) => input,
        Err(err) => return err.into_compile_error().into(),
    };
    let object = match DeriveObject::parse(input) {
        Ok(object) => object,
        Err(err) => return err.into_compile_error().into(),
    };
    object.derive_deserialize().into()
}
