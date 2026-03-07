use proc_macro2::TokenStream;
use quote::{ToTokens, quote};

use crate::attribute::ByteOrder;
use crate::ir::constants::{
    BIG_ENDIAN, DESERIALIZE_TRAIT, DESERIALIZER_TRAIT, LITTLE_ENDIAN, SERIALIZE_TRAIT, SERIALIZER_TRAIT,
    TRACE_ERROR_TRAIT,
};
use crate::ir::dag::{Id, Operation, Region, Value};

//------------------------------------------------------------------------------
// Success
//------------------------------------------------------------------------------

struct SuccessOp {
    id: Id,
    serializer: Value,
}

pub fn success(region: &mut Region, serializer: Value) -> Value {
    region.push(SuccessOp { id: Id::new(), serializer })[0]
}

impl Operation for SuccessOp {
    fn name(&self) -> &str {
        "success"
    }

    fn id(&self) -> Id {
        self.id
    }

    fn inputs(&self) -> Vec<Value> {
        vec![self.serializer]
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
        let serializer = &self.serializer;
        quote! { #SERIALIZER_TRAIT::success(#serializer) }
    }
}

//------------------------------------------------------------------------------
// Error
//------------------------------------------------------------------------------

struct ErrorOp {
    id: Id,
    deserializer: Value,
    message: String,
}

pub fn error(region: &mut Region, deserializer: Value, message: String) -> Value {
    region.push(ErrorOp { id: Id::new(), deserializer, message })[0]
}

impl Operation for ErrorOp {
    fn name(&self) -> &str {
        "error"
    }

    fn id(&self) -> Id {
        self.id
    }

    fn inputs(&self) -> Vec<Value> {
        vec![self.deserializer]
    }

    fn outputs(&self) -> Vec<Value> {
        vec![self.id.value(0)]
    }

    fn regions(&self) -> Vec<&Region> {
        vec![]
    }

    fn attributes(&self) -> Vec<String> {
        vec![self.message.clone()]
    }

    fn to_token_stream(&self) -> TokenStream {
        let deserializer = &self.deserializer;
        let message = &self.message;
        quote! { #DESERIALIZER_TRAIT::error(#deserializer, #message) }
    }
}

//------------------------------------------------------------------------------
// Pad
//------------------------------------------------------------------------------

struct PadOp {
    id: Id,
    serializer: Value,
    until: u64,
    serializing: bool,
}

pub fn pad(region: &mut Region, serializer: Value, until: u64, serializing: bool) -> Value {
    region.push(PadOp { id: Id::new(), serializer, until, serializing })[0]
}

impl Operation for PadOp {
    fn name(&self) -> &str {
        "pad"
    }

    fn id(&self) -> Id {
        self.id
    }

    fn inputs(&self) -> Vec<Value> {
        vec![self.serializer]
    }

    fn outputs(&self) -> Vec<Value> {
        vec![self.id.value(0)]
    }

    fn regions(&self) -> Vec<&Region> {
        vec![]
    }

    fn attributes(&self) -> Vec<String> {
        vec![self.until.to_string(), self.serializing.to_string()]
    }

    fn to_token_stream(&self) -> TokenStream {
        let serializer = &self.serializer;
        let until = self.until;
        match self.serializing {
            true => quote! { #SERIALIZER_TRAIT::pad(#serializer, #until) },
            false => quote! { #DESERIALIZER_TRAIT::pad(#serializer, #until) },
        }
    }
}

//------------------------------------------------------------------------------
// Align
//------------------------------------------------------------------------------

struct AlignOp {
    id: Id,
    serializer: Value,
    multiple_of: u64,
    serializing: bool,
}

pub fn align(region: &mut Region, serializer: Value, multiple_of: u64, serializing: bool) -> Value {
    region.push(AlignOp { id: Id::new(), serializer, multiple_of, serializing })[0]
}

impl Operation for AlignOp {
    fn name(&self) -> &str {
        "align"
    }

    fn id(&self) -> Id {
        self.id
    }

    fn inputs(&self) -> Vec<Value> {
        vec![self.serializer]
    }

    fn outputs(&self) -> Vec<Value> {
        vec![self.id.value(0)]
    }

    fn regions(&self) -> Vec<&Region> {
        vec![]
    }

    fn attributes(&self) -> Vec<String> {
        vec![self.multiple_of.to_string(), self.serializing.to_string()]
    }

    fn to_token_stream(&self) -> TokenStream {
        let serializer = &self.serializer;
        let multiple_of = self.multiple_of;
        match self.serializing {
            true => quote! { #SERIALIZER_TRAIT::align(#serializer, #multiple_of) },
            false => quote! { #DESERIALIZER_TRAIT::align(#serializer, #multiple_of) },
        }
    }
}

//------------------------------------------------------------------------------
// Annotate result
//------------------------------------------------------------------------------

#[allow(unused)]
struct AnnotateResultOp {
    id: Id,
    result: Value,
    annotation: String,
}

#[allow(unused)]
pub fn annotate_result(region: &mut Region, result: Value, annotation: String) -> Value {
    region.push(AnnotateResultOp { id: Id::new(), result, annotation })[0]
}

impl Operation for AnnotateResultOp {
    fn name(&self) -> &str {
        "annotate_result"
    }

    fn id(&self) -> Id {
        self.id
    }

    fn inputs(&self) -> Vec<Value> {
        vec![self.result]
    }

    fn outputs(&self) -> Vec<Value> {
        vec![self.id.value(0)]
    }

    fn regions(&self) -> Vec<&Region> {
        vec![]
    }

    fn attributes(&self) -> Vec<String> {
        vec![self.annotation.clone()]
    }

    fn to_token_stream(&self) -> TokenStream {
        let result = &self.result;
        let annotation = &self.annotation;
        quote! { #result.map_err(|err| #TRACE_ERROR_TRAIT::annotate(err, #annotation)) }
    }
}

//------------------------------------------------------------------------------
// Serialize object
//------------------------------------------------------------------------------

struct SerializeObjectOp {
    id: Id,
    serializer: Value,
    object: Value,
}

pub fn serialize_object(region: &mut Region, serializer: Value, object: Value) -> Value {
    region.push(SerializeObjectOp { id: Id::new(), serializer, object })[0]
}

impl Operation for SerializeObjectOp {
    fn name(&self) -> &str {
        "serialize_object"
    }

    fn id(&self) -> Id {
        self.id
    }

    fn inputs(&self) -> Vec<Value> {
        vec![self.serializer, self.object]
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
        let serializer = &self.serializer;
        let object = &self.object;
        quote! { #SERIALIZE_TRAIT::serialize(#object, #serializer) }
    }
}

//------------------------------------------------------------------------------
// Serialize composite
//------------------------------------------------------------------------------

struct SerializeCompositeOp {
    id: Id,
    serializer: Value,
    body: Region,
}

pub fn serialize_composite(region: &mut Region, serializer: Value, body: impl FnOnce(&mut Region, Value)) -> Value {
    let body = {
        let mut region = Region::new(1);
        let se_inner = region.arguments()[0];
        body(&mut region, se_inner);
        region
    };
    region.push(SerializeCompositeOp { id: Id::new(), serializer, body })[0]
}

impl Operation for SerializeCompositeOp {
    fn name(&self) -> &str {
        "serialize_composite"
    }

    fn id(&self) -> Id {
        self.id
    }

    fn inputs(&self) -> Vec<Value> {
        vec![self.serializer]
    }

    fn outputs(&self) -> Vec<Value> {
        vec![self.id.value(0)]
    }

    fn regions(&self) -> Vec<&Region> {
        vec![&self.body]
    }

    fn attributes(&self) -> Vec<String> {
        vec![]
    }

    fn to_token_stream(&self) -> TokenStream {
        let serializer = &self.serializer;
        let body = &self.body;
        let inner_serializer = body.arguments()[0];
        quote! {
            #SERIALIZER_TRAIT::serialize_composite(#serializer, |#inner_serializer| {
                #body
            })
        }
    }
}

//------------------------------------------------------------------------------
// Deserialize object
//------------------------------------------------------------------------------

struct DeserializeObjectOp {
    id: Id,
    deserializer: Value,
    ty: syn::Type,
}

pub fn deserialize_object(region: &mut Region, deserializer: Value, ty: syn::Type) -> Value {
    region.push(DeserializeObjectOp { id: Id::new(), deserializer, ty })[0]
}

impl Operation for DeserializeObjectOp {
    fn name(&self) -> &str {
        "deserialize_object"
    }

    fn id(&self) -> Id {
        self.id
    }

    fn inputs(&self) -> Vec<Value> {
        vec![self.deserializer]
    }

    fn outputs(&self) -> Vec<Value> {
        vec![self.id.value(0)]
    }

    fn regions(&self) -> Vec<&Region> {
        vec![]
    }

    fn attributes(&self) -> Vec<String> {
        vec![self.ty.to_token_stream().to_string()]
    }

    fn to_token_stream(&self) -> TokenStream {
        let deserializer = &self.deserializer;
        let ty = &self.ty;
        quote! { <#ty as #DESERIALIZE_TRAIT>::deserialize(#deserializer)}
    }
}

//------------------------------------------------------------------------------
// Deserialize composite
//------------------------------------------------------------------------------

pub struct DeserializeCompositeOp {
    id: Id,
    deserializer: Value,
    body: Region,
}

pub fn deserialize_composite(region: &mut Region, deserializer: Value, body: impl FnOnce(&mut Region, Value)) -> Value {
    let body = {
        let mut region = Region::new(1);
        let de_inner = region.arguments()[0];
        body(&mut region, de_inner);
        region
    };
    region.push(DeserializeCompositeOp { id: Id::new(), deserializer, body })[0]
}

impl Operation for DeserializeCompositeOp {
    fn name(&self) -> &str {
        "deserialize_composite"
    }

    fn id(&self) -> Id {
        self.id
    }

    fn inputs(&self) -> Vec<Value> {
        vec![self.deserializer]
    }

    fn outputs(&self) -> Vec<Value> {
        vec![self.id.value(0)]
    }

    fn regions(&self) -> Vec<&Region> {
        vec![&self.body]
    }

    fn attributes(&self) -> Vec<String> {
        vec![]
    }

    fn to_token_stream(&self) -> TokenStream {
        let deserializer = &self.deserializer;
        let body = &self.body;
        let inner_deserializer = self.body.arguments()[0];
        quote! {
            #DESERIALIZER_TRAIT::deserialize_composite(#deserializer, |#inner_deserializer| {
                #body
            })
        }
    }
}

//------------------------------------------------------------------------------
// Deserialize byte order
//------------------------------------------------------------------------------

struct ByteOrderOp {
    id: Id,
    serializer: Value,
    byte_order: ByteOrder,
    is_serializing: bool,
    body: Region,
}

pub fn byte_order(
    region: &mut Region,
    serializer: Value,
    byte_order: ByteOrder,
    is_serializing: bool,
    body: impl FnOnce(&mut Region, Value),
) -> Value {
    let body = {
        let mut region = Region::new(1);
        let de_inner = region.arguments()[0];
        body(&mut region, de_inner);
        region
    };
    region.push(ByteOrderOp { id: Id::new(), serializer, byte_order, is_serializing, body })[0]
}

impl Operation for ByteOrderOp {
    fn name(&self) -> &str {
        "byte_order"
    }

    fn id(&self) -> Id {
        self.id
    }

    fn inputs(&self) -> Vec<Value> {
        vec![self.serializer]
    }

    fn outputs(&self) -> Vec<Value> {
        vec![self.id.value(0)]
    }

    fn regions(&self) -> Vec<&Region> {
        vec![&self.body]
    }

    fn attributes(&self) -> Vec<String> {
        vec![format!("{:?}", self.byte_order)]
    }

    fn to_token_stream(&self) -> TokenStream {
        use crate::attribute::ByteOrder::*;
        let se = &self.serializer;
        let body = &self.body;
        let inner = self.body.arguments()[0];
        let trait_ = match self.is_serializing {
            true => quote! { #SERIALIZER_TRAIT },
            false => quote! { #DESERIALIZER_TRAIT },
        };
        match self.byte_order {
            BigEndian => quote! {
                #trait_::with_byte_order(#se, #BIG_ENDIAN, |#inner| {
                    #body
                })
            },
            LittleEndian => quote! {
                #trait_::with_byte_order(#se, #LITTLE_ENDIAN, |#inner| {
                    #body
                })
            },
        }
    }
}
