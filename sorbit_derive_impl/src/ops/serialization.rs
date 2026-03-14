use proc_macro2::TokenStream;
use quote::{ToTokens, quote};

use crate::attribute::ByteOrder;
use crate::ir::op;
use crate::ops::constants::{
    BIG_ENDIAN, DESERIALIZE_TRAIT, DESERIALIZER_TRAIT, LITTLE_ENDIAN, SERIALIZE_TRAIT, SERIALIZER_TRAIT,
};

//------------------------------------------------------------------------------
// Success
//------------------------------------------------------------------------------

op!(
    name: "success",
    builder: success,
    op: SuccessOp,
    inputs: {serializer},
    outputs: {success_result},
    attributes: {},
    regions: {},
    terminator: false
);

impl ToTokens for SuccessOp {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let serializer = &self.serializer;
        tokens.extend(quote! { #SERIALIZER_TRAIT::success(#serializer) })
    }
}

//------------------------------------------------------------------------------
// Error
//------------------------------------------------------------------------------

op!(
    name: "error",
    builder: error,
    op: ErrorOp,
    inputs: {deserializer},
    outputs: {error_result},
    attributes: {message: String},
    regions: {},
    terminator: false
);

impl ToTokens for ErrorOp {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let deserializer = &self.deserializer;
        let message = &self.message;
        tokens.extend(quote! { #DESERIALIZER_TRAIT::error(#deserializer, #message) })
    }
}

//------------------------------------------------------------------------------
// Pad
//------------------------------------------------------------------------------

op!(
    name: "pad",
    builder: pad,
    op: PadOp,
    inputs: {serializer},
    outputs: {padded_serializer},
    attributes: {until: u64, serializing: bool},
    regions: {},
    terminator: false
);

impl ToTokens for PadOp {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let serializer = &self.serializer;
        let until = self.until;
        match self.serializing {
            true => tokens.extend(quote! { #SERIALIZER_TRAIT::pad(#serializer, #until) }),
            false => tokens.extend(quote! { #DESERIALIZER_TRAIT::pad(#serializer, #until) }),
        }
    }
}

//------------------------------------------------------------------------------
// Align
//------------------------------------------------------------------------------

op!(
    name: "align",
    builder: align,
    op: AlignOp,
    inputs: {serializer},
    outputs: {aligned_serializer},
    attributes: {multiple_of: u64, serializing: bool},
    regions: {},
    terminator: false
);

impl ToTokens for AlignOp {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let serializer = &self.serializer;
        let multiple_of = self.multiple_of;
        match self.serializing {
            true => tokens.extend(quote! { #SERIALIZER_TRAIT::align(#serializer, #multiple_of) }),
            false => tokens.extend(quote! { #DESERIALIZER_TRAIT::align(#serializer, #multiple_of) }),
        }
    }
}

//------------------------------------------------------------------------------
// Annotate result
//------------------------------------------------------------------------------

/*
op!(
    name: "annotate_result",
    builder: annotate_result,
    op: AnnotateResultOp,
    inputs: {result},
    outputs: {annotated_result},
    attributes: {annotation: String},
    regions: {},
    terminator: false
);

#[allow(unused)]
impl ToTokens for AnnotateResultOp {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let result = &self.result;
        let annotation = &self.annotation;
        tokens.extend(quote! { #result.map_err(|err| #TRACE_ERROR_TRAIT::annotate(err, #annotation)) })
    }
}
*/

//------------------------------------------------------------------------------
// Serialize object
//------------------------------------------------------------------------------

op!(
    name: "serialize_object",
    builder: serialize_object,
    op: SerializeObjectOp,
    inputs: {serializer, object},
    outputs: {serialized_object},
    attributes: {},
    regions: {},
    terminator: false
);

impl ToTokens for SerializeObjectOp {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let serializer = &self.serializer;
        let object = &self.object;
        tokens.extend(quote! { #SERIALIZE_TRAIT::serialize(#object, #serializer) })
    }
}

//------------------------------------------------------------------------------
// Serialize composite
//------------------------------------------------------------------------------

op!(
    name: "serialize_composite",
    builder: serialize_composite,
    op: SerializeCompositeOp,
    inputs: {serializer},
    outputs: {composite_result},
    attributes: {},
    regions: {body},
    terminator: false
);

impl ToTokens for SerializeCompositeOp {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let serializer = &self.serializer;
        let body = &self.body;
        let inner_serializer = body.arguments()[0];
        tokens.extend(quote! {
            #SERIALIZER_TRAIT::serialize_composite(#serializer, |#inner_serializer| {
                #body
            })
        })
    }
}

//------------------------------------------------------------------------------
// Deserialize object
//------------------------------------------------------------------------------

op!(
    name: "deserialize_object",
    builder: deserialize_object,
    op: DeserializeObjectOp,
    inputs: {deserializer},
    outputs: {deserialized_object},
    attributes: {ty: syn::Type},
    regions: {},
    terminator: false
);

impl ToTokens for DeserializeObjectOp {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let deserializer = &self.deserializer;
        let ty = &self.ty;
        tokens.extend(quote! { <#ty as #DESERIALIZE_TRAIT>::deserialize(#deserializer)})
    }
}

//------------------------------------------------------------------------------
// Deserialize composite
//------------------------------------------------------------------------------

op!(
    name: "deserialize_composite",
    builder: deserialize_composite,
    op: DeserializeCompositeOp,
    inputs: {deserializer},
    outputs: {composite_result},
    attributes: {},
    regions: {body},
    terminator: false
);

impl ToTokens for DeserializeCompositeOp {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let deserializer = &self.deserializer;
        let body = &self.body;
        let inner_deserializer = self.body.arguments()[0];
        tokens.extend(quote! {
            #DESERIALIZER_TRAIT::deserialize_composite(#deserializer, |#inner_deserializer| {
                #body
            })
        })
    }
}

//------------------------------------------------------------------------------
// Deserialize byte order
//------------------------------------------------------------------------------

op!(
    name: "byte_order",
    builder: byte_order,
    op: ByteOrderOp,
    inputs: {serializer},
    outputs: {ordered_result},
    attributes: {byte_order: ByteOrder, is_serializing: bool},
    regions: {body},
    terminator: false
);

impl ToTokens for ByteOrderOp {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        use crate::attribute::ByteOrder::*;
        let se = &self.serializer;
        let body = &self.body;
        let inner = self.body.arguments()[0];
        let trait_ = match self.is_serializing {
            true => quote! { #SERIALIZER_TRAIT },
            false => quote! { #DESERIALIZER_TRAIT },
        };
        match self.byte_order {
            BigEndian => tokens.extend(quote! {
                #trait_::with_byte_order(#se, #BIG_ENDIAN, |#inner| {
                    #body
                })
            }),
            LittleEndian => tokens.extend(quote! {
                #trait_::with_byte_order(#se, #LITTLE_ENDIAN, |#inner| {
                    #body
                })
            }),
        }
    }
}
