use std::ops::Range;

use syn::{Ident, Member, Type};

use super::lowering::{
    ToSerializeOp, lower_alignment, lower_deserialization_rounding, lower_offset, lower_serialization_rounding,
};

use crate::ir::dag::{Operation, Region, Value};
use crate::ir::ops::{
    EmptyBitFieldOp, ExecuteOp, IntoBitFieldOp, IntoRawBitsOp, MemberOp, PackBitFieldOp, RefOp, SelfOp, TryOp,
    UnpackBitFieldOp, YieldOp,
};
use crate::r#struct::ast::lowering::ToDeserializeOp;

pub enum Field {
    Direct {
        member: Member,
        ty: Type,
        offset: Option<u64>,
        align: Option<u64>,
        round: Option<u64>,
    },
    Bit {
        #[allow(unused)]
        ident: Ident,
        ty: Type,
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

    fn to_serialize_op(&self, serializer: Value) -> Operation {
        match self {
            Field::Direct { member, ty: _, offset, align, round } => {
                ExecuteOp::new(Region::new(0, |_| {
                    let mut ops = Vec::new();

                    let self_ = SelfOp::new();
                    let object = MemberOp::new(self_.output(), member.clone(), true);
                    let object_val = object.output();
                    ops.extend([self_.operation, object.operation].into_iter());

                    lower_offset(serializer.clone(), *offset, true, &mut ops);
                    lower_alignment(serializer.clone(), *align, true, &mut ops);
                    let output = lower_serialization_rounding(serializer, object_val, *round, &mut ops);
                    ops.push(YieldOp::new(vec![output]).operation);

                    ops
                }))
                .operation
            }
            Field::Bit { ident: _, ty, offset, align, round, members } => {
                ExecuteOp::new(Region::new(0, |_| {
                    let mut ops = Vec::new();
                    let self_ = SelfOp::new();
                    let self_output = self_.output();
                    ops.extend([self_.operation].into_iter());

                    let mut bf = EmptyBitFieldOp::new(ty.clone()).operation;

                    for member in members {
                        let mem = MemberOp::new(self_output.clone(), member.member.clone(), true);
                        let maybe_new_bf =
                            PackBitFieldOp::new(serializer.clone(), mem.output(), bf.output(0), member.bits.clone());
                        let new_bf = TryOp::new(maybe_new_bf.output()).operation;
                        ops.extend(
                            [
                                std::mem::replace(&mut bf, new_bf),
                                mem.operation,
                                maybe_new_bf.operation,
                            ]
                            .into_iter(),
                        );
                    }

                    let raw = IntoRawBitsOp::new(bf.output(0));
                    let raw_ref = RefOp::new(raw.output());
                    let raw_ref_out = raw_ref.output();
                    ops.extend([bf, raw.operation, raw_ref.operation].into_iter());

                    lower_offset(serializer.clone(), *offset, true, &mut ops);
                    lower_alignment(serializer.clone(), *align, true, &mut ops);
                    let output = lower_serialization_rounding(serializer, raw_ref_out, *round, &mut ops);
                    ops.push(YieldOp::new(vec![output]).operation);

                    ops
                }))
                .operation
            }
        }
    }
}

impl ToDeserializeOp for Field {
    type Args = Value;

    fn to_deserialize_op(&self, deserializer: Value) -> Operation {
        match self {
            Field::Direct { member: _, ty, offset, align, round } => {
                ExecuteOp::new(Region::new(0, |_| {
                    let mut ops = Vec::new();
                    lower_offset(deserializer.clone(), *offset, false, &mut ops);
                    lower_alignment(deserializer.clone(), *align, false, &mut ops);
                    let output = lower_deserialization_rounding(deserializer, ty.clone(), *round, &mut ops);
                    ops.push(YieldOp::new(vec![output]).operation);
                    ops
                }))
                .operation
            }
            Field::Bit { ident: _, ty, offset, align, round, members } => {
                ExecuteOp::new(Region::new(0, |_| {
                    let mut ops = Vec::new();

                    lower_offset(deserializer.clone(), *offset, false, &mut ops);
                    lower_alignment(deserializer.clone(), *align, false, &mut ops);
                    let maybe_raw_bits = lower_deserialization_rounding(deserializer, ty.clone(), *round, &mut ops);
                    let raw_bits = TryOp::new(maybe_raw_bits);
                    let bit_field = IntoBitFieldOp::new(raw_bits.output());
                    let bit_field_output = bit_field.output();
                    ops.extend([raw_bits.operation, bit_field.operation].into_iter());

                    let _unpacked = members
                        .iter()
                        .map(|member| {
                            let maybe_mem =
                                UnpackBitFieldOp::new(member.ty.clone(), bit_field_output.clone(), member.bits.clone());
                            let maybe_mem_output = maybe_mem.output();
                            ops.extend([maybe_mem.operation].into_iter());
                            maybe_mem_output
                        })
                        .collect();

                    let yield_ = YieldOp::new(_unpacked);
                    ops.extend([yield_.operation].into_iter());

                    ops
                }))
                .operation
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::ir::pattern_match::assert_matches;

    use quote::ToTokens;
    use syn::parse_quote;

    #[test]
    fn to_serialize_op_direct_default() {
        let input =
            Field::Direct { member: parse_quote!(foo), ty: parse_quote!(i32), offset: None, align: None, round: None };

        let serializer = Value::new_standalone();
        let op = format!("{:#?}", input.to_serialize_op(serializer));
        let pattern = "
        %out = execute || [
            %self = self
            %foo = member [foo, &] %self
            %res = serialize_object %serializer, %foo
            yield %res
        ]
        ";
        assert_matches!(op, pattern);
    }

    #[test]
    fn to_serialize_op_direct_layout() {
        let input = Field::Direct {
            member: parse_quote!(foo),
            ty: parse_quote!(i32),
            offset: Some(1),
            align: Some(2),
            round: Some(3),
        };

        let serializer = Value::new_standalone();
        let op = format!("{:#?}", input.to_serialize_op(serializer.clone()));
        let pattern = "
        %out = execute || [
            %self = self
            %foo = member [foo, &] %self

            %offset = pad [1] %serializer
            %try_offset = try %offset

            %align = align [2] %serializer
            %try_align = try %align
            
            %res = serialize_composite %serializer |%s_inner| [
                %res_inner = serialize_object %s_inner, %foo
                %round = align [3] %s_inner
                %try_round = try %round
                yield %res_inner
            ]
            %res_try = try %res
            %res_1 = member [1, *] %res_try
            %res_ok = ok %res_1
            yield %res_ok
        ]
        ";
        assert_matches!(op, pattern);
    }

    #[test]
    fn to_deserialize_op_direct_default() {
        let input =
            Field::Direct { member: parse_quote!(foo), ty: parse_quote!(i32), offset: None, align: None, round: None };

        let deserializer = Value::new_standalone();
        let op = format!("{:#?}", input.to_deserialize_op(deserializer));
        let pattern = "
        %out = execute || [
            %res = deserialize_object [i32] %serializer
            yield %res
        ]
        ";
        assert_matches!(op, pattern);
    }

    #[test]
    fn to_deserialize_op_direct_layout() {
        let input = Field::Direct {
            member: parse_quote!(foo),
            ty: parse_quote!(i32),
            offset: Some(1),
            align: Some(2),
            round: Some(3),
        };

        let deserializer = Value::new_standalone();
        let op = format!("{:#?}", input.to_deserialize_op(deserializer.clone()));
        let pattern = "
        %out = execute || [
            %offset = pad [1] %deserializer
            %try_offset = try %offset

            %align = align [2] %deserializer
            %try_align = try %align

            %res = deserialize_composite %deserializer |%des_inner| [
                %res_inner = deserialize_object [i32] %des_inner
                %round = align [3] %des_inner
                %try_round = try %round
                yield %res_inner
            ]
            yield %res
        ]
        ";
        assert_matches!(op, pattern);
        println!("{}", input.to_deserialize_op(deserializer).to_token_stream())
    }

    fn make_bit_field_empty() -> Field {
        Field::Bit {
            ident: parse_quote!(_bit_field),
            ty: parse_quote!(u16),
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
        let serializer = Value::new_standalone();
        let op = format!("{:#?}", input.to_serialize_op(serializer));
        let pattern = "
        %res = execute || [
        %self = self
            %bf = empty_bit_field [u16]
            %raw = into_raw_bits %bf
            %ref_raw = ref %raw
            %s = serialize_object %serializer %ref_raw
            yield %s
        ]
        ";
        assert_matches!(op, pattern);
    }

    #[test]
    fn to_serialize_op_bit_with_members() {
        let input = make_bit_field_with_members();
        let serializer = Value::new_standalone();
        let op = format!("{:#?}", input.to_serialize_op(serializer));
        let pattern = "
        %res = execute || [
            %self = self

            %bf0 = empty_bit_field [u16]
            
            %foo = member [foo, &] %self
            %maybe_bf1 = pack_bit_field [4..7] %serializer %foo %bf0
            %bf1 = try %maybe_bf1

            %bar = member [bar, &] %self
            %maybe_bf2 = pack_bit_field [0..4] %serializer %bar %bf1
            %bf2 = try %maybe_bf2

            %raw = into_raw_bits %bf2
            %ref_raw = ref %raw
            %s = serialize_object %serializer %ref_raw
            yield %s
        ]        
        ";
        assert_matches!(op, pattern);
    }

    #[test]
    fn to_deserialize_op_bit_empty() {
        let input = make_bit_field_empty();
        let deserializer = Value::new_standalone();
        let op = format!("{:#?}", input.to_deserialize_op(deserializer));
        let pattern = "
        execute || [
            %s = deserialize_object [u16] %deserializer
            %try_s = try %s
            %bf = into_bit_field %try_s
            yield
        ]
        ";
        assert_matches!(op, pattern);
    }

    #[test]
    fn to_deserialize_op_bit_with_members() {
        let input = make_bit_field_with_members();
        let deserializer = Value::new_standalone();
        let op = format!("{:#?}", input.to_deserialize_op(deserializer));
        let pattern = "
        %0, %1 = execute || [
            %s = deserialize_object [u16] %deserializer
            %try_s = try %s
            %bf = into_bit_field %try_s

            %maybe_foo = unpack_bit_field [u8, 4..7] %bf
            %maybe_bar = unpack_bit_field [i8, 0..4] %bf

            yield %maybe_foo, %maybe_bar
        ]
        ";
        assert_matches!(op, pattern);
    }
}
