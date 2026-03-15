use std::ops::Range;

use proc_macro2::Span;
use syn::parse_quote;
use syn::spanned::Spanned;
use syn::{Ident, Member, Type};

use crate::attribute::BitNumbering;
use crate::attribute::Transform;
use crate::ir::{Region, ToDeserializeOp, ToSerializeOp, Value};
use crate::ops::algorithm::with_field_layout;
use crate::ops::constants::BIT_FIELD_TYPE;
use crate::ops::{
    deserialize_items_by_byte_count, deserialize_items_by_len, deserialize_object, empty_bit_field, items, len,
    pack_bit_field, ref_, serialize_object, symref, try_, unpack_bit_field,
};
use crate::r#struct::parse::FieldLayoutProperties;
use crate::utility::member_to_ident;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitFieldMember {
    pub member: Member,
    pub ty: Type,
    pub transform: Transform,
    pub bits: Range<u8>,
}

impl BitFieldMember {
    pub fn span(&self) -> Span {
        match &self.member {
            Member::Named(ident) => ident.span(),
            Member::Unnamed(_) => self.ty.span(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Field {
    Direct {
        member: Member,
        ty: Type,
        transform: Transform,
        layout_properties: FieldLayoutProperties,
    },
    Bit {
        #[allow(unused)]
        ident: Ident,
        ty: Type,
        bit_numbering: BitNumbering,
        layout_properties: FieldLayoutProperties,
        members: Vec<BitFieldMember>,
    },
}

impl Field {
    pub fn members(&self) -> Vec<&Member> {
        match self {
            Field::Direct { member, .. } => vec![member],
            Field::Bit { members, .. } => members.iter().map(|member| &member.member).collect(),
        }
    }
}

impl ToSerializeOp for Field {
    type Args = (Value, bool);

    fn to_serialize_op(&self, region: &mut Region, (serializer, use_padding): (Value, bool)) -> Vec<Value> {
        match self {
            Field::Direct { member, ty, transform, layout_properties, .. } => {
                let layout = &conditionally_padded_layout(layout_properties, use_padding);
                let result = with_layout(region, serializer, true, layout, |region, serializer| {
                    let field = symref(region, member_to_ident(member.clone()));
                    let transformed = serialize_transform(region, serializer, field, ty, transform);
                    serialize_object(region, serializer, transformed)
                });
                vec![result]
            }
            Field::Bit { ty, bit_numbering, layout_properties, members, .. } => {
                let layout = &conditionally_padded_layout(layout_properties, use_padding);
                let result = with_layout(region, serializer, true, layout, |region, serializer| {
                    let mut bit_field = empty_bit_field(region, ty.clone());

                    for BitFieldMember { member, ty, transform, bits, .. } in members {
                        let field = symref(region, member_to_ident(member.clone()));
                        let transformed = serialize_transform(region, serializer, field, ty, transform);
                        let result_new_bit_field =
                            pack_bit_field(region, transformed, bit_field, bits.clone(), *bit_numbering);
                        bit_field = try_(region, result_new_bit_field);
                    }

                    let bit_field_ref = ref_(region, bit_field);
                    serialize_object(region, serializer, bit_field_ref)
                });
                vec![result]
            }
        }
    }
}

impl ToDeserializeOp for Field {
    type Args = Value;

    fn to_deserialize_op(&self, region: &mut Region, deserializer: Value) -> Vec<Value> {
        match self {
            Field::Direct { ty, transform, layout_properties, .. } => {
                let result =
                    with_layout(region, deserializer, false, layout_properties, |region, de| match transform {
                        Transform::None => deserialize_object(region, de, ty.clone()),
                        Transform::Length(_) => deserialize_object(region, de, ty.clone()),
                        Transform::ByteCount(_) => deserialize_object(region, de, ty.clone()),
                        Transform::LengthBy(len_by) => {
                            let len = symref(region, member_to_ident(len_by.clone()));
                            deserialize_items_by_len(region, de, len, ty.clone())
                        }
                        Transform::ByteCountBy(byte_count_by) => {
                            let byte_count = symref(region, member_to_ident(byte_count_by.clone()));
                            deserialize_items_by_byte_count(region, de, byte_count, ty.clone())
                        }
                    });
                vec![result]
            }
            Field::Bit { ty, bit_numbering, layout_properties, members, .. } => {
                let result_raw_bits = with_layout(region, deserializer, false, layout_properties, |region, de| {
                    deserialize_object(region, de, parse_quote!(#BIT_FIELD_TYPE <#ty>))
                });
                let bit_field = try_(region, result_raw_bits);

                let unpacked = members
                    .iter()
                    .map(|BitFieldMember { ty, bits, .. }| {
                        unpack_bit_field(region, bit_field, ty.clone(), bits.clone(), *bit_numbering)
                    })
                    .collect();

                unpacked
            }
        }
    }
}

fn with_layout(
    region: &mut Region,
    serializer: Value,
    is_serializing: bool,
    layout_properties: &FieldLayoutProperties,
    body: impl FnOnce(&mut Region, Value) -> Value,
) -> Value {
    let FieldLayoutProperties { byte_order, offset, align, round } = layout_properties;
    with_field_layout(region, serializer, is_serializing, *byte_order, *offset, *align, *round, body)
}

fn conditionally_padded_layout(layout: &FieldLayoutProperties, use_padding: bool) -> FieldLayoutProperties {
    match use_padding {
        false => FieldLayoutProperties { byte_order: layout.byte_order, ..Default::default() },
        true => layout.clone(),
    }
}

pub fn serialize_transform(
    region: &mut Region,
    serializer: Value,
    value: Value,
    ty: &Type,
    transform: &Transform,
) -> Value {
    match transform {
        Transform::None => value,
        Transform::Length(member) => {
            // Get the length of the collection referred to by `member`.
            let pair = symref(region, member_to_ident(member.clone()));
            let result_len = len(region, serializer, pair, ty.clone());
            let len = try_(region, result_len);
            ref_(region, len)
        }
        Transform::ByteCount(_member) => value, // Needs to be updated in a second pass.
        Transform::LengthBy(_member) => {
            // Items without the length.
            let items = items(region, value);
            ref_(region, items)
        }
        Transform::ByteCountBy(_member) => {
            // Items without the length.
            let items = items(region, value);
            ref_(region, items)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::attribute::ByteOrder;
    use crate::ir::pattern_match::assert_matches;
    use crate::ops::yield_;

    use syn::parse_quote;

    #[test]
    fn to_serialize_op_direct_default() {
        let input = Field::Direct {
            member: parse_quote!(foo),
            ty: parse_quote!(i32),
            transform: Transform::None,
            layout_properties: Default::default(),
        };

        let serializer = Value::new();
        let mut region = Region::new(0);
        let results = input.to_serialize_op(&mut region, (serializer, true));
        yield_(&mut region, results);
        let op = format!("{:#}", region);

        let pattern = "
        {
            %foo = symref [foo]
            %res = serialize_object %serializer, %foo
            yield %res
        }
        ";
        assert_matches!(op, pattern);
    }

    #[test]
    fn to_serialize_op_direct_byte_order() {
        let input = Field::Direct {
            member: parse_quote!(foo),
            ty: parse_quote!(i32),
            transform: Transform::None,
            layout_properties: FieldLayoutProperties { byte_order: Some(ByteOrder::BigEndian), ..Default::default() },
        };

        let serializer = Value::new();
        let mut region = Region::new(0);
        let results = input.to_serialize_op(&mut region, (serializer, true));
        yield_(&mut region, results);
        let op = format!("{:#}", region);

        let pattern = "
        {
            %res = byte_order[BigEndian, true] %serializer |%se_inner| {
                %foo = symref [foo]
                %res_inner = serialize_object %se_inner, %foo
                yield %res_inner
            }
            yield %res
        }
        ";
        assert_matches!(op, pattern);
    }

    #[test]
    fn to_serialize_op_direct_layout() {
        let input = Field::Direct {
            member: parse_quote!(foo),
            ty: parse_quote!(i32),
            transform: Transform::None,
            layout_properties: FieldLayoutProperties {
                byte_order: None,
                offset: Some(1),
                align: Some(2),
                round: Some(3),
            },
        };

        let serializer = Value::new();
        let mut region = Region::new(0);
        let results = input.to_serialize_op(&mut region, (serializer, true));
        yield_(&mut region, results);
        let op = format!("{:#}", region);

        let pattern = "
        {
            %offset = pad [1, true] %serializer
            %try_offset = try %offset

            %align = align [2, true] %serializer
            %try_align = try %align
            
            %res = serialize_composite %serializer |%s_inner| {
                %foo = symref [foo]
                %res_inner = serialize_object %s_inner, %foo
                %round = align [3, true] %s_inner
                %try_round = try %round
                yield %res_inner
            }
            %res_try = try %res
            %res_1 = member [1, false] %res_try
            %res_ok = ok %res_1
            yield %res_ok
        }
        ";
        assert_matches!(op, pattern);
    }

    #[test]
    fn to_serialize_op_direct_layout_byte_order() {
        let input = Field::Direct {
            member: parse_quote!(foo),
            ty: parse_quote!(i32),
            transform: Transform::None,
            layout_properties: FieldLayoutProperties {
                byte_order: Some(ByteOrder::BigEndian),
                offset: Some(1),
                align: Some(2),
                round: Some(3),
            },
        };

        let serializer = Value::new();
        let mut region = Region::new(0);
        let results = input.to_serialize_op(&mut region, (serializer, true));
        yield_(&mut region, results);
        let op = format!("{:#}", region);

        let pattern = "
        {
            %offset = pad [1, true] %serializer
            %try_offset = try %offset

            %align = align [2, true] %serializer
            %try_align = try %align
            
            %res = serialize_composite %serializer |%s_inner| {
                %res_inner = byte_order[BigEndian, true] %s_inner |%se_bo| {
                    %foo = symref [foo]
                    %res_bo = serialize_object %se_bo, %foo
                    yield %res_bo
                }
                %round = align [3, true] %s_inner
                %try_round = try %round
                yield %res_inner
            }
            %res_try = try %res
            %res_1 = member [1, false] %res_try
            %res_ok = ok %res_1
            yield %res_ok
        }
        ";
        assert_matches!(op, pattern);
    }

    #[test]
    fn to_deserialize_op_direct_default() {
        let input = Field::Direct {
            member: parse_quote!(foo),
            ty: parse_quote!(i32),
            transform: Transform::None,
            layout_properties: Default::default(),
        };

        let serializer = Value::new();
        let mut region = Region::new(0);
        let results = input.to_deserialize_op(&mut region, serializer);
        yield_(&mut region, results);
        let op = format!("{:#}", region);

        let pattern = "
        {
            %res = deserialize_object [i32] %serializer
            yield %res
        }
        ";
        assert_matches!(op, pattern);
    }
    #[test]
    fn to_deserialize_op_direct_byte_order() {
        let input = Field::Direct {
            member: parse_quote!(foo),
            ty: parse_quote!(i32),
            transform: Transform::None,
            layout_properties: FieldLayoutProperties { byte_order: Some(ByteOrder::BigEndian), ..Default::default() },
        };

        let de = Value::new();
        let mut region = Region::new(0);
        let results = input.to_deserialize_op(&mut region, de);
        yield_(&mut region, results);
        let op = format!("{:#}", region);

        let pattern = "
        {
            %res = byte_order[BigEndian, false] %de |%de_bo| {
                %res_bo = deserialize_object [i32] %de_bo
                yield %res_bo
            }
            yield %res
        }
        ";
        assert_matches!(op, pattern);
    }

    #[test]
    fn to_deserialize_op_direct_layout() {
        let input = Field::Direct {
            member: parse_quote!(foo),
            ty: parse_quote!(i32),
            transform: Transform::None,
            layout_properties: FieldLayoutProperties {
                byte_order: None,
                offset: Some(1),
                align: Some(2),
                round: Some(3),
            },
        };

        let de = Value::new();
        let mut region = Region::new(0);
        let results = input.to_deserialize_op(&mut region, de);
        yield_(&mut region, results);
        let op = format!("{:#}", region);

        let pattern = "
        {
            %offset = pad [1, false] %deserializer
            %try_offset = try %offset

            %align = align [2, false] %deserializer
            %try_align = try %align

            %res = deserialize_composite %deserializer |%des_inner| {
                %res_inner = deserialize_object [i32] %des_inner
                %round = align [3, false] %des_inner
                %try_round = try %round
                yield %res_inner
            }
            yield %res
        }
        ";
        assert_matches!(op, pattern);
    }

    #[test]
    fn to_deserialize_op_direct_layout_byte_order() {
        let input = Field::Direct {
            member: parse_quote!(foo),
            ty: parse_quote!(i32),
            transform: Transform::None,
            layout_properties: FieldLayoutProperties {
                byte_order: Some(ByteOrder::BigEndian),
                offset: Some(1),
                align: Some(2),
                round: Some(3),
            },
        };

        let de = Value::new();
        let mut region = Region::new(0);
        let results = input.to_deserialize_op(&mut region, de);
        yield_(&mut region, results);
        let op = format!("{:#}", region);

        let pattern = "
        {
            %offset = pad [1, false] %deserializer
            %try_offset = try %offset

            %align = align [2, false] %deserializer
            %try_align = try %align

            %res = deserialize_composite %deserializer |%des_inner| {
                %res_inner = byte_order[BigEndian, false] %des_inner |%de_bo| {
                    %res_bo = deserialize_object [i32] %de_bo
                    yield %res_bo
                }
                %round = align [3, false] %des_inner
                %try_round = try %round
                yield %res_inner
            }
            yield %res
        }
        ";
        assert_matches!(op, pattern);
    }

    fn make_bit_field_empty() -> Field {
        Field::Bit {
            ident: parse_quote!(_bit_field),
            ty: parse_quote!(u16),
            bit_numbering: BitNumbering::LSB0,
            layout_properties: Default::default(),
            members: vec![],
        }
    }

    fn make_bit_field_with_members() -> Field {
        Field::Bit {
            ident: parse_quote!(_bit_field),
            ty: parse_quote!(u16),
            bit_numbering: BitNumbering::LSB0,
            layout_properties: Default::default(),
            members: vec![
                BitFieldMember {
                    member: parse_quote!(foo),
                    ty: parse_quote!(u8),
                    transform: Transform::None,
                    bits: 4..7,
                },
                BitFieldMember {
                    member: parse_quote!(bar),
                    ty: parse_quote!(i8),
                    transform: Transform::None,
                    bits: 0..4,
                },
            ],
        }
    }

    #[test]
    fn to_serialize_op_bit_default() {
        let input = make_bit_field_empty();

        let serializer = Value::new();
        let mut region = Region::new(0);
        let results = input.to_serialize_op(&mut region, (serializer, true));
        yield_(&mut region, results);
        let op = format!("{:#}", region);

        let pattern = "
        {
            %bf = empty_bit_field [u16]
            %ref_bf = ref %bf
            %s = serialize_object %serializer %ref_bf
            yield %s
        }
        ";
        assert_matches!(op, pattern);
    }

    #[test]
    fn to_serialize_op_bit_with_members() {
        let input = make_bit_field_with_members();

        let serializer = Value::new();
        let mut region = Region::new(0);
        let results = input.to_serialize_op(&mut region, (serializer, true));
        yield_(&mut region, results);
        let op = format!("{:#}", region);

        let pattern = "
        {
            %bf0 = empty_bit_field [u16]
            
            %foo = symref [foo]
            %maybe_bf1 = pack_bit_field [4..7, LSB0] %foo %bf0
            %bf1 = try %maybe_bf1

            %bar = symref [bar]
            %maybe_bf2 = pack_bit_field [0..4, LSB0] %bar %bf1
            %bf2 = try %maybe_bf2

            %ref_bf2 = ref %bf2
            %s = serialize_object %serializer %ref_bf2
            yield %s
        }
        ";
        assert_matches!(op, pattern);
    }

    #[test]
    fn to_deserialize_op_bit_empty() {
        let input = make_bit_field_empty();

        let de = Value::new();
        let mut region = Region::new(0);
        let results = input.to_deserialize_op(&mut region, de);
        yield_(&mut region, results);
        let op = format!("{:#}", region);

        let pattern = "
        {
            %s = deserialize_object [::sorbit::bit::BitField < u16 > ] %deserializer
            %bf = try %s
            yield
        }
        ";
        assert_matches!(op, pattern);
    }

    #[test]
    fn to_deserialize_op_bit_with_members() {
        let input = make_bit_field_with_members();

        let de = Value::new();
        let mut region = Region::new(0);
        let results = input.to_deserialize_op(&mut region, de);
        yield_(&mut region, results);
        let op = format!("{:#}", region);

        let pattern = "
        {
            %s = deserialize_object [::sorbit::bit::BitField < u16 >] %deserializer
            %bf = try %s

            %maybe_foo = unpack_bit_field [u8, 4..7, LSB0] %bf
            %maybe_bar = unpack_bit_field [i8, 0..4, LSB0] %bf

            yield %maybe_foo, %maybe_bar
        }
        ";
        assert_matches!(op, pattern);
    }
}
