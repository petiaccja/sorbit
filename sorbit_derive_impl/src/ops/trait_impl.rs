use crate::ir::op;
use crate::ops::constants::{
    DESERIALIZE_TRAIT, DESERIALIZER_TRAIT, DESERIALIZER_TYPE, SERIALIZATION_OUTCOME_TRAIT, SERIALIZE_TRAIT,
    SERIALIZER_TRAIT, SERIALIZER_TYPE,
};
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};

//------------------------------------------------------------------------------
// Serialize trait impl
//------------------------------------------------------------------------------

op!(
    name: "impl_serialize",
    builder: impl_serialize,
    op: ImplSerializeOp,
    inputs: {},
    outputs: {},
    attributes: {name: syn::Ident, generics: syn::Generics},
    regions: {body},
    terminator: false
);

impl ToTokens for ImplSerializeOp {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();
        let name = &self.name;
        let body = &self.body;
        let serializer = body.arguments()[0];

        tokens.extend(quote! {
            #[automatically_derived]
            impl #impl_generics #SERIALIZE_TRAIT for #name #ty_generics #where_clause{
                fn serialize<#SERIALIZER_TYPE: #SERIALIZER_TRAIT>(
                    &self,
                    #serializer: &mut #SERIALIZER_TYPE
                ) -> ::core::result::Result<
                        <#SERIALIZER_TYPE as #SERIALIZATION_OUTCOME_TRAIT>::Success,
                        <#SERIALIZER_TYPE as #SERIALIZATION_OUTCOME_TRAIT>::Error
                    >
                {
                    #body
                }
            }
        })
    }
}

//------------------------------------------------------------------------------
// Deserialize trait impl
//------------------------------------------------------------------------------

op!(
    name: "impl_deserialize",
    builder: impl_deserialize,
    op: ImplDeserializeOp,
    inputs: {},
    outputs: {},
    attributes: {name: syn::Ident, generics: syn::Generics},
    regions: {body},
    terminator: false
);

impl ToTokens for ImplDeserializeOp {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();
        let name = &self.name;
        let body = &self.body;
        let deserializer = body.arguments()[0];

        tokens.extend(quote! {
            #[automatically_derived]
            impl #impl_generics #DESERIALIZE_TRAIT for #name #ty_generics #where_clause{
                fn deserialize<#DESERIALIZER_TYPE: #DESERIALIZER_TRAIT>(
                    #deserializer: &mut #DESERIALIZER_TYPE
                ) -> ::core::result::Result<
                        Self,
                        <#DESERIALIZER_TYPE as #DESERIALIZER_TRAIT>::Error
                    >
                {
                    #body
                }
            }
        })
    }
}
