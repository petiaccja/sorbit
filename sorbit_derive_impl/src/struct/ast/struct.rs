use syn::{Generics, Ident};

use crate::attribute::ByteOrder;
use crate::ir::dag::{Region, Value};
use crate::ir::ops::{
    self, deserialize_composite, impl_deserialize, impl_serialize, member, ok, serialize_composite, serialize_nothing,
    struct_, try_, tuple, yield_,
};

use super::super::parse;
use super::field::Field;
use super::field_group::group_fields;
use super::lowering::{ToDeserializeOp, ToSerializeOp, lower_alignment, lower_offset};

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
        let field_groups = group_fields(value.fields.into_iter())?;
        let fields = field_groups
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
        impl_serialize(region, self.ident.clone(), self.generics.clone(), |region, serializer| {
            let result = if let Some(byte_order) = self.byte_order {
                let result_bo = ops::byte_order(region, serializer, byte_order, true, |region, serializer| {
                    let result = self.serialize_members(region, serializer);
                    yield_(region, vec![result]);
                });
                let span_bo = try_(region, result_bo);
                let span = member(region, span_bo, syn::Member::Unnamed(syn::Index::from(0)), false);
                ok(region, span)
            } else {
                self.serialize_members(region, serializer)
            };
            yield_(region, vec![result]);
        });
        vec![]
    }
}

impl ToDeserializeOp for Struct {
    type Args = ();

    fn to_deserialize_op(&self, region: &mut Region, _: Self::Args) -> Vec<Value> {
        impl_deserialize(region, self.ident.clone(), self.generics.clone(), |region, deserializer| {
            let result = if let Some(byte_order) = self.byte_order {
                ops::byte_order(region, deserializer, byte_order, false, |region, deserializer| {
                    let result = self.deserialize_members(region, deserializer);
                    yield_(region, vec![result]);
                })
            } else {
                self.deserialize_members(region, deserializer)
            };
            yield_(region, vec![result]);
        });
        vec![]
    }
}

impl Struct {
    fn serialize_members(&self, region: &mut Region, serializer: Value) -> Value {
        let maybe_composite = serialize_composite(region, serializer, |region, serializer| {
            if self.fields.is_empty() {
                let serialize_nothing = serialize_nothing(region, serializer.clone());
                lower_offset(region, serializer, self.len, true);
                lower_alignment(region, serializer, self.round, true);
                let _ = yield_(region, vec![serialize_nothing]);
            } else {
                let maybe_spans: Vec<_> =
                    self.fields.iter().map(|field| field.to_serialize_op(region, serializer)).flatten().collect();
                let spans: Vec<_> = maybe_spans.into_iter().map(|maybe_span| try_(region, maybe_span)).collect();
                lower_offset(region, serializer, self.len, true);
                lower_alignment(region, serializer, self.round, true);
                let span_tuple = tuple(region, spans);
                let result = ok(region, span_tuple);
                let _ = yield_(region, vec![result]);
            }
        });
        let composite = try_(region, maybe_composite);
        let span = member(region, composite, syn::Member::Unnamed(syn::Index::from(0)), false);
        ok(region, span)
    }

    fn deserialize_members(&self, region: &mut Region, deserializer: Value) -> Value {
        deserialize_composite(region, deserializer, |region, deserializer| {
            let maybe_deserialized: Vec<_> =
                self.fields.iter().map(|field| field.to_deserialize_op(region, deserializer)).collect();
            let mut field_names = Vec::new();
            let mut field_values = Vec::new();
            for (field, maybe_deserialized) in self.fields.iter().zip(maybe_deserialized.iter()) {
                match field {
                    Field::Direct { member, ty: _, byte_order: _, offset: _, align: _, round: _ } => {
                        let deserialized = try_(region, maybe_deserialized[0]);
                        field_names.push(member.clone());
                        field_values.push(deserialized);
                    }
                    Field::Bit {
                        ident: _,
                        ty: _,
                        byte_order: _,
                        bit_numbering: _,
                        offset: _,
                        align: _,
                        round: _,
                        members,
                    } => {
                        for (idx, member) in members.iter().enumerate() {
                            let deserialized = try_(region, maybe_deserialized[idx]);
                            field_names.push(member.member.clone());
                            field_values.push(deserialized);
                        }
                    }
                }
            }

            lower_offset(region, deserializer, self.len, false);
            lower_alignment(region, deserializer, self.round, false);

            let struct_ = struct_(
                region,
                syn::TypePath { qself: None, path: syn::Path::from(self.ident.clone()) }.into(),
                field_names.into_iter().zip(field_values.into_iter()).collect(),
            );
            let result = ok(region, struct_);
            let _ = yield_(region, vec![result]);
        })
    }
}

#[cfg(test)]
mod tests {
    use syn::{DeriveInput, parse_quote};

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
            impl_serialize [ Test ] |%serializer| {
                %maybe_composite = serialize_composite %serializer |%s_inner| {
                    %nothing = serialize_nothing %s_inner
                    yield %nothing
                }
                %composite = try %maybe_composite
                %span = member [0, val] %composite
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
                %maybe_composite = serialize_composite %serializer |%s_inner| {
                    %nothing = serialize_nothing %s_inner
                    %maybe_len = pad [12, true] %s_inner
                    %len = try %maybe_len
                    %maybe_round = align [8, true] %s_inner
                    %round = try %maybe_round
                    yield %nothing
                }
                %composite = try %maybe_composite
                %span = member [0, val] %composite
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
                    byte_order: None,
                    offset: None,
                    align: None,
                    round: None,
                },
                Field::Direct {
                    member: parse_quote!(bar),
                    ty: parse_quote!(i8),
                    byte_order: None,
                    offset: None,
                    align: None,
                    round: None,
                },
            ],
        };

        let mut region = Region::new(0);
        input.to_serialize_op(&mut region, ());
        let op = format!("{:#}", region);

        let pattern = "
        {
            impl_serialize [ Test ] |%serializer| {
                %maybe_composite = serialize_composite %serializer |%s_inner| {
                    %self_foo = self
                    %foo = member [foo, ref] %self_foo
                    %maybe_span_foo = serialize_object %s_inner, %foo

                    %self_bar = self
                    %bar = member [bar, ref] %self_bar
                    %maybe_span_bar = serialize_object %s_inner, %bar

                    %span_foo = try %maybe_span_foo
                    %span_bar = try %maybe_span_bar
                    %spans = tuple %span_foo, %span_bar
                    %ok_spans = ok %spans
                    yield %ok_spans
                }
                %composite = try %maybe_composite
                %span = member [0, val] %composite
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
            impl_deserialize [ Test ] |%deserializer| {
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
