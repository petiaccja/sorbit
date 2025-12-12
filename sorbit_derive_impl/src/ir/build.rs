#![allow(unused)]

use super::nodes::{
    AndThen, Block, DeserializeComposite, DeserializeNothing, DeserializeObject, Direction, Enclose, Expr,
    ImplDeserialize, ImplSerialize, IntoBitField, Layout, Let, MakeStruct, MakeTuple, NewBitField, Ok, PackBitField,
    PackObject, SerializeComposite, SerializeNothing, SerializeObject, Statement, SymRef, Try, UnpackObject,
};

//------------------------------------------------------------------------------
// Trait implementation nodes.
//------------------------------------------------------------------------------

pub fn impl_serialize(name: syn::Ident, generics: syn::Generics, body: Expr) -> ImplSerialize {
    ImplSerialize { name, generics, body }
}

pub fn impl_deserialize(name: syn::Ident, generics: syn::Generics, body: Expr) -> ImplDeserialize {
    ImplDeserialize { name, generics, body }
}

//------------------------------------------------------------------------------
// Language expression nodes.
//------------------------------------------------------------------------------

pub fn try_(expr: Expr) -> Expr {
    Expr::Try(Try { expr }.into())
}

pub fn make_tuple(elements: Vec<Expr>) -> Expr {
    Expr::MakeTuple(MakeTuple { elements })
}

pub fn make_struct(name: syn::Ident, members: Vec<(syn::Member, Expr)>) -> Expr {
    Expr::MakeStruct(MakeStruct { name, members })
}

pub fn and_then(result: Expr, value: Option<syn::Ident>, expr: Expr) -> Expr {
    Expr::AndThen(AndThen { result, value, expr }.into())
}

pub fn ok(expr: Expr) -> Expr {
    Expr::Ok(Ok { expr }.into())
}

pub fn block(statements: Vec<Statement>, result: Expr) -> Expr {
    Expr::Block(Block { statements, result }.into())
}

pub fn symref(ident: syn::Ident) -> Expr {
    Expr::Symref(SymRef { ident })
}

//------------------------------------------------------------------------------
// Language statement nodes.
//------------------------------------------------------------------------------

pub fn let_(ident: Option<syn::Ident>, expr: Expr) -> Statement {
    Statement::Let(Let { ident, expr })
}

//------------------------------------------------------------------------------
// Serialization expression nodes.
//------------------------------------------------------------------------------

pub fn enclose(expr: Expr, item: String) -> Expr {
    Expr::Enclose(Enclose { item, expr }.into())
}

pub fn new_bit_field(ty: syn::Type) -> Expr {
    Expr::NewBitField(NewBitField { ty }.into())
}

pub fn into_bit_field(packed: Expr) -> Expr {
    Expr::IntoBitField(IntoBitField { packed }.into())
}

pub fn layout(
    expr: Expr,
    offset: Option<u64>,
    align: Option<u64>,
    len: Option<u64>,
    round: Option<u64>,
    direction: Direction,
) -> Expr {
    Expr::Layout(Layout { expr, offset, align, round, len, direction }.into())
}

pub fn layout_se(expr: Expr, offset: Option<u64>, align: Option<u64>, len: Option<u64>, round: Option<u64>) -> Expr {
    layout(expr, offset, align, len, round, Direction::Serialize)
}

pub fn layout_de(expr: Expr, offset: Option<u64>, align: Option<u64>, len: Option<u64>, round: Option<u64>) -> Expr {
    layout(expr, offset, align, len, round, Direction::Deserialize)
}

pub fn serialize_nothing() -> Expr {
    Expr::SerializeNothing(SerializeNothing)
}

pub fn serialize_object(object: syn::Expr) -> Expr {
    Expr::SerializeObject(SerializeObject { object })
}

pub fn serialize_composite(expr: Expr) -> Expr {
    Expr::SerializeComposite(SerializeComposite { expr }.into())
}

pub fn deserialize_nothing() -> Expr {
    Expr::DeserializeNothing(DeserializeNothing)
}

pub fn deserialize_object(ty: syn::Type) -> Expr {
    Expr::DeserializeObject(DeserializeObject { ty })
}

pub fn deserialize_composite(expr: Expr) -> Expr {
    Expr::DeserializeComposite(DeserializeComposite { expr }.into())
}

pub fn pack_object(bit_field: syn::Expr, object: syn::Expr, bit_range: std::ops::Range<u8>) -> Expr {
    Expr::PackObject(PackObject { bit_field, object, bit_range })
}

pub fn pack_bit_field(bit_field: syn::Ident, packed_ty: syn::Type, members: Vec<Expr>) -> Expr {
    Expr::PackBitField(PackBitField { bit_field, packed_ty, members })
}

pub fn unpack_object(bit_field: syn::Expr, ty: syn::Type, bit_range: std::ops::Range<u8>) -> Expr {
    Expr::UnpackObject(UnpackObject { bit_field, ty, bit_range })
}
