use syn::{Generics, Ident};

use crate::ir::dag::{Operation, Region};
use crate::ir::ops::{
    DeserializeCompositeOp, ImplDeserializeOp, ImplSerializeOp, MemberOp, OkOp, SerializeCompositeOp,
    SerializeNothingOp, StructOp, TryOp, TupleOp, YieldOp,
};

use super::super::parse;
use super::field::Field;
use super::field_group::group_fields;
use super::lowering::{ToDeserializeOp, ToSerializeOp, lower_alignment, lower_offset};

pub struct Struct {
    pub ident: Ident,
    pub generics: Generics,
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
        Ok(Self { ident: value.ident, generics: value.generics, len: value.len, round: value.round, fields })
    }
}

impl ToSerializeOp for Struct {
    type Args = ();

    fn to_serialize_op(&self, _: Self::Args) -> Operation {
        let body = Region::new(1, |arguments| {
            let serializer = &arguments[0];
            let maybe_composite = SerializeCompositeOp::new(
                serializer.clone(),
                Region::new(1, |arguments| {
                    let serializer = &arguments[0];

                    let mut layout_ops = Vec::new();
                    lower_offset(serializer.clone(), self.len, true, &mut layout_ops);
                    lower_alignment(serializer.clone(), self.round, true, &mut layout_ops);

                    if self.fields.is_empty() {
                        let serialize_nothing = SerializeNothingOp::new(serializer.clone());
                        let yield_ = YieldOp::new(vec![serialize_nothing.output()]);
                        std::iter::once(serialize_nothing.operation)
                            .chain(layout_ops.into_iter())
                            .chain(std::iter::once(yield_.operation))
                            .collect()
                    } else {
                        let maybe_spans: Vec<_> =
                            self.fields.iter().map(|field| field.to_serialize_op(serializer.clone())).collect();
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
        ImplSerializeOp::new(self.ident.clone(), self.generics.clone(), body).operation
    }
}

impl ToDeserializeOp for Struct {
    type Args = ();

    fn to_deserialize_op(&self, _: Self::Args) -> Operation {
        let body = Region::new(1, |arguments| {
            let deserializer = &arguments[0];
            let maybe_composite = DeserializeCompositeOp::new(
                deserializer.clone(),
                Region::new(1, |arguments| {
                    let deserializer = &arguments[0];

                    let mut layout_ops = Vec::new();
                    lower_offset(deserializer.clone(), self.len, false, &mut layout_ops);
                    lower_alignment(deserializer.clone(), self.round, false, &mut layout_ops);

                    let maybe_des_ops: Vec<_> =
                        self.fields.iter().map(|field| field.to_deserialize_op(deserializer.clone())).collect();
                    let mut des_ops = Vec::new();
                    let mut field_names = Vec::new();
                    let mut field_values = Vec::new();
                    for (field, maybe_des_op) in self.fields.iter().zip(maybe_des_ops.iter()) {
                        match field {
                            Field::Direct { member, ty: _, offset: _, align: _, round: _ } => {
                                let des_op = TryOp::new(maybe_des_op.output(0));
                                field_names.push(member.clone());
                                field_values.push(des_op.output());
                                des_ops.extend([des_op.operation].into_iter());
                            }
                            Field::Bit { ident: _, ty: _, offset: _, align: _, round: _, members } => {
                                for (idx, member) in members.iter().enumerate() {
                                    let des_op = TryOp::new(maybe_des_op.output(idx));
                                    field_names.push(member.member.clone());
                                    field_values.push(des_op.output());
                                    des_ops.extend([des_op.operation].into_iter());
                                }
                            }
                        }
                    }

                    let struct_ = StructOp::new(
                        syn::TypePath { qself: None, path: syn::Path::from(self.ident.clone()) }.into(),
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
        ImplDeserializeOp::new(self.ident.clone(), self.generics.clone(), body).operation
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

        let input =
            Struct { ident: parse_quote!(Test), generics: input.generics, len: None, round: None, fields: vec![] };

        let op = format!("{:#?}", input.to_serialize_op(()));
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
    fn to_serialize_op_with_layout() {
        let input = Struct {
            ident: parse_quote!(Test),
            generics: Generics::default(),
            len: Some(12),
            round: Some(8),
            fields: vec![],
        };

        let op = format!("{:#?}", input.to_serialize_op(()));
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
    fn to_serialize_op_with_fields() {
        let input = Struct {
            ident: parse_quote!(Test),
            generics: Generics::default(),
            len: None,
            round: None,
            fields: vec![
                Field::Direct {
                    member: parse_quote!(foo),
                    ty: parse_quote!(u8),
                    offset: None,
                    align: None,
                    round: None,
                },
                Field::Direct {
                    member: parse_quote!(bar),
                    ty: parse_quote!(i8),
                    offset: None,
                    align: None,
                    round: None,
                },
            ],
        };

        let op = format!("{:#?}", input.to_serialize_op(()));
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

        let input =
            Struct { ident: parse_quote!(Test), generics: input.generics, len: None, round: None, fields: vec![] };

        let op = format!("{:#?}", input.to_deserialize_op(()));
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
}
