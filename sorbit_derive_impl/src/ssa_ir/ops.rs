use std::ops::Range;

use crate::ir::constants::*;
use proc_macro2::TokenStream;
use quote::{ToTokens as _, quote};

use super::ir::{Operation, Region, Value};

//------------------------------------------------------------------------------
// ImplSerializeOp
//------------------------------------------------------------------------------

pub struct ImplSerializeOp {
    pub operation: Operation,
}

impl ImplSerializeOp {
    pub fn new(name: syn::Ident, generics: syn::Generics, body: Region) -> Self {
        let to_token_stream = move |operation: &Operation| Self::to_token_stream(operation, &name, &generics);
        Self {
            operation: Operation::new(
                "impl_serialize".into(),
                vec![],
                Box::new(to_token_stream),
                0,
                vec![],
                vec![body],
            ),
        }
    }

    pub fn to_token_stream(operation: &Operation, name: &syn::Ident, generics: &syn::Generics) -> TokenStream {
        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
        let body = &operation.regions[0];
        let serializer = body.argument(0);
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
// ImplDeserializeOp
//------------------------------------------------------------------------------

pub struct ImplDeserializeOp {
    pub operation: Operation,
}

impl ImplDeserializeOp {
    pub fn new(name: syn::Ident, generics: syn::Generics, body: Region) -> Self {
        let to_token_stream = move |operation: &Operation| Self::to_token_stream(operation, &name, &generics);
        Self {
            operation: Operation::new(
                "impl_deserialize".into(),
                vec![],
                Box::new(to_token_stream),
                0,
                vec![],
                vec![body],
            ),
        }
    }

    pub fn to_token_stream(operation: &Operation, name: &syn::Ident, generics: &syn::Generics) -> TokenStream {
        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
        let body = &operation.regions[0];
        let deserializer = body.argument(0);
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

//------------------------------------------------------------------------------
// SelfOp
//------------------------------------------------------------------------------

pub struct SelfOp {
    pub operation: Operation,
}

impl SelfOp {
    pub fn new() -> Self {
        Self { operation: Operation::new("self".into(), vec![], Box::new(Self::to_token_stream), 1, vec![], vec![]) }
    }

    pub fn to_token_stream(_operation: &Operation) -> TokenStream {
        quote! { self }
    }

    pub fn output(&self) -> Value {
        self.operation.output(0)
    }
}

//------------------------------------------------------------------------------
// RefOp
//------------------------------------------------------------------------------

pub struct RefOp {
    pub operation: Operation,
}

impl RefOp {
    pub fn new(value: Value) -> Self {
        Self {
            operation: Operation::new("ref".into(), vec![], Box::new(Self::to_token_stream), 1, vec![value], vec![]),
        }
    }

    pub fn to_token_stream(operation: &Operation) -> TokenStream {
        let value = &operation.inputs[0];
        quote! { &#value }
    }

    pub fn output(&self) -> Value {
        self.operation.output(0)
    }
}

//------------------------------------------------------------------------------
// YieldOp
//------------------------------------------------------------------------------

pub struct YieldOp {
    pub operation: Operation,
}

impl YieldOp {
    pub fn new(values: Vec<Value>) -> Self {
        Self {
            operation: Operation::new("yield".into(), vec![], Box::new(Self::to_token_stream), 0, values, vec![]),
        }
    }

    pub fn to_token_stream(operation: &Operation) -> TokenStream {
        let values = &operation.inputs;
        match values.len() {
            0 => quote! { () },
            1 => quote! { #(#values)* },
            _ => quote! { (#(#values),*) },
        }
    }
}

//------------------------------------------------------------------------------
// ExecuteOp
//------------------------------------------------------------------------------

pub struct ExecuteOp {
    pub operation: Operation,
}

impl ExecuteOp {
    pub fn new(region: Region) -> Self {
        let num_outputs = if let Some(last_op) = region.operations().last() {
            if last_op.mnemonic() == "yield" { last_op.inputs.len() } else { 0 }
        } else {
            0
        };
        Self {
            operation: Operation::new(
                "execute".into(),
                vec![],
                Box::new(Self::to_token_stream),
                num_outputs,
                vec![],
                vec![region],
            ),
        }
    }

    pub fn to_token_stream(operation: &Operation) -> TokenStream {
        let region = &operation.regions[0];
        quote! { #region }
    }

    #[allow(unused)]
    pub fn output(&self, index: usize) -> Value {
        self.operation.output(index)
    }
}

//------------------------------------------------------------------------------
// MemberOp
//------------------------------------------------------------------------------

pub struct MemberOp {
    pub operation: Operation,
}

impl MemberOp {
    pub fn new(value: Value, member: syn::Member, reference: bool) -> Self {
        Self {
            operation: Operation::new(
                "member".into(),
                vec![
                    match &member {
                        syn::Member::Named(ident) => ident.to_string(),
                        syn::Member::Unnamed(index) => index.index.to_string(),
                    },
                    match reference {
                        true => "&".into(),
                        false => "*".into(),
                    },
                ],
                Box::new(move |operation: &Operation| Self::to_token_stream(operation, &member, reference)),
                1,
                vec![value],
                vec![],
            ),
        }
    }

    pub fn to_token_stream(operation: &Operation, member: &syn::Member, reference: bool) -> TokenStream {
        let value = operation.inputs[0].to_ident();
        match reference {
            false => quote! { #value.#member },
            true => quote! { &#value.#member },
        }
    }

    pub fn output(&self) -> Value {
        self.operation.output(0)
    }
}

//------------------------------------------------------------------------------
// TryOp
//------------------------------------------------------------------------------

pub struct TryOp {
    pub operation: Operation,
}

impl TryOp {
    pub fn new(value: Value) -> Self {
        Self {
            operation: Operation::new("try".into(), vec![], Box::new(Self::to_token_stream), 1, vec![value], vec![]),
        }
    }

    pub fn to_token_stream(operation: &Operation) -> TokenStream {
        let value = operation.inputs[0].to_ident();
        quote! { #value? }
    }

    pub fn output(&self) -> Value {
        self.operation.output(0)
    }
}

//------------------------------------------------------------------------------
// OkOp
//------------------------------------------------------------------------------

pub struct OkOp {
    pub operation: Operation,
}

impl OkOp {
    pub fn new(value: Value) -> Self {
        Self {
            operation: Operation::new("ok".into(), vec![], Box::new(Self::to_token_stream), 1, vec![value], vec![]),
        }
    }

    pub fn to_token_stream(operation: &Operation) -> TokenStream {
        let value = operation.inputs[0].to_ident();
        quote! { ::core::result::Result::Ok(#value) }
    }

    pub fn output(&self) -> Value {
        self.operation.output(0)
    }
}

//------------------------------------------------------------------------------
// TupleOp
//------------------------------------------------------------------------------

pub struct TupleOp {
    pub operation: Operation,
}

impl TupleOp {
    pub fn new(members: Vec<Value>) -> Self {
        Self {
            operation: Operation::new("tuple".into(), vec![], Box::new(Self::to_token_stream), 1, members, vec![]),
        }
    }

    pub fn to_token_stream(operation: &Operation) -> TokenStream {
        let members = &operation.inputs;
        quote! { (#(#members,)*) }
    }

    pub fn output(&self) -> Value {
        self.operation.output(0)
    }
}

//------------------------------------------------------------------------------
// StructOp
//------------------------------------------------------------------------------

pub struct StructOp {
    pub operation: Operation,
}

impl StructOp {
    pub fn new(ty: syn::Type, members: Vec<(syn::Member, Value)>) -> Self {
        let (names, values): (Vec<_>, Vec<_>) = members.into_iter().unzip();
        let attrs = std::iter::once(ty.to_token_stream().to_string()).chain(names.iter().map(|name| match name {
            syn::Member::Named(ident) => ident.to_string(),
            syn::Member::Unnamed(index) => index.index.to_string(),
        }));
        Self {
            operation: Operation::new(
                "struct".into(),
                attrs.collect(),
                Box::new(move |op| Self::to_token_stream(op, &ty, &names)),
                1,
                values,
                vec![],
            ),
        }
    }

    pub fn to_token_stream(operation: &Operation, ty: &syn::Type, members: &[syn::Member]) -> TokenStream {
        let values = &operation.inputs;
        quote! { #ty{ #(#members: #values,)* } }
    }

    pub fn output(&self) -> Value {
        self.operation.output(0)
    }
}

//------------------------------------------------------------------------------
// PadOp
//------------------------------------------------------------------------------

pub struct PadOp {
    pub operation: Operation,
}

impl PadOp {
    pub fn new(serializer: Value, until: u64, serializing: bool) -> Self {
        Self {
            operation: Operation::new(
                "pad".into(),
                vec![until.to_string()],
                Box::new(move |op| Self::to_token_stream(op, until, serializing)),
                1,
                vec![serializer],
                vec![],
            ),
        }
    }

    pub fn to_token_stream(operation: &Operation, until: u64, serializing: bool) -> TokenStream {
        let serializer = &operation.inputs[0];
        match serializing {
            true => quote! { #SERIALIZER_TRAIT::pad(#serializer, #until) },
            false => quote! { #DESERIALIZER_TRAIT::pad(#serializer, #until) },
        }
    }

    pub fn output(&self) -> Value {
        self.operation.output(0)
    }
}

//------------------------------------------------------------------------------
// AlignOp
//------------------------------------------------------------------------------

pub struct AlignOp {
    pub operation: Operation,
}

impl AlignOp {
    pub fn new(serializer: Value, multiple_of: u64, serializing: bool) -> Self {
        Self {
            operation: Operation::new(
                "align".into(),
                vec![multiple_of.to_string()],
                Box::new(move |op| Self::to_token_stream(op, multiple_of, serializing)),
                1,
                vec![serializer],
                vec![],
            ),
        }
    }

    pub fn to_token_stream(operation: &Operation, multiple_of: u64, serializing: bool) -> TokenStream {
        let serializer = &operation.inputs[0];
        match serializing {
            true => quote! { #SERIALIZER_TRAIT::align(#serializer, #multiple_of) },
            false => quote! { #DESERIALIZER_TRAIT::align(#serializer, #multiple_of) },
        }
    }

    pub fn output(&self) -> Value {
        self.operation.output(0)
    }
}

//------------------------------------------------------------------------------
// SerializeNothingOp
//------------------------------------------------------------------------------

pub struct SerializeNothingOp {
    pub operation: Operation,
}

impl SerializeNothingOp {
    pub fn new(serializer: Value) -> Self {
        Self {
            operation: Operation::new(
                "serialize_nothing".into(),
                vec![],
                Box::new(Self::to_token_stream),
                1,
                vec![serializer],
                vec![],
            ),
        }
    }

    pub fn to_token_stream(operation: &Operation) -> TokenStream {
        let serializer = operation.inputs[0].to_ident();
        quote! { #SERIALIZER_TRAIT::serialize_nothing(#serializer)}
    }

    pub fn output(&self) -> Value {
        self.operation.output(0)
    }
}

//------------------------------------------------------------------------------
// SerializeObjectOp
//------------------------------------------------------------------------------

pub struct SerializeObjectOp {
    pub operation: Operation,
}

impl SerializeObjectOp {
    pub fn new(serializer: Value, object: Value) -> Self {
        Self {
            operation: Operation::new(
                "serialize_object".into(),
                vec![],
                Box::new(Self::to_token_stream),
                1,
                vec![serializer, object],
                vec![],
            ),
        }
    }

    pub fn to_token_stream(operation: &Operation) -> TokenStream {
        let serializer = &operation.inputs[0];
        let object = &operation.inputs[1];
        quote! { #SERIALIZE_TRAIT::serialize(#object, #serializer)}
    }

    pub fn output(&self) -> Value {
        self.operation.output(0)
    }
}

//------------------------------------------------------------------------------
// SerializeCompositeOp
//------------------------------------------------------------------------------

pub struct SerializeCompositeOp {
    pub operation: Operation,
}

impl SerializeCompositeOp {
    pub fn new(serializer: Value, body: Region) -> Self {
        Self {
            operation: Operation::new(
                "serialize_composite".into(),
                vec![],
                Box::new(Self::to_token_stream),
                1,
                vec![serializer],
                vec![body],
            ),
        }
    }

    pub fn output(&self) -> Value {
        self.operation.output(0)
    }

    pub fn to_token_stream(operation: &Operation) -> TokenStream {
        let serializer = operation.inputs[0].to_ident();
        let body = &operation.regions[0];
        let inner_serializer = body.argument(0).to_ident();
        quote! {
            #SERIALIZER_TRAIT::serialize_composite(#serializer, |#inner_serializer| {
                #body
            })
        }
    }
}

//------------------------------------------------------------------------------
// DeserializeObjectOp
//------------------------------------------------------------------------------

pub struct DeserializeObjectOp {
    pub operation: Operation,
}

impl DeserializeObjectOp {
    pub fn new(deserializer: Value, ty: syn::Type) -> Self {
        Self {
            operation: Operation::new(
                "deserialize_object".into(),
                vec![ty.to_token_stream().to_string()],
                Box::new(move |op| Self::to_token_stream(op, &ty)),
                1,
                vec![deserializer],
                vec![],
            ),
        }
    }

    pub fn to_token_stream(operation: &Operation, ty: &syn::Type) -> TokenStream {
        let serializer = &operation.inputs[0];
        quote! { <#ty as #DESERIALIZE_TRAIT>::deserialize(#serializer)}
    }

    pub fn output(&self) -> Value {
        self.operation.output(0)
    }
}

//------------------------------------------------------------------------------
// DeserializeCompositeOp
//------------------------------------------------------------------------------

pub struct DeserializeCompositeOp {
    pub operation: Operation,
}

impl DeserializeCompositeOp {
    pub fn new(deserializer: Value, body: Region) -> Self {
        Self {
            operation: Operation::new(
                "deserialize_composite".into(),
                vec![],
                Box::new(Self::to_token_stream),
                1,
                vec![deserializer],
                vec![body],
            ),
        }
    }

    pub fn output(&self) -> Value {
        self.operation.output(0)
    }

    pub fn to_token_stream(operation: &Operation) -> TokenStream {
        let deserializer = operation.inputs[0].to_ident();
        let body = &operation.regions[0];
        let inner_deserializer = body.argument(0).to_ident();
        quote! {
            #DESERIALIZER_TRAIT::deserialize_composite(#deserializer, |#inner_deserializer| {
                #body
            })
        }
    }
}

//------------------------------------------------------------------------------
// EmptyBitFieldOp
//------------------------------------------------------------------------------

pub struct EmptyBitFieldOp {
    pub operation: Operation,
}

impl EmptyBitFieldOp {
    pub fn new(packed_ty: syn::Type) -> Self {
        Self {
            operation: Operation::new(
                "empty_bit_field".into(),
                vec![packed_ty.to_token_stream().to_string()],
                Box::new(move |op| Self::to_token_stream(op, &packed_ty)),
                1,
                vec![],
                vec![],
            ),
        }
    }

    #[allow(unused)]
    pub fn output(&self) -> Value {
        self.operation.output(0)
    }

    pub fn to_token_stream(_operation: &Operation, packed_ty: &syn::Type) -> TokenStream {
        quote! { #BIT_FIELD_TYPE::<#packed_ty>::new() }
    }
}

//------------------------------------------------------------------------------
// IntoBitFieldOp
//------------------------------------------------------------------------------

pub struct IntoBitFieldOp {
    pub operation: Operation,
}

impl IntoBitFieldOp {
    pub fn new(packed: Value) -> Self {
        Self {
            operation: Operation::new(
                "into_bit_field".into(),
                vec![],
                Box::new(Self::to_token_stream),
                1,
                vec![packed],
                vec![],
            ),
        }
    }

    pub fn output(&self) -> Value {
        self.operation.output(0)
    }

    pub fn to_token_stream(operation: &Operation) -> TokenStream {
        let packed = &operation.inputs[0];
        quote! { #BIT_FIELD_TYPE::from_bits(#packed) }
    }
}

//------------------------------------------------------------------------------
// IntoRawBitsOp
//------------------------------------------------------------------------------

pub struct IntoRawBitsOp {
    pub operation: Operation,
}

impl IntoRawBitsOp {
    pub fn new(bit_field: Value) -> Self {
        Self {
            operation: Operation::new(
                "into_raw_bits".into(),
                vec![],
                Box::new(Self::to_token_stream),
                1,
                vec![bit_field],
                vec![],
            ),
        }
    }

    pub fn output(&self) -> Value {
        self.operation.output(0)
    }

    pub fn to_token_stream(operation: &Operation) -> TokenStream {
        let bit_field = &operation.inputs[0];
        quote! { #bit_field.into_bits() }
    }
}

//------------------------------------------------------------------------------
// PackBitFieldOp
//------------------------------------------------------------------------------

pub struct PackBitFieldOp {
    pub operation: Operation,
}

impl PackBitFieldOp {
    pub fn new(serializer: Value, value: Value, bit_field: Value, bits: Range<u8>) -> Self {
        Self {
            operation: Operation::new(
                "pack_bit_field".into(),
                vec![format!("{}..{}", bits.start, bits.end)],
                Box::new(move |op| Self::to_token_stream(op, &bits)),
                1,
                vec![serializer, value, bit_field],
                vec![],
            ),
        }
    }

    pub fn output(&self) -> Value {
        self.operation.output(0)
    }

    pub fn to_token_stream(operation: &Operation, bits: &Range<u8>) -> TokenStream {
        let serializer = &operation.inputs[0];
        let value = &operation.inputs[1];
        let bit_field = &operation.inputs[2];
        let Range { start, end } = bits;
        quote! {
            {
                let mut bit_field = #bit_field;
                bit_field.pack(&#value, #start..#end)
                          .map_err(|err| ::sorbit::codegen::bit_error_to_error_se(#serializer, err))
                          .map(|_| bit_field)
            }
        }
    }
}

//------------------------------------------------------------------------------
// UnpackBitFieldOp
//------------------------------------------------------------------------------

pub struct UnpackBitFieldOp {
    pub operation: Operation,
}

impl UnpackBitFieldOp {
    pub fn new(ty: syn::Type, bit_field: Value, bits: Range<u8>) -> Self {
        Self {
            operation: Operation::new(
                "unpack_bit_field".into(),
                vec![
                    ty.to_token_stream().to_string(),
                    format!("{}..{}", bits.start, bits.end),
                ],
                Box::new(move |op| Self::to_token_stream(op, &ty, &bits)),
                1,
                vec![bit_field],
                vec![],
            ),
        }
    }

    pub fn output(&self) -> Value {
        self.operation.output(0)
    }

    pub fn to_token_stream(operation: &Operation, ty: &syn::Type, bits: &Range<u8>) -> TokenStream {
        let bit_field = &operation.inputs[0];
        let Range { start, end } = bits;
        quote! {
            #bit_field.unpack::<#ty, _, _>(#start..#end)
                      .map_err(|err| err.into())
        }
    }
}

//------------------------------------------------------------------------------
// Tests
//------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use crate::ssa_ir::ir::assert_matches;

    use super::*;

    #[test]
    fn foo() {
        let body = Region::new(1, |arguments| {
            let serializer = &arguments[0];
            let serialize_composite = SerializeCompositeOp::new(
                serializer.clone(),
                Region::new(1, |arguments| {
                    let serializer = &arguments[0];
                    let self_ = SelfOp::new();
                    let execute = ExecuteOp::new(Region::new(0, |_| {
                        let member = MemberOp::new(self_.output(), parse_quote!(foo), true);
                        let yield_ = YieldOp::new(vec![member.output()]);
                        vec![member.operation, yield_.operation]
                    }));
                    let serialize_object = SerializeObjectOp::new(serializer.clone(), execute.output(0));
                    let yield_ = YieldOp::new(vec![serialize_object.output()]);
                    vec![
                        self_.operation,
                        execute.operation,
                        serialize_object.operation,
                        yield_.operation,
                    ]
                }),
            );
            vec![serialize_composite.operation]
        });
        let impl_serialize = ImplSerializeOp::new(parse_quote!(Foo), Default::default(), body);
        let pattern = "
            impl_serialize |%serializer| [
                %0 = serialize_composite %serializer |%comp_ser| [
                    %1 = self,
                    %2 = execute || [
                        %01 = member[foo, &] %1,
                        yield %01,
                    ],
                    %3 = serialize_object %comp_ser %2,
                    yield %3,
                ],
            ]
        ";
        assert_matches!(format!("{:#?}", impl_serialize.operation), pattern);
    }
}
