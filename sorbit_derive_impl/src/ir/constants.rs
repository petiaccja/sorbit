use quote::{ToTokens, quote};

pub struct BitFieldType;

#[allow(unused)]
pub struct ErrorTrait;

pub struct SerializerTrait;
pub struct SerializerOutputTrait;
pub struct SerializerType;
pub struct SerializeTrait;

pub struct DeserializerTrait;
pub struct DeserializerType;
pub struct DeserializeTrait;

pub const BIT_FIELD_TYPE: BitFieldType = BitFieldType {};

#[allow(unused)]
pub const ERROR_TRAIT: ErrorTrait = ErrorTrait {};

pub const SERIALIZER_TRAIT: SerializerTrait = SerializerTrait {};
pub const SERIALIZER_OUTPUT_TRAIT: SerializerOutputTrait = SerializerOutputTrait {};
pub const SERIALIZER_TYPE: SerializerType = SerializerType {};
pub const SERIALIZE_TRAIT: SerializeTrait = SerializeTrait {};

pub const DESERIALIZER_TRAIT: DeserializerTrait = DeserializerTrait {};
pub const DESERIALIZER_TYPE: DeserializerType = DeserializerType {};
pub const DESERIALIZE_TRAIT: DeserializeTrait = DeserializeTrait {};

impl ToTokens for BitFieldType {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend(quote! {::sorbit::bit::BitField});
    }
}

impl ToTokens for ErrorTrait {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend(quote! {::sorbit::error::SerializeError});
    }
}

impl ToTokens for SerializerTrait {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend(quote! {::sorbit::serialize::Serializer});
    }
}

impl ToTokens for SerializerOutputTrait {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend(quote! {::sorbit::serialize::SerializerOutput});
    }
}

impl ToTokens for SerializerType {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend(quote! {S});
    }
}

impl ToTokens for SerializeTrait {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend(quote! {::sorbit::serialize::Serialize});
    }
}

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
