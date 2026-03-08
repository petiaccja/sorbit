use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::Type;

use crate::ir::dag::{Id, Operation, Region, Value};

//------------------------------------------------------------------------------
// Length
//------------------------------------------------------------------------------

struct LenOp {
    id: Id,
    serializer: Value,
    collection: Value,
    len: Type,
}

pub fn len(region: &mut Region, serializer: Value, collection: Value, len: Type) -> Value {
    region.push(LenOp { id: Id::new(), serializer, collection, len })[0]
}

impl Operation for LenOp {
    fn name(&self) -> &str {
        "len"
    }

    fn id(&self) -> Id {
        self.id
    }

    fn inputs(&self) -> Vec<Value> {
        vec![self.serializer, self.collection]
    }

    fn outputs(&self) -> Vec<Value> {
        vec![self.id.value(0)]
    }

    fn regions(&self) -> Vec<&Region> {
        vec![]
    }

    fn attributes(&self) -> Vec<String> {
        vec![self.len.to_token_stream().to_string()]
    }

    fn to_token_stream(&self) -> TokenStream {
        let serializer = &self.serializer;
        let collection = &self.collection;
        let len = &self.len;
        quote! { ::sorbit::collection::len::<#len, _, _>(#serializer, #collection) }
    }
}

//------------------------------------------------------------------------------
// Get items of collection
//------------------------------------------------------------------------------

struct ItemsOp {
    id: Id,
    collection: Value,
}

pub fn items(region: &mut Region, collection: Value) -> Value {
    region.push(ItemsOp { id: Id::new(), collection })[0]
}

impl Operation for ItemsOp {
    fn name(&self) -> &str {
        "items"
    }

    fn id(&self) -> Id {
        self.id
    }

    fn inputs(&self) -> Vec<Value> {
        vec![self.collection]
    }

    fn outputs(&self) -> Vec<Value> {
        vec![self.id.value(0)]
    }

    fn regions(&self) -> Vec<&Region> {
        vec![]
    }

    fn attributes(&self) -> Vec<String> {
        vec![]
    }

    fn to_token_stream(&self) -> TokenStream {
        let collection = &self.collection;
        quote! { ::sorbit::collection::items(#collection) }
    }
}

//------------------------------------------------------------------------------
// Deserialize items exact
//------------------------------------------------------------------------------

struct DeserializeItemsExactOp {
    id: Id,
    deserializer: Value,
    len: Value,
    collection: Type,
}

pub fn deserialize_items_exact(region: &mut Region, deserializer: Value, len: Value, collection: Type) -> Value {
    region.push(DeserializeItemsExactOp { id: Id::new(), deserializer, len, collection })[0]
}

impl Operation for DeserializeItemsExactOp {
    fn name(&self) -> &str {
        "serialize_items_exact"
    }

    fn id(&self) -> Id {
        self.id
    }

    fn inputs(&self) -> Vec<Value> {
        vec![self.deserializer, self.len]
    }

    fn outputs(&self) -> Vec<Value> {
        vec![self.id.value(0)]
    }

    fn regions(&self) -> Vec<&Region> {
        vec![]
    }

    fn attributes(&self) -> Vec<String> {
        vec![]
    }

    fn to_token_stream(&self) -> TokenStream {
        let deserializer = &self.deserializer;
        let len = &self.len;
        let collection = &self.collection;
        quote! {
            ::sorbit::collection::deserialize_items_exact::<#collection>(
                #deserializer,
                #len
            )
        }
    }
}
