use std::iter::once;

use itertools::Either;
use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
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
        TokenStream::new()
    }

    pub fn derive_deserialize(&self) -> TokenStream {
        TokenStream::new()
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
                #[sorbit::bit_field(A, repr(u8), bits(0..4))]
                #[sorbit::layout(A, align=8)]
                foo: u8,
            }
        };
        assert!(Struct::parse(&input).is_err());
    }

    #[test]
    fn parse_named_struct() -> Result<(), syn::Error> {
        let input: DeriveInput = parse_quote! {
            #[sorbit::layout(len=16)]
            #[sorbit::bit_field(A, repr(u8), round=4)]
            struct Test {
                #[sorbit::bit_field(A, bits(0..4))]
                foo: u8,
                #[sorbit::bit_field(A, bits(4..8))]
                bar: i8,
                #[sorbit::layout(align=8)]
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
            #[sorbit::layout(len=16)]
            #[sorbit::bit_field(A, repr(u8), round=4)]
            struct Test (
                #[sorbit::bit_field(A, bits(0..4))]
                u8,
                #[sorbit::bit_field(A, bits(4..8))]
                i8,
                #[sorbit::layout(align=8)]
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
}
