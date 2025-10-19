use proc_macro2::TokenStream;
use quote::quote;
use syn::{Expr, Index, Member, Type};

use crate::derive_struct::direct_field_attribute::DirectFieldAttribute;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectField {
    pub name: Member,
    pub ty: Type,
    pub attribute: DirectFieldAttribute,
}

impl DirectField {
    pub fn parse(field: &syn::Field, index: usize) -> Result<Self, syn::Error> {
        let attribute = DirectFieldAttribute::parse(field.attrs.iter())?;
        let name = match &field.ident {
            Some(ident) => Member::Named(ident.clone()),
            None => Member::Unnamed(Index::from(index)),
        };
        let ty = field.ty.clone();
        Ok(Self { name, ty, attribute })
    }

    pub fn derive_serialize(&self, parent: &Expr, serializer: &Expr) -> TokenStream {
        let member = &self.name;
        let serialize_trait = quote! { ::sorbit::serialize::Serialize };
        let serializer_trait = quote! { ::sorbit::serialize::Serializer };

        // The basic expression to serialize the field without any parameters.
        let expr = quote! { #serialize_trait::serialize(&#parent.#member, #serializer) };

        // If the field needs to be rounded, wrap the serialization in a composite and align it.
        let expr = match self.attribute.round {
            Some(round) => quote! {
                #serializer_trait::serialize_composite(serializer, |#serializer| {
                    #expr.and(#serializer_trait::align(#serializer, #round))
                })
            },
            None => expr,
        };

        // If the field needs alignment, align the stream before serializing the
        // field. The alignment is deliberately applied AFTER the offset,
        // because the offset may not be aligned. (Don't get confused, the
        // alignment expression is built BEFORE the offset expression, the
        // order is reversed.)
        let expr = match self.attribute.align {
            Some(align) => quote! { #serializer_trait::align(#serializer, #align).and(#expr) },
            None => expr,
        };

        // If the field is at an absolute offset, pad the stream before serializing the field.
        let expr = match self.attribute.offset {
            Some(offset) => quote! { #serializer_trait::pad(#serializer, #offset).and(#expr) },
            None => expr,
        };

        expr
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use syn::parse_quote;

    #[test]
    fn parse_trivial() -> Result<(), syn::Error> {
        let input: syn::Field = parse_quote! {
            foo: u8
        };
        let field = DirectField::parse(&input, 0)?;
        let expected =
            DirectField { name: parse_quote!(foo), ty: parse_quote!(u8), attribute: DirectFieldAttribute::default() };
        assert_eq!(field, expected);
        Ok(())
    }

    #[test]
    fn parse_layout() -> Result<(), syn::Error> {
        let input: syn::Field = parse_quote! {
            #[sorbit_layout(align=8)]
            foo: u8
        };
        let field = DirectField::parse(&input, 0)?;
        let expected = DirectField {
            name: parse_quote!(foo),
            ty: parse_quote!(u8),
            attribute: DirectFieldAttribute { offset: None, align: Some(8), round: None },
        };
        assert_eq!(field, expected);
        Ok(())
    }

    #[test]
    fn parse_bit_field_decl() {
        let input: syn::Field = parse_quote! {
            #[sorbit_bit_field(A, align=8)]
            foo: u8
        };
        assert!(DirectField::parse(&input, 0).is_err());
    }

    #[test]
    fn parse_bit_field_bits() {
        let input: syn::Field = parse_quote! {
            #[sorbit_bit_field(A, bits(4..8))]
            foo: u8
        };
        assert!(DirectField::parse(&input, 0).is_err());
    }

    #[test]
    fn derive_serialize_no_parameters() {
        let input =
            DirectField { name: parse_quote!(foo), ty: parse_quote!(i32), attribute: DirectFieldAttribute::default() };
        let expected = quote! { ::sorbit::serialize::Serialize::serialize(&self.foo, serializer) };
        let output = input.derive_serialize(&parse_quote!(self), &parse_quote!(serializer));
        assert_eq!(output.to_string(), expected.to_string());
    }

    #[test]
    fn derive_serialize_offset_and_align() {
        let input = DirectField {
            name: parse_quote!(foo),
            ty: parse_quote!(i32),
            attribute: DirectFieldAttribute { offset: Some(4), align: Some(6), round: None },
        };
        let expected = quote! {
            ::sorbit::serialize::Serializer::pad(serializer, 4u64).and(
                ::sorbit::serialize::Serializer::align(serializer, 6u64).and(
                    ::sorbit::serialize::Serialize::serialize(&self.foo, serializer)
                )
            )
        };
        let output = input.derive_serialize(&parse_quote!(self), &parse_quote!(serializer));
        assert_eq!(output.to_string(), expected.to_string());
    }

    #[test]
    fn derive_serialize_round() {
        let input = DirectField {
            name: parse_quote!(foo),
            ty: parse_quote!(i32),
            attribute: DirectFieldAttribute { offset: None, align: None, round: Some(16) },
        };
        let expected = quote! {
            ::sorbit::serialize::Serializer::serialize_composite(serializer, |serializer| {
                ::sorbit::serialize::Serialize::serialize(&self.foo, serializer).and(
                    ::sorbit::serialize::Serializer::align(serializer, 16u64)
                )
            })
        };
        let output = input.derive_serialize(&parse_quote!(self), &parse_quote!(serializer));
        assert_eq!(output.to_string(), expected.to_string());
    }
}
