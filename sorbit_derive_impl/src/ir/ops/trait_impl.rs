use crate::ir::constants::{
    DESERIALIZE_TRAIT, DESERIALIZER_TRAIT, DESERIALIZER_TYPE, SERIALIZE_TRAIT, SERIALIZER_OUTPUT_TRAIT,
    SERIALIZER_TRAIT, SERIALIZER_TYPE,
};
use crate::ir::dag::{Id, Operation, Region, Value};
use proc_macro2::TokenStream;
use quote::quote;

//------------------------------------------------------------------------------
// Serialize trait impl
//------------------------------------------------------------------------------

pub struct ImplSerializeOp {
    id: Id,
    name: syn::Ident,
    generics: syn::Generics,
    body: Region,
}

pub fn impl_serialize(
    region: &mut Region,
    name: syn::Ident,
    generics: syn::Generics,
    body: impl FnOnce(&mut Region, Value),
) {
    let body = {
        let mut region = Region::new(1);
        let serializer = region.arguments()[0];
        body(&mut region, serializer);
        region
    };
    region.push(ImplSerializeOp { id: Id::new(), name, generics, body });
}

impl Operation for ImplSerializeOp {
    fn name(&self) -> &str {
        "impl_serialize"
    }

    fn id(&self) -> Id {
        self.id
    }

    fn inputs(&self) -> Vec<Value> {
        vec![]
    }

    fn outputs(&self) -> Vec<Value> {
        vec![]
    }

    fn regions(&self) -> Vec<&Region> {
        vec![&self.body]
    }

    fn attributes(&self) -> Vec<String> {
        vec![self.name.to_string()]
    }

    fn to_token_stream(&self) -> TokenStream {
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();
        let name = &self.name;
        let body = &self.body;
        let serializer = body.arguments()[0];

        quote! {
            #[automatically_derived]
            impl #impl_generics #SERIALIZE_TRAIT for #name #ty_generics #where_clause{
                fn serialize<#SERIALIZER_TYPE: #SERIALIZER_TRAIT>(
                    &self,
                    #serializer: &mut #SERIALIZER_TYPE
                ) -> ::core::result::Result<
                        <#SERIALIZER_TYPE as #SERIALIZER_OUTPUT_TRAIT>::Success,
                        <#SERIALIZER_TYPE as #SERIALIZER_OUTPUT_TRAIT>::Error
                    >
                {
                    #body
                }
            }
        }
    }
}

//------------------------------------------------------------------------------
// Deserialize trait impl
//------------------------------------------------------------------------------

pub struct ImplDeserializeOp {
    id: Id,
    name: syn::Ident,
    generics: syn::Generics,
    body: Region,
}

pub fn impl_deserialize(
    region: &mut Region,
    name: syn::Ident,
    generics: syn::Generics,
    body: impl FnOnce(&mut Region, Value),
) {
    let body = {
        let mut region = Region::new(1);
        let deserializer = region.arguments()[0];
        body(&mut region, deserializer);
        region
    };
    region.push(ImplDeserializeOp { id: Id::new(), name, generics, body });
}

impl Operation for ImplDeserializeOp {
    fn name(&self) -> &str {
        "impl_deserialize"
    }

    fn id(&self) -> Id {
        self.id
    }

    fn inputs(&self) -> Vec<Value> {
        vec![]
    }

    fn outputs(&self) -> Vec<Value> {
        vec![]
    }

    fn regions(&self) -> Vec<&Region> {
        vec![&self.body]
    }

    fn attributes(&self) -> Vec<String> {
        vec![self.name.to_string()]
    }

    fn to_token_stream(&self) -> TokenStream {
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();
        let name = &self.name;
        let body = &self.body;
        let deserializer = body.arguments()[0];

        quote! {
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
        }
    }
}
