use std::ops::Range;

use syn::{Ident, Member, Type};

use crate::attribute::BitNumbering;
use crate::attribute::Transform;
use crate::ir::algorithm::with_field_layout;
use crate::ir::dag::{Region, ToDeserializeOp, ToSerializeOp, Value};
use crate::ir::ops::deserialize_items_exact;
use crate::ir::ops::items;
use crate::ir::ops::len;
use crate::ir::ops::{
    deserialize_object, empty_bit_field, into_bit_field, into_raw_bits, pack_bit_field, ref_, serialize_object, symref,
    try_, unpack_bit_field,
};
use crate::r#struct::parse::FieldLayoutProperties;
use crate::utility::member_to_ident;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitFieldMember {
    pub member: Member,
    pub ty: Type,
    pub transform: Transform,
    pub bits: Range<u8>,
}

impl ToSerializeOp for Field {
    type Args = Value;

    fn to_serialize_op(&self, region: &mut Region, serializer: Value) -> Vec<Value> {
        match self {
            Field::Direct { member, ty, transform, layout_properties, .. } => {
                let result = with_layout(region, serializer, true, layout_properties, |region, serializer| {
                    let field = symref(region, member_to_ident(member.clone()));
                    let transformed = serialize_transform(region, serializer, field, ty, transform);
                    serialize_object(region, serializer, transformed)
                });
                vec![result]
            }
            Field::Bit { ty, bit_numbering, layout_properties, members, .. } => {
                let mut bit_field = empty_bit_field(region, ty.clone());

                for BitFieldMember { member, ty, transform, bits, .. } in members {
                    let field = symref(region, member_to_ident(member.clone()));
                    let transformed = serialize_transform(region, serializer, field, ty, transform);
                    let result_new_bit_field =
                        pack_bit_field(region, transformed, bit_field, bits.clone(), *bit_numbering);
                    bit_field = try_(region, result_new_bit_field);
                }

                let raw = into_raw_bits(region, bit_field);
                let raw_ref = ref_(region, raw);
                let result = with_layout(region, serializer, true, layout_properties, |region, serializer| {
                    serialize_object(region, serializer, raw_ref)
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
                let result = with_layout(region, deserializer, false, layout_properties, |region, de| {
                    match transform {
                        Transform::None => deserialize_object(region, de, ty.clone()),
                        Transform::Length(member) => deserialize_object(region, de, ty.clone()),
                        Transform::ByteCount(member) => deserialize_object(region, de, ty.clone()),
                        Transform::LengthBy(member) => todo!(), //deserialize_items_exact(region, de, len, ty.clone()),
                        Transform::ByteCountBy(member) => todo!(),
                    }
                    //deserialize_object(region, de, ty.clone())
                });
                vec![result]
            }
            Field::Bit { ty, bit_numbering, layout_properties, members, .. } => {
                let result_raw_bits = with_layout(region, deserializer, false, layout_properties, |region, de| {
                    deserialize_object(region, de, ty.clone())
                });
                let raw_bits = try_(region, result_raw_bits);
                let bit_field = into_bit_field(region, raw_bits);

                let unpacked = members
                    .iter()
                    .map(|BitFieldMember { ty, transform, bits, .. }| {
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
    use crate::ir::{dag::Id, ops::yield_, pattern_match::assert_matches};

    use syn::parse_quote;

    #[test]
    fn to_serialize_op_direct_default() {
        let input = Field::Direct {
            member: parse_quote!(foo),
            ty: parse_quote!(i32),
            transform: Transform::None,
            layout_properties: Default::default(),
        };

        let serializer = Id::new().value(0);
        let mut region = Region::new(0);
        let results = input.to_serialize_op(&mut region, serializer);
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

        let serializer = Id::new().value(0);
        let mut region = Region::new(0);
        let results = input.to_serialize_op(&mut region, serializer);
        yield_(&mut region, results);
        let op = format!("{:#}", region);

        let pattern = "
        {
            %res = byte_order[BigEndian] %serializer |%se_inner| {
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

        let serializer = Id::new().value(0);
        let mut region = Region::new(0);
        let results = input.to_serialize_op(&mut region, serializer);
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
            %res_1 = member [1, val] %res_try
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

        let serializer = Id::new().value(0);
        let mut region = Region::new(0);
        let results = input.to_serialize_op(&mut region, serializer);
        yield_(&mut region, results);
        let op = format!("{:#}", region);

        let pattern = "
        {
            %offset = pad [1, true] %serializer
            %try_offset = try %offset

            %align = align [2, true] %serializer
            %try_align = try %align
            
            %res = serialize_composite %serializer |%s_inner| {
                %res_inner = byte_order[BigEndian] %s_inner |%se_bo| {
                    %foo = symref [foo]
                    %res_bo = serialize_object %se_bo, %foo
                    yield %res_bo
                }
                %round = align [3, true] %s_inner
                %try_round = try %round
                yield %res_inner
            }
            %res_try = try %res
            %res_1 = member [1, val] %res_try
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

        let serializer = Id::new().value(0);
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

        let de = Id::new().value(0);
        let mut region = Region::new(0);
        let results = input.to_deserialize_op(&mut region, de);
        yield_(&mut region, results);
        let op = format!("{:#}", region);

        let pattern = "
        {
            %res = byte_order[BigEndian] %de |%de_bo| {
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

        let de = Id::new().value(0);
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

        let de = Id::new().value(0);
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
                %res_inner = byte_order[BigEndian] %des_inner |%de_bo| {
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

        let serializer = Id::new().value(0);
        let mut region = Region::new(0);
        let results = input.to_serialize_op(&mut region, serializer);
        yield_(&mut region, results);
        let op = format!("{:#}", region);

        let pattern = "
        {
            %bf = empty_bit_field [u16]
            %raw = into_raw_bits %bf
            %ref_raw = ref %raw
            %s = serialize_object %serializer %ref_raw
            yield %s
        }
        ";
        assert_matches!(op, pattern);
    }

    #[test]
    fn to_serialize_op_bit_with_members() {
        let input = make_bit_field_with_members();

        let serializer = Id::new().value(0);
        let mut region = Region::new(0);
        let results = input.to_serialize_op(&mut region, serializer);
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

            %raw = into_raw_bits %bf2
            %ref_raw = ref %raw
            %s = serialize_object %serializer %ref_raw
            yield %s
        }
        ";
        assert_matches!(op, pattern);
    }

    #[test]
    fn to_deserialize_op_bit_empty() {
        let input = make_bit_field_empty();

        let de = Id::new().value(0);
        let mut region = Region::new(0);
        let results = input.to_deserialize_op(&mut region, de);
        yield_(&mut region, results);
        let op = format!("{:#}", region);

        let pattern = "
        {
            %s = deserialize_object [u16] %deserializer
            %try_s = try %s
            %bf = into_bit_field %try_s
            yield
        }
        ";
        assert_matches!(op, pattern);
    }

    #[test]
    fn to_deserialize_op_bit_with_members() {
        let input = make_bit_field_with_members();

        let de = Id::new().value(0);
        let mut region = Region::new(0);
        let results = input.to_deserialize_op(&mut region, de);
        yield_(&mut region, results);
        let op = format!("{:#}", region);

        let pattern = "
        {
            %s = deserialize_object [u16] %deserializer
            %try_s = try %s
            %bf = into_bit_field %try_s

            %maybe_foo = unpack_bit_field [u8, 4..7, LSB0] %bf
            %maybe_bar = unpack_bit_field [i8, 0..4, LSB0] %bf

            yield %maybe_foo, %maybe_bar
        }
        ";
        assert_matches!(op, pattern);
    }
}
