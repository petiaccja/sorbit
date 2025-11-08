use quote::{ToTokens, quote};

#[derive(Clone, PartialEq, Eq)]
pub struct DeserializeImpl {
    pub name: syn::Ident,
    pub generics: syn::Generics,
}

pub fn deserialize_impl(name: syn::Ident, generics: syn::Generics) -> DeserializeImpl {
    DeserializeImpl { name, generics }
}

//------------------------------------------------------------------------------
// Implement Debug trait.
//------------------------------------------------------------------------------

impl std::fmt::Debug for DeserializeImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "impl Deserialize for {} {{ {:?} }}", self.name, "todo!()")
    }
}

//------------------------------------------------------------------------------
// Implement ToTokens trait.
//------------------------------------------------------------------------------

struct DeserializerTrait;
struct DeserializerType;
struct DeserializeTrait;
struct DeserializerObject;

const DESERIALIZER_TRAIT: DeserializerTrait = DeserializerTrait {};
const DESERIALIZER_TYPE: DeserializerType = DeserializerType {};
const DESERIALIZE_TRAIT: DeserializeTrait = DeserializeTrait {};
const DESERIALIZER_OBJECT: DeserializerObject = DeserializerObject {};

impl ToTokens for DeserializerTrait {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend(quote! {::sorbit::deserialize::Deserializer});
    }
}

impl ToTokens for DeserializerType {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend(quote! {D});
    }
}

impl ToTokens for DeserializeTrait {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend(quote! {::sorbit::deserialize::Deserialize});
    }
}

impl ToTokens for DeserializerObject {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend(quote! {deserializer});
    }
}

impl ToTokens for DeserializeImpl {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let name = &self.name;
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();
        tokens.extend(quote! {
            impl #impl_generics #DESERIALIZE_TRAIT for #name #ty_generics #where_clause{
                fn deserialize<#DESERIALIZER_TYPE: #DESERIALIZER_TRAIT>(
                    #DESERIALIZER_OBJECT: &mut #DESERIALIZER_TYPE
                ) -> ::core::result::Result<
                        Self,
                        <#DESERIALIZER_TYPE as #DESERIALIZER_TRAIT>::Error
                    >
                {
                    todo!()
                }
            }
        });
    }
}
