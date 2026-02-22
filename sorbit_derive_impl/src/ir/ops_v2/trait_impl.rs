use crate::ir::constants::{DESERIALIZE_TRAIT, SERIALIZE_TRAIT};
use crate::ir::dag_v2::{Id, Operation, Region, Value};
use proc_macro2::TokenStream;
use quote::quote;

//------------------------------------------------------------------------------
// Serialize trait impl
//------------------------------------------------------------------------------

struct ImplSerializeOp {
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
) -> Value {
    let body = {
        let mut region = Region::new(1);
        let serializer = region.arguments()[0];
        body(&mut region, serializer);
        region
    };
    region.push(ImplSerializeOp { id: Id::new(), name, generics, body })[0]
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
        vec![self.id.value(0)]
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

        quote! {
            #[automatically_derived]
            impl #impl_generics #SERIALIZE_TRAIT for #name #ty_generics #where_clause {
                fn serialize(&self, serializer: &mut dyn sorbit::Serializer) -> sorbit::Result<()> {
                    #body
                }
            }
        }
    }
}

//------------------------------------------------------------------------------
// Deserialize trait impl
//------------------------------------------------------------------------------

struct ImplDeserializeOp {
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
) -> Value {
    let body = {
        let mut region = Region::new(1);
        let deserializer = region.arguments()[0];
        body(&mut region, deserializer);
        region
    };
    region.push(ImplDeserializeOp { id: Id::new(), name, generics, body })[0]
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
        vec![self.id.value(0)]
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

        quote! {
            #[automatically_derived]
            impl #impl_generics #DESERIALIZE_TRAIT for #name #ty_generics #where_clause {
                fn deserialize(deserializer: &mut dyn sorbit::Deserializer) -> sorbit::Result<Self> {
                    #body
                }
            }
        }
    }
}
