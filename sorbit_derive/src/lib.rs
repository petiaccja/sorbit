use proc_macro::TokenStream;
use syn::{DeriveInput, spanned::Spanned};

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

#[proc_macro_derive(PackInto, attributes(sorbit))]
pub fn derive_pack_into(tokens: TokenStream) -> TokenStream {
    let input: DeriveInput = match syn::parse(tokens) {
        Ok(input) => input,
        Err(err) => return err.into_compile_error().into(),
    };
    if let syn::Data::Enum(_) = input.data {
        let object = match DeriveObject::parse(input) {
            Ok(object) => object,
            Err(err) => return err.into_compile_error().into(),
        };
        object.derive_pack_into().into()
    } else {
        syn::Error::new(input.span(), "PackInto can only be derived for enums").into_compile_error().into()
    }
}

#[proc_macro_derive(UnpackFrom, attributes(sorbit))]
pub fn derive_unpack_from(tokens: TokenStream) -> TokenStream {
    let input: DeriveInput = match syn::parse(tokens) {
        Ok(input) => input,
        Err(err) => return err.into_compile_error().into(),
    };
    if let syn::Data::Enum(_) = input.data {
        let object = match DeriveObject::parse(input) {
            Ok(object) => object,
            Err(err) => return err.into_compile_error().into(),
        };
        object.derive_unpack_from().into()
    } else {
        syn::Error::new(input.span(), "UnpackFrom can only be derived for enums")
            .into_compile_error()
            .into()
    }
}
