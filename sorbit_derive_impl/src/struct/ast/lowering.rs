use crate::attribute::ByteOrder;
use crate::ir::dag::{Region, Value};
use crate::ir::ops::{
    self as ops, align, deserialize_composite, deserialize_object, member, ok, pad, serialize_composite,
    serialize_object, try_, yield_,
};

pub trait ToSerializeOp {
    type Args;
    fn to_serialize_op(&self, region: &mut Region, args: Self::Args) -> Vec<Value>;
}

pub trait ToDeserializeOp {
    type Args;
    fn to_deserialize_op(&self, region: &mut Region, args: Self::Args) -> Vec<Value>;
}

pub fn lower_offset(region: &mut Region, serializer: Value, offset: Option<u64>, serializing: bool) {
    if let Some(offset) = offset {
        let maybe_offset = pad(region, serializer, offset, serializing);
        let _ = try_(region, maybe_offset);
    }
}

pub fn lower_alignment(region: &mut Region, serializer: Value, align: Option<u64>, serializing: bool) {
    if let Some(align) = align {
        let align = ops::align(region, serializer, align, serializing);
        let _ = try_(region, align);
    }
}

pub fn lower_serialization_rounding(
    region: &mut Region,
    serializer: Value,
    object: Value,
    byte_order: Option<ByteOrder>,
    round: Option<u64>,
) -> Value {
    if let Some(round) = round {
        let maybe_serialized = serialize_composite(region, serializer, |region, serializer| {
            let maybe_serialized = lower_serialization_byte_order(region, serializer.clone(), object, byte_order);
            let maybe_round = align(region, serializer.clone(), round, true);
            let _ = try_(region, maybe_round);
            let _ = yield_(region, vec![maybe_serialized]);
        });
        let serialized = try_(region, maybe_serialized);
        let composite_span = member(region, serialized, syn::Member::Unnamed(syn::Index::from(1)), false);
        ok(region, composite_span)
    } else {
        lower_serialization_byte_order(region, serializer, object, byte_order)
    }
}

pub fn lower_deserialization_rounding(
    region: &mut Region,
    deserializer: Value,
    ty: syn::Type,
    byte_order: Option<ByteOrder>,
    round: Option<u64>,
) -> Value {
    if let Some(round) = round {
        deserialize_composite(region, deserializer, |region, deserializer| {
            let maybe_deserialized = lower_deserialization_byte_order(region, deserializer.clone(), ty, byte_order);
            let maybe_round = align(region, deserializer.clone(), round, false);
            let _ = try_(region, maybe_round);
            let _ = yield_(region, vec![maybe_deserialized]);
        })
    } else {
        lower_deserialization_byte_order(region, deserializer, ty, byte_order)
    }
}

pub fn lower_serialization_byte_order(
    region: &mut Region,
    serializer: Value,
    object: Value,
    byte_order: Option<ByteOrder>,
) -> Value {
    if let Some(byte_order) = byte_order {
        ops::byte_order(region, serializer, byte_order, true, |region, serializer| {
            let result = serialize_object(region, serializer.clone(), object);
            let _ = yield_(region, vec![result]);
        })
    } else {
        serialize_object(region, serializer.clone(), object)
    }
}

pub fn lower_deserialization_byte_order(
    region: &mut Region,
    serializer: Value,
    ty: syn::Type,
    byte_order: Option<ByteOrder>,
) -> Value {
    if let Some(byte_order) = byte_order {
        ops::byte_order(region, serializer, byte_order, false, |region, serializer| {
            let result = deserialize_object(region, serializer.clone(), ty);
            let _ = yield_(region, vec![result]);
        })
    } else {
        deserialize_object(region, serializer.clone(), ty)
    }
}
