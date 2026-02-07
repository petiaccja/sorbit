use std::iter::once;

use itertools::Either;
use proc_macro2::Span;
use quote::ToTokens;
use syn::{DeriveInput, Generics, Ident, spanned::Spanned};

use crate::derive_struct::binary_field::BinaryField;
use crate::derive_struct::bit_field::BitField;
use crate::derive_struct::bit_field_attribute::BitFieldAttribute;
use crate::derive_struct::field_utils::{lower_alignment, lower_offset};
use crate::derive_struct::source_field::SourceField;
use crate::derive_struct::struct_attribute::StructAttribute;
use crate::ir::dag::Region;
use crate::ir::ops::{
    DeserializeCompositeOp, ImplDeserializeOp, ImplSerializeOp, MemberOp, OkOp, SerializeCompositeOp,
    SerializeNothingOp, StructOp, TryOp, TupleOp, YieldOp,
};

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

    pub fn lower_se(&self) -> ImplSerializeOp {
        let body = Region::new(1, |arguments| {
            let serializer = &arguments[0];
            let maybe_composite = SerializeCompositeOp::new(
                serializer.clone(),
                Region::new(1, |arguments| {
                    let serializer = &arguments[0];

                    let mut layout_ops = Vec::new();
                    lower_offset(serializer.clone(), self.attributes.len, true, &mut layout_ops);
                    lower_alignment(serializer.clone(), self.attributes.round, true, &mut layout_ops);

                    if self.fields.is_empty() {
                        let serialize_nothing = SerializeNothingOp::new(serializer.clone());
                        let yield_ = YieldOp::new(vec![serialize_nothing.output()]);
                        std::iter::once(serialize_nothing.operation)
                            .chain(layout_ops.into_iter())
                            .chain(std::iter::once(yield_.operation))
                            .collect()
                    } else {
                        let maybe_spans: Vec<_> =
                            self.fields.iter().map(|field| field.lower_se(serializer.clone())).collect();
                        let spans: Vec<_> =
                            maybe_spans.iter().map(|maybe_span| TryOp::new(maybe_span.output(0))).collect();
                        let span_tuple = TupleOp::new(spans.iter().map(|span| span.output()).collect());
                        let ok_span_tuple = OkOp::new(span_tuple.output());
                        let yield_ = YieldOp::new(vec![ok_span_tuple.output()]);
                        maybe_spans
                            .into_iter()
                            .chain(spans.into_iter().map(|span| span.operation))
                            .chain(std::iter::once(span_tuple.operation))
                            .chain(std::iter::once(ok_span_tuple.operation))
                            .chain(layout_ops.into_iter())
                            .chain(std::iter::once(yield_.operation))
                            .collect()
                    }
                }),
            );
            let composite = TryOp::new(maybe_composite.output());
            let span = MemberOp::new(composite.output(), syn::Member::Unnamed(syn::Index::from(0)), false);
            let ok_span = OkOp::new(span.output());
            let yield_ = YieldOp::new(vec![ok_span.output()]);
            vec![
                maybe_composite.operation,
                composite.operation,
                span.operation,
                ok_span.operation,
                yield_.operation,
            ]
        });
        ImplSerializeOp::new(self.name.clone(), self.generics.clone(), body)
    }

    pub fn lower_de(&self) -> ImplDeserializeOp {
        let body = Region::new(1, |arguments| {
            let deserializer = &arguments[0];
            let maybe_composite = DeserializeCompositeOp::new(
                deserializer.clone(),
                Region::new(1, |arguments| {
                    let deserializer = &arguments[0];

                    let mut layout_ops = Vec::new();
                    lower_offset(deserializer.clone(), self.attributes.len, false, &mut layout_ops);
                    lower_alignment(deserializer.clone(), self.attributes.round, false, &mut layout_ops);

                    let maybe_des_ops: Vec<_> =
                        self.fields.iter().map(|field| field.lower_de(deserializer.clone())).collect();
                    let mut des_ops = Vec::new();
                    let mut field_names = Vec::new();
                    let mut field_values = Vec::new();
                    for (field, maybe_des_op) in self.fields.iter().zip(maybe_des_ops.iter()) {
                        match field {
                            BinaryField::Direct(direct_field) => {
                                let des_op = TryOp::new(maybe_des_op.output(0));
                                field_names.push(direct_field.name.clone());
                                field_values.push(des_op.output());
                                des_ops.extend([des_op.operation].into_iter());
                            }
                            BinaryField::Bit(bit_field) => {
                                for (idx, member) in bit_field.members.iter().enumerate() {
                                    let des_op = TryOp::new(maybe_des_op.output(idx));
                                    field_names.push(member.name.clone());
                                    field_values.push(des_op.output());
                                    des_ops.extend([des_op.operation].into_iter());
                                }
                            }
                        }
                    }

                    let struct_ = StructOp::new(
                        syn::TypePath { qself: None, path: syn::Path::from(self.name.clone()) }.into(),
                        field_names.into_iter().zip(field_values.into_iter()).collect(),
                    );
                    let ok_struct = OkOp::new(struct_.output());
                    let yield_ = YieldOp::new(vec![ok_struct.output()]);

                    maybe_des_ops
                        .into_iter()
                        .chain(des_ops.into_iter())
                        .chain(layout_ops.into_iter())
                        .chain([struct_.operation, ok_struct.operation, yield_.operation].into_iter())
                        .collect()
                }),
            );

            let yield_ = YieldOp::new(vec![maybe_composite.output()]);
            vec![maybe_composite.operation, yield_.operation]
        });
        ImplDeserializeOp::new(self.name.clone(), self.generics.clone(), body)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        derive_struct::{
            direct_field::DirectField, direct_field_attribute::DirectFieldAttribute, packed_field::PackedField,
            packed_field_attribute::PackedFieldAttribute,
        },
        ir::pattern_match::assert_matches,
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
    fn lower_se_generic() {
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

        let op = format!("{:#?}", input.lower_se().operation);
        let pattern = "
        impl_serialize |%serializer| [
            %maybe_composite = serialize_composite %serializer |%s_inner| [
                %nothing = serialize_nothing %s_inner
                yield %nothing
            ]
            %composite = try %maybe_composite
            %span = member [0, *] %composite
            %ok_span = ok %span
            yield %ok_span
        ]        
        ";
        assert_matches!(op, pattern);
    }

    #[test]
    fn lower_se_len_and_round() {
        let input = Struct {
            name: parse_quote!(Test),
            generics: Generics::default(),
            attributes: StructAttribute { len: Some(12), round: Some(8) },
            fields: vec![],
        };

        let op = format!("{:#?}", input.lower_se().operation);
        let pattern = "
        impl_serialize |%serializer| [
            %maybe_composite = serialize_composite %serializer |%s_inner| [
                %nothing = serialize_nothing %s_inner
                %maybe_len = pad [12] %s_inner
                %len = try %maybe_len
                %maybe_round = align [8] %s_inner
                %round = try %maybe_round
                yield %nothing
            ]
            %composite = try %maybe_composite
            %span = member [0, *] %composite
            %ok_span = ok %span
            yield %ok_span
        ]
        ";
        assert_matches!(op, pattern);
    }

    #[test]
    fn lower_se_direct_fields() {
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

        let op = format!("{:#?}", input.lower_se().operation);
        let pattern = "
        impl_serialize |%serializer| [
            %maybe_composite = serialize_composite %serializer |%s_inner| [
                %maybe_span_foo = execute || [
                    %self_foo = self
                    %foo = member [foo, &] %self_foo
                    %maybe_span_foo_i = serialize_object %s_inner, %foo
                    yield %maybe_span_foo_i
                ]

                %maybe_span_bar = execute || [
                    %self_bar = self
                    %bar = member [bar, &] %self_bar
                    %maybe_span_bar_i = serialize_object %s_inner, %bar
                    yield %maybe_span_bar_i
                ]

                %span_foo = try %maybe_span_foo
                %span_bar = try %maybe_span_bar
                %spans = tuple %span_foo, %span_bar
                %ok_spans = ok %spans
                yield %ok_spans
            ]
            %composite = try %maybe_composite
            %span = member [0, *] %composite
            %ok_span = ok %span
            yield %ok_span
        ]        
        ";
        println!("{}", input.lower_se().operation.to_token_stream());
        assert_matches!(op, pattern);
    }

    #[test]
    fn lower_de_generic() {
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

        let op = format!("{:#?}", input.lower_de().operation);
        let pattern = "
        impl_deserialize |%deserializer| [
            %maybe_composite = deserialize_composite %deserializer |%de_inner| [
                %struct = struct [Test]
                %ok_struct = ok %struct
                yield %ok_struct
            ]
            yield %maybe_composite
        ]        
        ";
        assert_matches!(op, pattern);
    }

    #[test]
    fn lower_se_print_tokens() {
        #[rustfmt::skip]
        let input: DeriveInput = parse_quote!(
            #[sorbit_bit_field(_b, repr(u16))]
            struct Packing {
                #[sorbit_bit_field(_b, bits(4..10))]
                a: u8,
                #[sorbit_bit_field(_b, bits(14..=15))]
                b: bool,
            }
        );

        let parsed = Struct::parse(&input).unwrap();

        println!("{}", parsed.lower_de().operation.to_token_stream());
    }
}
