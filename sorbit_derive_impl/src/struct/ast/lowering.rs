use crate::ir::dag::{Operation, Region, Value};
use crate::ir::ops::{
    AlignOp, DeserializeCompositeOp, DeserializeObjectOp, MemberOp, OkOp, PadOp, SerializeCompositeOp,
    SerializeObjectOp, TryOp, YieldOp,
};

pub trait ToSerializeOp {
    type Args;
    fn to_serialize_op(&self, args: Self::Args) -> Operation;
}

pub trait ToDeserializeOp {
    type Args;
    fn to_deserialize_op(&self, args: Self::Args) -> Operation;
}

pub fn lower_offset(serializer: Value, offset: Option<u64>, serializing: bool, ops: &mut Vec<Operation>) {
    if let Some(offset) = offset {
        let offset = PadOp::new(serializer, offset, serializing);
        let try_offset = TryOp::new(offset.output());
        ops.extend([offset.operation, try_offset.operation].into_iter());
    }
}

pub fn lower_alignment(serializer: Value, align: Option<u64>, serializing: bool, ops: &mut Vec<Operation>) {
    if let Some(align) = align {
        let align = AlignOp::new(serializer, align, serializing);
        let try_offset = TryOp::new(align.output());
        ops.extend([align.operation, try_offset.operation].into_iter());
    }
}

pub fn lower_serialization_rounding(
    serializer: Value,
    object: Value,
    round: Option<u64>,
    ops: &mut Vec<Operation>,
) -> Value {
    if let Some(round) = round {
        let serialize = SerializeCompositeOp::new(
            serializer,
            Region::new(1, |arguments| {
                let serializer = &arguments[0];
                let serialize = SerializeObjectOp::new(serializer.clone(), object);
                let round = AlignOp::new(serializer.clone(), round, true);
                let try_round = TryOp::new(round.output());
                let yield_ = YieldOp::new(vec![serialize.output()]);
                vec![
                    serialize.operation,
                    round.operation,
                    try_round.operation,
                    yield_.operation,
                ]
            }),
        );
        let try_serialize = TryOp::new(serialize.output());
        let serialize_1 = MemberOp::new(try_serialize.output(), syn::Member::Unnamed(syn::Index::from(1)), false);
        let ok_serialize = OkOp::new(serialize_1.output());
        let output = ok_serialize.output();
        ops.extend(
            [
                serialize.operation,
                try_serialize.operation,
                serialize_1.operation,
                ok_serialize.operation,
            ]
            .into_iter(),
        );
        output
    } else {
        let serialize = SerializeObjectOp::new(serializer, object);
        let output = serialize.output();
        ops.extend([serialize.operation].into_iter());
        output
    }
}

pub fn lower_deserialization_rounding(
    deserializer: Value,
    ty: syn::Type,
    round: Option<u64>,
    ops: &mut Vec<Operation>,
) -> Value {
    if let Some(round) = round {
        let deserialize = DeserializeCompositeOp::new(
            deserializer,
            Region::new(1, |arguments| {
                let deserializer = &arguments[0];
                let deserialize = DeserializeObjectOp::new(deserializer.clone(), ty);
                let round = AlignOp::new(deserializer.clone(), round, false);
                let try_round = TryOp::new(round.output());
                let yield_ = YieldOp::new(vec![deserialize.output()]);
                vec![
                    deserialize.operation,
                    round.operation,
                    try_round.operation,
                    yield_.operation,
                ]
            }),
        );
        let output = deserialize.output();
        ops.extend([deserialize.operation].into_iter());
        output
    } else {
        let deserialize = DeserializeObjectOp::new(deserializer, ty);
        let output = deserialize.output();
        ops.extend([deserialize.operation].into_iter());
        output
    }
}
