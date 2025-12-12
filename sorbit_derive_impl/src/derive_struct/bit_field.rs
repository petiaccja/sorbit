use syn::Ident;

use crate::derive_struct::bit_field_attribute::BitFieldAttribute;
use crate::derive_struct::field_utils::{
    lower_alignment, lower_deserialization_rounding, lower_offset, lower_serialization_rounding,
};
use crate::ssa_ir::ir::{Operation, Region, Value};
use crate::ssa_ir::ops::{
    EmptyBitFieldOp, ExecuteOp, IntoBitFieldOp, IntoRawBitsOp, MemberOp, PackBitFieldOp, RefOp, SelfOp, TryOp,
    UnpackBitFieldOp, YieldOp,
};

use super::packed_field::PackedField;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitField {
    pub name: Ident,
    pub attribute: BitFieldAttribute,
    pub members: Vec<PackedField>,
}

impl BitField {
    pub fn lower_se(&self, serializer: Value) -> Operation {
        ExecuteOp::new(Region::new(0, |_| {
            let mut ops = Vec::new();
            let self_ = SelfOp::new();
            let self_output = self_.output();
            ops.extend([self_.operation].into_iter());

            let mut bf = EmptyBitFieldOp::new(self.attribute.repr.clone()).operation;

            for member in &self.members {
                let mem = MemberOp::new(self_output.clone(), member.name.clone(), true);
                let maybe_new_bf =
                    PackBitFieldOp::new(serializer.clone(), mem.output(), bf.output(0), member.attribute.bits.clone());
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

            lower_offset(serializer.clone(), self.attribute.offset, true, &mut ops);
            lower_alignment(serializer.clone(), self.attribute.align, true, &mut ops);
            let output = lower_serialization_rounding(serializer, raw_ref_out, self.attribute.round, &mut ops);
            ops.push(YieldOp::new(vec![output]).operation);

            ops
        }))
        .operation
    }

    pub fn lower_de(&self, deserializer: Value) -> Operation {
        ExecuteOp::new(Region::new(0, |_| {
            let mut ops = Vec::new();

            lower_offset(deserializer.clone(), self.attribute.offset, false, &mut ops);
            lower_alignment(deserializer.clone(), self.attribute.align, false, &mut ops);
            let maybe_raw_bits = lower_deserialization_rounding(
                deserializer,
                self.attribute.repr.clone(),
                self.attribute.round,
                &mut ops,
            );
            let raw_bits = TryOp::new(maybe_raw_bits);
            let bit_field = IntoBitFieldOp::new(raw_bits.output());
            let bit_field_output = bit_field.output();
            ops.extend([raw_bits.operation, bit_field.operation].into_iter());

            let _unpacked = self
                .members
                .iter()
                .map(|member| {
                    let maybe_mem = UnpackBitFieldOp::new(
                        member.ty.clone(),
                        bit_field_output.clone(),
                        member.attribute.bits.clone(),
                    );
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

#[cfg(test)]
mod tests {
    use super::*;

    use syn::parse_quote;

    use crate::{
        derive_struct::{
            bit_field::BitField, bit_field_attribute::BitFieldAttribute, packed_field::PackedField,
            packed_field_attribute::PackedFieldAttribute,
        },
        ssa_ir::ir::assert_matches,
    };

    fn make_empty() -> BitField {
        BitField {
            name: parse_quote!(bf),
            attribute: BitFieldAttribute { repr: parse_quote!(u16), ..Default::default() },
            members: vec![],
        }
    }

    fn make_two_members() -> BitField {
        BitField {
            name: parse_quote!(bf),
            attribute: BitFieldAttribute { repr: parse_quote!(u16), ..Default::default() },
            members: vec![
                PackedField {
                    name: parse_quote!(foo),
                    ty: parse_quote!(u8),
                    attribute: PackedFieldAttribute { storage: parse_quote!(bf), bits: 4..7 },
                },
                PackedField {
                    name: parse_quote!(bar),
                    ty: parse_quote!(i8),
                    attribute: PackedFieldAttribute { storage: parse_quote!(bf), bits: 0..4 },
                },
            ],
        }
    }

    #[test]
    fn lower_se_empty() {
        let input = make_empty();
        let serializer = Value::new_standalone();
        let op = format!("{:#?}", input.lower_se(serializer));
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
    fn lower_se_two_members() {
        let input = make_two_members();
        let serializer = Value::new_standalone();
        let op = format!("{:#?}", input.lower_se(serializer));
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
    fn lower_de_empty() {
        let input = make_empty();
        let deserializer = Value::new_standalone();
        let op = format!("{:#?}", input.lower_de(deserializer));
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
    fn lower_de_two_members() {
        let input = make_two_members();
        let deserializer = Value::new_standalone();
        let op = format!("{:#?}", input.lower_de(deserializer));
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
