use crate::attribute::ByteOrder;
use crate::ir::dag::{Region, Value};
use crate::ir::ops::{self as ops, align, deserialize_composite, member, ok, pad, serialize_composite, try_, yield_};

pub fn with_maybe_offset(region: &mut Region, serializer: Value, offset: Option<u64>, serializing: bool) {
    if let Some(offset) = offset {
        let maybe_offset = pad(region, serializer, offset, serializing);
        let _ = try_(region, maybe_offset);
    }
}

pub fn with_maybe_alignment(region: &mut Region, serializer: Value, align: Option<u64>, serializing: bool) {
    if let Some(align) = align {
        let align = ops::align(region, serializer, align, serializing);
        let _ = try_(region, align);
    }
}

pub fn with_maybe_rounding(
    region: &mut Region,
    serializer: Value,
    round: Option<u64>,
    is_serializing: bool,
    body: impl FnOnce(&mut Region, Value) -> Value,
) -> Value {
    if let Some(round) = round {
        let composite_body = |region: &mut Region, deserializer| {
            let maybe_deserialized = body(region, deserializer);
            let maybe_round = align(region, deserializer.clone(), round, is_serializing);
            let _ = try_(region, maybe_round);
            let _ = yield_(region, vec![maybe_deserialized]);
        };
        match is_serializing {
            true => {
                let maybe_composite = serialize_composite(region, serializer, composite_body);
                let composite = try_(region, maybe_composite);
                let composite_body_span = member(region, composite, syn::Member::from(1), false);
                ok(region, composite_body_span)
            }
            false => deserialize_composite(region, serializer, composite_body),
        }
    } else {
        body(region, serializer)
    }
}

// let body = with_maybe_byte_order(region, deserializer, byte_order, false, |region, deserializer| {
//     deserialize_object_body(region, deserializer, ty)
// });

pub fn with_maybe_byte_order(
    region: &mut Region,
    serializer: Value,
    byte_order: Option<ByteOrder>,
    is_serializing: bool,
    body: impl FnOnce(&mut Region, Value) -> Value,
) -> Value {
    match byte_order {
        Some(byte_order) => ops::byte_order(region, serializer, byte_order, is_serializing, |region, serializer| {
            let result = body(region, serializer);
            let _ = yield_(region, vec![result]);
        }),
        None => (body)(region, serializer),
    }
}
