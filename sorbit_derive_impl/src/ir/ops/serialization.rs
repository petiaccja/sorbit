use proc_macro2::TokenStream;
use quote::{ToTokens, quote};

use crate::attribute::{BitNumbering, ByteOrder};
use crate::ir::constants::{
    BIG_ENDIAN, BIT_FIELD_TYPE, DESERIALIZE_TRAIT, DESERIALIZER_TRAIT, LITTLE_ENDIAN, SERIALIZE_TRAIT,
    SERIALIZER_TRAIT, TRACE_ERROR_TRAIT,
};
use crate::ir::dag::{Id, Operation, Region, Value};

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
// Serialize nothing
//------------------------------------------------------------------------------

struct SerializeNothingOp {
    id: Id,
    serializer: Value,
}

pub fn serialize_nothing(region: &mut Region, serializer: Value) -> Value {
    region.push(SerializeNothingOp { id: Id::new(), serializer })[0]
}

impl Operation for SerializeNothingOp {
    fn name(&self) -> &str {
        "serialize_nothing"
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
        quote! { #SERIALIZER_TRAIT::serialize_nothing(#serializer) }
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

//------------------------------------------------------------------------------
// Empty bit field
//------------------------------------------------------------------------------

struct EmptyBitFieldOp {
    id: Id,
    packed_ty: syn::Type,
}

pub fn empty_bit_field(region: &mut Region, packed_ty: syn::Type) -> Value {
    region.push(EmptyBitFieldOp { id: Id::new(), packed_ty })[0]
}

impl Operation for EmptyBitFieldOp {
    fn name(&self) -> &str {
        "empty_bit_field"
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
        vec![]
    }

    fn attributes(&self) -> Vec<String> {
        vec![self.packed_ty.to_token_stream().to_string()]
    }

    fn to_token_stream(&self) -> TokenStream {
        let packed_ty = &self.packed_ty;
        quote! { #BIT_FIELD_TYPE::<#packed_ty>::new() }
    }
}

//------------------------------------------------------------------------------
// Into bit field
//------------------------------------------------------------------------------

struct IntoBitFieldOp {
    id: Id,
    packed: Value,
}

pub fn into_bit_field(region: &mut Region, packed: Value) -> Value {
    region.push(IntoBitFieldOp { id: Id::new(), packed })[0]
}

impl Operation for IntoBitFieldOp {
    fn name(&self) -> &str {
        "into_bit_field"
    }

    fn id(&self) -> Id {
        self.id
    }

    fn inputs(&self) -> Vec<Value> {
        vec![self.packed]
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
        let packed = &self.packed;
        quote! { #BIT_FIELD_TYPE::from_bits(#packed) }
    }
}

//------------------------------------------------------------------------------
// Into raw bits
//------------------------------------------------------------------------------

struct IntoRawBitsOp {
    id: Id,
    bit_field: Value,
}

pub fn into_raw_bits(region: &mut Region, bit_field: Value) -> Value {
    region.push(IntoRawBitsOp { id: Id::new(), bit_field })[0]
}

impl Operation for IntoRawBitsOp {
    fn name(&self) -> &str {
        "into_raw_bits"
    }

    fn id(&self) -> Id {
        self.id
    }

    fn inputs(&self) -> Vec<Value> {
        vec![self.bit_field]
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
        let bit_field = &self.bit_field;
        quote! { #bit_field.into_bits() }
    }
}

//------------------------------------------------------------------------------
// Pack bit field
//------------------------------------------------------------------------------

struct PackBitFieldOp {
    id: Id,
    value: Value,
    bit_field: Value,
    bits: std::ops::Range<u8>,
    bit_numbering: BitNumbering,
}

pub fn pack_bit_field(
    region: &mut Region,
    value: Value,
    bit_field: Value,
    bits: std::ops::Range<u8>,
    bit_numbering: BitNumbering,
) -> Value {
    region.push(PackBitFieldOp { id: Id::new(), value, bit_field, bits, bit_numbering })[0]
}

impl Operation for PackBitFieldOp {
    fn name(&self) -> &str {
        "pack_bit_field"
    }

    fn id(&self) -> Id {
        self.id
    }

    fn inputs(&self) -> Vec<Value> {
        vec![self.value, self.bit_field]
    }

    fn outputs(&self) -> Vec<Value> {
        vec![self.id.value(0)]
    }

    fn regions(&self) -> Vec<&Region> {
        vec![]
    }

    fn attributes(&self) -> Vec<String> {
        vec![
            format!("{}..{}", self.bits.start, self.bits.end),
            format!("{:?}", self.bit_numbering),
        ]
    }

    fn to_token_stream(&self) -> TokenStream {
        let value = &self.value;
        let bit_field = &self.bit_field;
        let start = self.bits.start;
        let end = self.bits.end;
        let bit_range = bit_range_to_token_stream(quote! {bit_field}, start, end, self.bit_numbering);
        quote! {
            {
                let mut bit_field = #bit_field;
                bit_field.pack(&#value, #bit_range)
                          .map_err(|err| err.into())
                          .map(|_| bit_field)
            }
        }
    }
}

fn bit_range_to_token_stream(bit_field: impl ToTokens, start: u8, end: u8, bit_numbering: BitNumbering) -> TokenStream {
    let bit_range = match bit_numbering {
        BitNumbering::MSB0 => {
            quote! { (#bit_field.bit_size_of() as u8 - #end)..(#bit_field.bit_size_of() as u8 - #start) }
        }
        BitNumbering::LSB0 => quote! { #start..#end },
    };
    bit_range
}

//------------------------------------------------------------------------------
// Unpack bit field
//------------------------------------------------------------------------------

struct UnpackBitFieldOp {
    id: Id,
    bit_field: Value,
    ty: syn::Type,
    bits: std::ops::Range<u8>,
    bit_numbering: BitNumbering,
}

pub fn unpack_bit_field(
    region: &mut Region,
    bit_field: Value,
    ty: syn::Type,
    bits: std::ops::Range<u8>,
    bit_numbering: BitNumbering,
) -> Value {
    region.push(UnpackBitFieldOp { id: Id::new(), bit_field, ty, bits, bit_numbering })[0]
}

impl Operation for UnpackBitFieldOp {
    fn name(&self) -> &str {
        "unpack_bit_field"
    }

    fn id(&self) -> Id {
        self.id
    }

    fn inputs(&self) -> Vec<Value> {
        vec![self.bit_field]
    }

    fn outputs(&self) -> Vec<Value> {
        vec![self.id.value(0)]
    }

    fn regions(&self) -> Vec<&Region> {
        vec![]
    }

    fn attributes(&self) -> Vec<String> {
        vec![
            self.ty.to_token_stream().to_string(),
            format!("{}..{}", self.bits.start, self.bits.end),
            format!("{:?}", self.bit_numbering),
        ]
    }

    fn to_token_stream(&self) -> TokenStream {
        let bit_field = &self.bit_field;
        let ty = &self.ty;
        let start = self.bits.start;
        let end = self.bits.end;
        let bit_range = bit_range_to_token_stream(bit_field, start, end, self.bit_numbering);
        quote! { #bit_field.unpack::<#ty, _, _>(#bit_range).map_err(|err| err.into()) }
    }
}
