use crate::ir::op;
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};

op!(
    name: "len",
    builder: len,
    op: LenOp,
    inputs: {serializer, collection},
    outputs: {len},
    attributes: {len_ty: syn::Type},
    regions: {},
    terminator: false
);

impl ToTokens for LenOp {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let serializer = &self.serializer;
        let collection = &self.collection;
        let len_ty = &self.len_ty;
        tokens.extend(quote! { ::sorbit::collection::len::<#len_ty, _, _>(#serializer, #collection) })
    }
}

op!(
    name: "byte_count",
    builder: byte_count,
    op: ByteCountOp,
    inputs: {serializer, span},
    outputs: {byte_count},
    attributes: {byte_count_ty: syn::Type},
    regions: {},
    terminator: false
);

impl ToTokens for ByteCountOp {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let serializer = &self.serializer;
        let collection = &self.span;
        let byte_count_ty = &self.byte_count_ty;
        tokens.extend(quote! { ::sorbit::collection::byte_count::<#byte_count_ty, _, _>(#serializer, #collection) })
    }
}

op!(
    name: "items",
    builder: items,
    op: ItemsOp,
    inputs: {collection},
    outputs: {items},
    attributes: {},
    regions: {},
    terminator: false
);

impl ToTokens for ItemsOp {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let collection = &self.collection;
        tokens.extend(quote! { ::sorbit::collection::items(#collection) })
    }
}

op!(
    name: "deserialize_items_by_len",
    builder: deserialize_items_by_len,
    op: DeserializeItemsByLenOp,
    inputs: {deserializer, len},
    outputs: {collection_value},
    attributes: {collection_ty: syn::Type},
    regions: {},
    terminator: false
);

impl ToTokens for DeserializeItemsByLenOp {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let deserializer = &self.deserializer;
        let len = &self.len;
        let collection_ty = &self.collection_ty;
        tokens.extend(quote! {
            ::sorbit::collection::deserialize_items_by_len::<#collection_ty, _, _, _>(
                #deserializer,
                #len
            )
        })
    }
}

op!(
    name: "deserialize_items_by_byte_count",
    builder: deserialize_items_by_byte_count,
    op: DeserializeItemsByByteCountOp,
    inputs: {deserializer, byte_count},
    outputs: {collection_value},
    attributes: {collection_ty: syn::Type},
    regions: {},
    terminator: false
);

impl ToTokens for DeserializeItemsByByteCountOp {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let deserializer = &self.deserializer;
        let byte_count = &self.byte_count;
        let collection_ty = &self.collection_ty;
        tokens.extend(quote! {
            ::sorbit::collection::deserialize_items_by_byte_count::<#collection_ty, _, _, _>(
                #deserializer,
                #byte_count
            )
        })
    }
}
