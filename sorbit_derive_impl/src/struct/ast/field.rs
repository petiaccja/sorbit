use std::ops::Range;

use syn::{Ident, Member, Type};

use super::lowering::{
    ToDeserializeOp, ToSerializeOp, lower_alignment, lower_deserialization_rounding, lower_offset,
    lower_serialization_rounding,
};

use crate::attribute::{BitNumbering, ByteOrder};
use crate::ir::dag::{Region, Value};
use crate::ir::ops::{
    self as ops, empty_bit_field, into_bit_field, into_raw_bits, pack_bit_field, ref_, self_, try_, unpack_bit_field,
};

pub enum Field {
    Direct {
        member: Member,
        ty: Type,
        byte_order: Option<ByteOrder>,
        offset: Option<u64>,
        align: Option<u64>,
        round: Option<u64>,
    },
    Bit {
        #[allow(unused)]
        ident: Ident,
        ty: Type,
        byte_order: Option<ByteOrder>,
        bit_numbering: BitNumbering,
        offset: Option<u64>,
        align: Option<u64>,
        round: Option<u64>,
        members: Vec<BitFieldMember>,
    },
}

pub struct BitFieldMember {
    pub member: Member,
    pub ty: Type,
    pub bits: Range<u8>,
}

impl ToSerializeOp for Field {
    type Args = Value;

    fn to_serialize_op(&self, region: &mut Region, serializer: Value) -> Vec<Value> {
        match self {
            Field::Direct { member, ty: _, byte_order, offset, align, round } => {
                let self_ = self_(region);
                let object = ops::member(region, self_, member.clone(), true);
                lower_offset(region, serializer.clone(), *offset, true);
                lower_alignment(region, serializer.clone(), *align, true);
                let result = lower_serialization_rounding(region, serializer, object, *byte_order, *round);
                vec![result]
            }
            Field::Bit { ident: _, ty, byte_order, bit_numbering, offset, align, round, members } => {
                let self_ = self_(region);

                let mut bit_field = empty_bit_field(region, ty.clone());

                for member in members {
                    let object = ops::member(region, self_, member.member.clone(), true);
                    let maybe_new_bit_field =
                        pack_bit_field(region, object, bit_field, member.bits.clone(), *bit_numbering);
                    bit_field = try_(region, maybe_new_bit_field);
                }

                let raw = into_raw_bits(region, bit_field);
                let raw_ref = ref_(region, raw);
                lower_offset(region, serializer, *offset, true);
                lower_alignment(region, serializer, *align, true);
                let result = lower_serialization_rounding(region, serializer, raw_ref, *byte_order, *round);
                vec![result]
            }
        }
    }
}

impl ToDeserializeOp for Field {
    type Args = Value;

    fn to_deserialize_op(&self, region: &mut Region, deserializer: Value) -> Vec<Value> {
        match self {
            Field::Direct { member: _, ty, byte_order, offset, align, round } => {
                lower_offset(region, deserializer, *offset, false);
                lower_alignment(region, deserializer, *align, false);
                let result = lower_deserialization_rounding(region, deserializer, ty.clone(), *byte_order, *round);
                vec![result]
            }
            Field::Bit { ident: _, ty, byte_order, bit_numbering, offset, align, round, members } => {
                lower_offset(region, deserializer, *offset, false);
                lower_alignment(region, deserializer, *align, false);
                let maybe_raw_bits =
                    lower_deserialization_rounding(region, deserializer, ty.clone(), *byte_order, *round);
                let raw_bits = try_(region, maybe_raw_bits);
                let bit_field = into_bit_field(region, raw_bits);

                let unpacked = members
                    .iter()
                    .map(|member| {
                        unpack_bit_field(region, bit_field, member.ty.clone(), member.bits.clone(), *bit_numbering)
                    })
                    .collect();

                unpacked
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::ir::{dag::Id, ops::yield_, pattern_match::assert_matches};

    use syn::parse_quote;

    #[test]
    fn to_serialize_op_direct_default() {
        let input = Field::Direct {
            member: parse_quote!(foo),
            ty: parse_quote!(i32),
            byte_order: None,
            offset: None,
            align: None,
            round: None,
        };

        let serializer = Id::new().value(0);
        let mut region = Region::new(0);
        let results = input.to_serialize_op(&mut region, serializer);
        yield_(&mut region, results);
        let op = format!("{:#}", region);

        println!("{op}");

        let pattern = "
        {
            %self = self
            %foo = member [foo, ref] %self
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
            byte_order: Some(ByteOrder::BigEndian),
            offset: None,
            align: None,
            round: None,
        };

        let serializer = Id::new().value(0);
        let mut region = Region::new(0);
        let results = input.to_serialize_op(&mut region, serializer);
        yield_(&mut region, results);
        let op = format!("{:#}", region);

        let pattern = "
        {
            %self = self
            %foo = member [foo, ref] %self
            %res = byte_order[BigEndian] %serializer |%se_inner| {
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
            byte_order: None,
            offset: Some(1),
            align: Some(2),
            round: Some(3),
        };

        let serializer = Id::new().value(0);
        let mut region = Region::new(0);
        let results = input.to_serialize_op(&mut region, serializer);
        yield_(&mut region, results);
        let op = format!("{:#}", region);

        let pattern = "
        {
            %self = self
            %foo = member [foo, ref] %self

            %offset = pad [1, true] %serializer
            %try_offset = try %offset

            %align = align [2, true] %serializer
            %try_align = try %align
            
            %res = serialize_composite %serializer |%s_inner| {
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
            byte_order: Some(ByteOrder::BigEndian),
            offset: Some(1),
            align: Some(2),
            round: Some(3),
        };

        let serializer = Id::new().value(0);
        let mut region = Region::new(0);
        let results = input.to_serialize_op(&mut region, serializer);
        yield_(&mut region, results);
        let op = format!("{:#}", region);

        let pattern = "
        {
            %self = self
            %foo = member [foo, ref] %self

            %offset = pad [1, true] %serializer
            %try_offset = try %offset

            %align = align [2, true] %serializer
            %try_align = try %align
            
            %res = serialize_composite %serializer |%s_inner| {
                %res_inner = byte_order[BigEndian] %s_inner |%se_bo| {
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
            byte_order: None,
            offset: None,
            align: None,
            round: None,
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
            byte_order: Some(ByteOrder::BigEndian),
            offset: None,
            align: None,
            round: None,
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
            byte_order: None,
            offset: Some(1),
            align: Some(2),
            round: Some(3),
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
            byte_order: Some(ByteOrder::BigEndian),
            offset: Some(1),
            align: Some(2),
            round: Some(3),
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
            byte_order: None,
            bit_numbering: BitNumbering::LSB0,
            offset: None,
            align: None,
            round: None,
            members: vec![],
        }
    }

    fn make_bit_field_with_members() -> Field {
        Field::Bit {
            ident: parse_quote!(_bit_field),
            ty: parse_quote!(u16),
            byte_order: None,
            bit_numbering: BitNumbering::LSB0,
            offset: None,
            align: None,
            round: None,
            members: vec![
                BitFieldMember { member: parse_quote!(foo), ty: parse_quote!(u8), bits: 4..7 },
                BitFieldMember { member: parse_quote!(bar), ty: parse_quote!(i8), bits: 0..4 },
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
            %self = self
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
            %self = self

            %bf0 = empty_bit_field [u16]
            
            %foo = member [foo, ref] %self
            %maybe_bf1 = pack_bit_field [4..7, LSB0] %foo %bf0
            %bf1 = try %maybe_bf1

            %bar = member [bar, ref] %self
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
