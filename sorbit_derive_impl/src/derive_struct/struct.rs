use std::iter::once;

use itertools::Either;
use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::parse_quote;
use syn::{DeriveInput, Generics, Ident, spanned::Spanned};

use crate::derive_struct::binary_field::BinaryField;
use crate::derive_struct::bit_field::BitField;
use crate::derive_struct::bit_field_attribute::BitFieldAttribute;
use crate::derive_struct::source_field::SourceField;
use crate::derive_struct::struct_attribute::StructAttribute;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Struct {
    name: Ident,
    generics: Generics,
    attributes: StructAttribute,
    fields: Vec<BinaryField>,
}

impl Struct {
    pub fn parse(input: &DeriveInput) -> Result<Self, syn::Error> {
        let syn::Data::Struct(data) = &input.data else {
            return Err(syn::Error::new(input.span(), "expected a struct"));
        };
        let name = input.ident.clone();
        let generics = input.generics.clone();
        let attributes = StructAttribute::parse(input.attrs.iter())?;

        // Collect all bit field storage declarations.
        let mut bit_fields_attrs = {
            let attrs = data.fields.iter().map(|field| field.attrs.iter()).chain(once(input.attrs.iter())).flatten();
            BitFieldAttribute::parse_all(attrs)
        }?;

        // Parse each declared field of the struct.
        let declared_fields = data
            .fields
            .iter()
            .enumerate()
            .map(|(index, field)| SourceField::parse(field, index))
            .collect::<Result<Vec<_>, _>>()?;

        use itertools::Itertools as _;

        // Group the declared fields together if they belong to the same bit field storage.
        let grouped_declared_fields = declared_fields.into_iter().chunk_by(|field| match field {
            SourceField::Direct(field) => field.name.to_token_stream().to_string(),
            SourceField::Packed(field) => field.attribute.storage.to_string(),
        });

        // Create fields and bit fields of the struct in order.
        let fields = grouped_declared_fields.into_iter().map(|(group, fields)| -> Result<BinaryField, syn::Error> {
            let (mut direct_fields, packed_fields): (Vec<_>, Vec<_>) = fields.partition_map(|field| match field {
                SourceField::Direct(field) => Either::Left(field),
                SourceField::Packed(field) => Either::Right(field),
            });
            if let Some(direct) = direct_fields.pop() {
                assert!(direct_fields.is_empty(), "each direct field should have its own group");
                Ok(BinaryField::Direct(direct))
            } else {
                let bit_field_attr = bit_fields_attrs.remove(&group).ok_or(syn::Error::new(
                    packed_fields[0].attribute.storage.span(),
                    "bit field storage not found; members of the same bit fields must follow each other",
                ))?;
                Ok(BinaryField::Bit(BitField {
                    name: Ident::new(&group, Span::call_site()),
                    attribute: bit_field_attr,
                    members: packed_fields,
                }))
            }
        });
        let fields = fields.collect::<Result<Vec<_>, _>>()?;

        Ok(Self { name, generics, attributes, fields })
    }

    pub fn derive_serialize(&self) -> TokenStream {
        let name = &self.name;
        let serialize_trait = quote! { ::sorbit::serialize::Serialize };
        let serializer_trait = quote! { ::sorbit::serialize::Serializer };
        let serializer_arg: syn::Expr = parse_quote! { serializer };
        let serializer_ty: syn::Type = parse_quote! { S };
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();
        let fields = self.fields.iter().map(|field| field.derive_serialize());

        let len = match self.attributes.len {
            Some(len) => quote! { #serializer_trait::pad(#serializer_arg, #len)?; },
            None => quote! {},
        };

        let round = match self.attributes.round {
            Some(round) => quote! { #serializer_trait::align(#serializer_arg, #round)?; },
            None => quote! {},
        };

        quote! {
            impl #impl_generics #serialize_trait for #name #ty_generics #where_clause{
                fn serialize<#serializer_ty: #serializer_trait>(
                    &self,
                    #serializer_arg: &mut #serializer_ty
                ) -> ::core::result::Result<#serializer_ty::Success, #serializer_ty::Error> {
                    #serializer_trait::serialize_composite(#serializer_arg, |#serializer_arg| {
                        #(#fields?;)*
                        #len
                        #round
                        #serializer_trait::serialize_nothing(#serializer_arg)
                    })
                }
            }
        }
    }

    pub fn derive_deserialize(&self) -> TokenStream {
        let name = &self.name;
        let deserialize_trait = quote! { ::sorbit::deserialize::Deserialize };
        let deserializer_trait = quote! { ::sorbit::deserialize::Deserializer };
        let deserializer_arg = quote! { deserializer };
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();
        quote! {
            impl #impl_generics #deserialize_trait for #name #ty_generics #where_clause{
                fn deserialize<D: #deserializer_trait>(
                    #deserializer_arg: &mut D
                ) -> ::core::result::Result<Self, D::Error> {
                    todo!()
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::derive_struct::{
        direct_field::DirectField, direct_field_attribute::DirectFieldAttribute, packed_field::PackedField,
        packed_field_attribute::PackedFieldAttribute,
    };

    use super::*;

    use syn::{DeriveInput, parse_quote};

    #[test]
    fn parse_ambiguous_field_kind() {
        let input: DeriveInput = parse_quote! {
            struct Test {
                #[sorbit_bit_field(A, repr(u8), bits(0..4))]
                #[sorbit_layout(A, align=8)]
                foo: u8,
            }
        };
        assert!(Struct::parse(&input).is_err());
    }

    #[test]
    fn parse_named_struct() -> Result<(), syn::Error> {
        let input: DeriveInput = parse_quote! {
            #[sorbit_layout(len=16)]
            #[sorbit_bit_field(A, repr(u8), round=4)]
            struct Test {
                #[sorbit_bit_field(A, bits(0..4))]
                foo: u8,
                #[sorbit_bit_field(A, bits(4..8))]
                bar: i8,
                #[sorbit_layout(align=8)]
                baz: u32,
            }
        };
        let desc = Struct::parse(&input)?;
        let expected = Struct {
            name: parse_quote!(Test),
            generics: Generics::default(),
            attributes: StructAttribute { len: Some(16), round: None },
            fields: vec![
                BinaryField::Bit(BitField {
                    name: parse_quote!(A),
                    attribute: BitFieldAttribute { repr: parse_quote!(u8), offset: None, align: None, round: Some(4) },
                    members: vec![
                        PackedField {
                            name: parse_quote!(foo),
                            ty: parse_quote!(u8),
                            attribute: PackedFieldAttribute { storage: parse_quote!(A), bits: 0..4 },
                        },
                        PackedField {
                            name: parse_quote!(bar),
                            ty: parse_quote!(i8),
                            attribute: PackedFieldAttribute { storage: parse_quote!(A), bits: 4..8 },
                        },
                    ],
                }),
                BinaryField::Direct(DirectField {
                    name: parse_quote!(baz),
                    ty: parse_quote!(u32),
                    attribute: DirectFieldAttribute { offset: None, align: Some(8), round: None },
                }),
            ],
        };
        assert_eq!(desc, expected);
        Ok(())
    }

    #[test]
    fn parse_tuple_struct() -> Result<(), syn::Error> {
        let input: DeriveInput = parse_quote! {
            #[sorbit_layout(len=16)]
            #[sorbit_bit_field(A, repr(u8), round=4)]
            struct Test (
                #[sorbit_bit_field(A, bits(0..4))]
                u8,
                #[sorbit_bit_field(A, bits(4..8))]
                i8,
                #[sorbit_layout(align=8)]
                u32,
            );
        };
        let desc = Struct::parse(&input)?;
        let expected = Struct {
            name: parse_quote!(Test),
            generics: Generics::default(),
            attributes: StructAttribute { len: Some(16), round: None },
            fields: vec![
                BinaryField::Bit(BitField {
                    name: parse_quote!(A),
                    attribute: BitFieldAttribute { repr: parse_quote!(u8), offset: None, align: None, round: Some(4) },
                    members: vec![
                        PackedField {
                            name: parse_quote!(0),
                            ty: parse_quote!(u8),
                            attribute: PackedFieldAttribute { storage: parse_quote!(A), bits: 0..4 },
                        },
                        PackedField {
                            name: parse_quote!(1),
                            ty: parse_quote!(i8),
                            attribute: PackedFieldAttribute { storage: parse_quote!(A), bits: 4..8 },
                        },
                    ],
                }),
                BinaryField::Direct(DirectField {
                    name: parse_quote!(2),
                    ty: parse_quote!(u32),
                    attribute: DirectFieldAttribute { offset: None, align: Some(8), round: None },
                }),
            ],
        };
        assert_eq!(desc, expected);
        Ok(())
    }

    #[test]
    fn derive_serialize_generic() {
        #[rustfmt::skip]
        let input: DeriveInput = parse_quote!(
            struct Ignore<'x, T: Clone>
            where
                T: Default {}
        );

        let input = Struct {
            name: parse_quote!(Test),
            generics: input.generics,
            attributes: StructAttribute::default(),
            fields: vec![],
        };

        let output = input.derive_serialize();
        let expected = quote! {
            impl<'x, T: Clone> ::sorbit::serialize::Serialize for Test<'x, T>
                where T: Default
            {
                fn serialize<S: ::sorbit::serialize::Serializer>(
                    &self,
                    serializer: &mut S
                ) -> ::core::result::Result<S::Success, S::Error> {
                    ::sorbit::serialize::Serializer::serialize_composite(serializer, |serializer| {
                        ::sorbit::serialize::Serializer::serialize_nothing(serializer)
                    })
                }
            }
        };
        assert_eq!(output.to_string(), expected.to_string());
    }

    #[test]
    fn derive_serialize_len_and_round() {
        let input = Struct {
            name: parse_quote!(Test),
            generics: Generics::default(),
            attributes: StructAttribute { len: Some(12), round: Some(8) },
            fields: vec![],
        };

        let output = input.derive_serialize();
        let expected = quote! {
            impl ::sorbit::serialize::Serialize for Test
            {
                fn serialize<S: ::sorbit::serialize::Serializer>(
                    &self,
                    serializer: &mut S
                ) -> ::core::result::Result<S::Success, S::Error> {
                    ::sorbit::serialize::Serializer::serialize_composite(serializer, |serializer| {
                        ::sorbit::serialize::Serializer::pad(serializer, 12u64)?;
                        ::sorbit::serialize::Serializer::align(serializer, 8u64)?;
                        ::sorbit::serialize::Serializer::serialize_nothing(serializer)
                    })
                }
            }
        };
        assert_eq!(output.to_string(), expected.to_string());
    }

    #[test]
    fn derive_serialize_direct_fields() {
        let input = Struct {
            name: parse_quote!(Test),
            generics: Generics::default(),
            attributes: StructAttribute::default(),
            fields: vec![
                BinaryField::Direct(DirectField {
                    name: parse_quote!(foo),
                    ty: parse_quote!(u8),
                    attribute: DirectFieldAttribute::default(),
                }),
                BinaryField::Direct(DirectField {
                    name: parse_quote!(bar),
                    ty: parse_quote!(i8),
                    attribute: DirectFieldAttribute::default(),
                }),
            ],
        };

        let output = input.derive_serialize();
        let expected = quote! {
            impl ::sorbit::serialize::Serialize for Test {
                fn serialize<S: ::sorbit::serialize::Serializer>(
                    &self,
                    serializer: &mut S
                ) -> ::core::result::Result<S::Success, S::Error> {
                    ::sorbit::serialize::Serializer::serialize_composite(serializer, |serializer| {
                        ::sorbit::serialize::Serialize::serialize(&self.foo, serializer).map_err(|err| ::sorbit::error::SerializeError::enclose(err, "foo"))?;
                        ::sorbit::serialize::Serialize::serialize(&self.bar, serializer).map_err(|err| ::sorbit::error::SerializeError::enclose(err, "bar"))?;
                        ::sorbit::serialize::Serializer::serialize_nothing(serializer)
                    })
                }
            }
        };
        assert_eq!(output.to_string(), expected.to_string());
    }

    #[test]
    fn derive_deserialize_empty() {
        let input = Struct {
            name: parse_quote!(Test),
            generics: Generics::default(),
            attributes: StructAttribute::default(),
            fields: vec![],
        };

        let output = input.derive_deserialize();
        let expected = quote! {
            impl ::sorbit::deserialize::Deserialize for Test {
                fn deserialize<D: ::sorbit::deserialize::Deserializer>(
                    deserializer: &mut D
                ) -> ::core::result::Result<Self, D::Error> {
                    todo!()
                }
            }
        };
        assert_eq!(output.to_string(), expected.to_string());
    }

    #[test]
    fn derive_deserialize_generic() {
        #[rustfmt::skip]
        let input: DeriveInput = parse_quote!(
            struct Ignore<'x, T: Clone>
            where
                T: Default {}
        );

        let input = Struct {
            name: parse_quote!(Test),
            generics: input.generics,
            attributes: StructAttribute::default(),
            fields: vec![],
        };

        let output = input.derive_deserialize();
        let expected = quote! {
            impl<'x, T: Clone> ::sorbit::deserialize::Deserialize for Test<'x, T>
                where T: Default
            {
                fn deserialize<D: ::sorbit::deserialize::Deserializer>(
                    deserializer: &mut D
                ) -> ::core::result::Result<Self, D::Error> {
                    todo!()
                }
            }
        };
        assert_eq!(output.to_string(), expected.to_string());
    }
}
