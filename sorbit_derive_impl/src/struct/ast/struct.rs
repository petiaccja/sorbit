use syn::{Generics, Ident, Member};

use crate::attribute::ByteOrder;
use crate::ir::{Region, Value};
use crate::ops::algorithm::{with_maybe_alignment, with_maybe_byte_order, with_maybe_offset};
use crate::ops::{
    deserialize_composite, destructure, impl_deserialize, impl_serialize, member, ok, self_, serialize_composite,
    struct_, success, sym, try_, tuple,
};
use crate::r#struct::ast::conversion::add_symmetric_transforms;
use crate::r#struct::ast::field::BitFieldMember;
use crate::utility::{ident_to_type, member_to_ident};

use super::super::parse;
use super::conversion::to_layout_fields;
use super::field::Field;
use crate::ir::{ToDeserializeOp, ToSerializeOp};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Struct {
    pub ident: Ident,
    pub generics: Generics,
    pub byte_order: Option<ByteOrder>,
    pub len: Option<u64>,
    pub round: Option<u64>,
    pub fields: Vec<Field>,
}

impl TryFrom<parse::Struct> for Struct {
    type Error = syn::Error;
    fn try_from(value: parse::Struct) -> Result<Self, Self::Error> {
        let symmetric_fields = add_symmetric_transforms(value.fields)?;
        let layout_fields = to_layout_fields(symmetric_fields.into_iter())?;
        let fields = layout_fields
            .into_iter()
            .map(|field_group| field_group.into_field())
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            ident: value.ident,
            generics: value.generics,
            byte_order: value.byte_order,
            len: value.len,
            round: value.round,
            fields,
        })
    }
}

impl ToSerializeOp for Struct {
    type Args = ();

    fn to_serialize_op(&self, region: &mut Region, _: Self::Args) -> Vec<Value> {
        impl_serialize(
            region,
            self.ident.clone(),
            self.generics.clone(),
            Region::build(|region, [serializer]| {
                self.destructure(region);
                vec![self.serialize_members(region, serializer)]
            }),
        );
        vec![]
    }
}

impl ToDeserializeOp for Struct {
    type Args = ();

    fn to_deserialize_op(&self, region: &mut Region, _: Self::Args) -> Vec<Value> {
        impl_deserialize(
            region,
            self.ident.clone(),
            self.generics.clone(),
            Region::build(|region, [deserializer]| vec![self.deserialize_members(region, deserializer)]),
        );
        vec![]
    }
}

impl Struct {
    pub fn serialize_members(&self, region: &mut Region, serializer: Value) -> Value {
        with_maybe_byte_order(region, serializer, self.byte_order, true, |region, serializer| {
            let maybe_composite = serialize_composite(
                region,
                serializer,
                Region::build(|region, [serializer]| {
                    if self.fields.is_empty() {
                        let success_ = success(region, serializer.clone());
                        with_maybe_offset(region, serializer, self.len, true);
                        with_maybe_alignment(region, serializer, self.round, true);
                        vec![success_]
                    } else {
                        let maybe_spans: Vec<_> = self
                            .fields
                            .iter()
                            .map(|field| field.to_serialize_op(region, serializer))
                            .flatten()
                            .collect();
                        let spans: Vec<_> =
                            maybe_spans.into_iter().map(|maybe_span| try_(region, maybe_span)).collect();
                        with_maybe_offset(region, serializer, self.len, true);
                        with_maybe_alignment(region, serializer, self.round, true);
                        let span_tuple = tuple(region, spans);
                        let result = ok(region, span_tuple);
                        vec![result]
                    }
                }),
            );
            let composite = try_(region, maybe_composite);
            let span = member(region, composite, syn::Member::Unnamed(syn::Index::from(0)), false);
            ok(region, span)
        })
    }

    pub fn deserialize_members(&self, region: &mut Region, deserializer: Value) -> Value {
        with_maybe_byte_order(region, deserializer, self.byte_order, false, |region, deserializer| {
            deserialize_composite(
                region,
                deserializer,
                Region::build(|region, [deserializer]| {
                    let fields: Vec<_> = self
                        .fields
                        .iter()
                        .map(|field| {
                            let results = field.to_deserialize_op(region, deserializer);
                            let members = field.members();
                            let values: Vec<_> = results.iter().map(|result| try_(region, *result)).collect();
                            std::iter::zip(members, &values)
                                .for_each(|(member, value)| sym(region, *value, member_to_ident(member.clone())));
                            values
                        })
                        .flatten()
                        .collect();
                    let members = self.members();

                    with_maybe_offset(region, deserializer, self.len, false);
                    with_maybe_alignment(region, deserializer, self.round, false);

                    let struct_ = struct_(
                        region,
                        syn::TypePath { qself: None, path: syn::Path::from(self.ident.clone()) }.into(),
                        members.into_iter().cloned().zip(fields.into_iter()).collect(),
                    );
                    let result = ok(region, struct_);
                    vec![result]
                }),
            )
        })
    }

    pub fn members(&self) -> Vec<&Member> {
        let mut result = Vec::new();
        for field in &self.fields {
            match field {
                Field::Direct { member, .. } => result.push(member),
                Field::Bit { members, .. } => {
                    for BitFieldMember { member, .. } in members {
                        result.push(member);
                    }
                }
            }
        }
        result
    }

    fn destructure(&self, region: &mut Region) {
        let self_ = self_(region);
        let members = self.members();
        let bindings = members.into_iter().map(|member| (member.clone(), member_to_ident(member.clone()))).collect();
        destructure(region, self_, ident_to_type(self.ident.clone()), bindings);
    }
}

#[cfg(test)]
mod tests {
    use syn::{DeriveInput, parse_quote};

    use crate::attribute::Transform;
    use crate::ir::pattern_match::assert_matches;

    use super::*;

    #[test]
    fn to_serialize_op_generic() {
        #[rustfmt::skip]
        let input: DeriveInput = parse_quote!(
            struct Ignore<'x, T: Clone>
            where
                T: Default {}
        );

        let input = Struct {
            ident: parse_quote!(Test),
            generics: input.generics,
            byte_order: None,
            len: None,
            round: None,
            fields: vec![],
        };

        let mut region = Region::new(0);
        input.to_serialize_op(&mut region, ());
        let op = format!("{:#}", region);

        let pattern = "
        {
            impl_serialize [ Test, < 'x T : Clone > ] |%serializer| {
                %self = self
                destructure [ Test ] %self
                %maybe_composite = serialize_composite %serializer |%s_inner| {
                    %nothing = success %s_inner
                    yield %nothing
                }
                %composite = try %maybe_composite
                %span = member [0, false] %composite
                %ok_span = ok %span
                yield %ok_span
            }
        }
        ";
        assert_matches!(op, pattern);
    }

    #[test]
    fn to_serialize_op_with_layout() {
        let input = Struct {
            ident: parse_quote!(Test),
            generics: Generics::default(),
            byte_order: None,
            len: Some(12),
            round: Some(8),
            fields: vec![],
        };

        let mut region = Region::new(0);
        input.to_serialize_op(&mut region, ());
        let op = format!("{:#}", region);

        let pattern = "
        {
            impl_serialize [ Test ] |%serializer| {
                %self = self
                destructure [ Test ] %self
                %maybe_composite = serialize_composite %serializer |%s_inner| {
                    %nothing = success %s_inner
                    %maybe_len = pad [12, true] %s_inner
                    %len = try %maybe_len
                    %maybe_round = align [8, true] %s_inner
                    %round = try %maybe_round
                    yield %nothing
                }
                %composite = try %maybe_composite
                %span = member [0, false] %composite
                %ok_span = ok %span
                yield %ok_span
            }
        }
        ";
        assert_matches!(op, pattern);
    }

    #[test]
    fn to_serialize_op_with_fields() {
        let input = Struct {
            ident: parse_quote!(Test),
            generics: Generics::default(),
            byte_order: None,
            len: None,
            round: None,
            fields: vec![
                Field::Direct {
                    member: parse_quote!(foo),
                    ty: parse_quote!(u8),
                    transform: Transform::None,
                    layout_properties: Default::default(),
                },
                Field::Direct {
                    member: parse_quote!(bar),
                    ty: parse_quote!(i8),
                    transform: Transform::None,
                    layout_properties: Default::default(),
                },
            ],
        };

        let mut region = Region::new(0);
        input.to_serialize_op(&mut region, ());
        let op = format!("{:#}", region);

        let pattern = "
        {
            impl_serialize [ Test ] |%serializer| {
                %self = self
                destructure [Test, foo: foo, bar: bar] %self
                %maybe_composite = serialize_composite %serializer |%s_inner| {
                    %foo = symref [foo]
                    %maybe_span_foo = serialize_object %s_inner, %foo

                    %bar = symref [bar]
                    %maybe_span_bar = serialize_object %s_inner, %bar

                    %span_foo = try %maybe_span_foo
                    %span_bar = try %maybe_span_bar
                    %spans = tuple %span_foo, %span_bar
                    %ok_spans = ok %spans
                    yield %ok_spans
                }
                %composite = try %maybe_composite
                %span = member [0, false] %composite
                %ok_span = ok %span
                yield %ok_span
            }
        }     
        ";
        assert_matches!(op, pattern);
    }

    #[test]
    fn to_deserialize_op_generic() {
        #[rustfmt::skip]
        let input: DeriveInput = parse_quote!(
            struct Ignore<'x, T: Clone>
            where
                T: Default {}
        );

        let input = Struct {
            ident: parse_quote!(Test),
            generics: input.generics,
            byte_order: None,
            len: None,
            round: None,
            fields: vec![],
        };

        let mut region = Region::new(0);
        input.to_deserialize_op(&mut region, ());
        let op = format!("{:#}", region);

        let pattern = "
        {
            impl_deserialize [ Test, < 'x T : Clone > ] |%deserializer| {
                %maybe_composite = deserialize_composite %deserializer |%de_inner| {
                    %struct = struct [Test]
                    %ok_struct = ok %struct
                    yield %ok_struct
                }
                yield %maybe_composite
            }
        }
        ";
        assert_matches!(op, pattern);
    }
}
