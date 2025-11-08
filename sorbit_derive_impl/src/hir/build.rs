use std::ops::Range;

use super::nodes::*;

pub fn serialize_impl(name: syn::Ident, generics: syn::Generics, body: Expr) -> SerializeImpl {
    SerializeImpl { name, generics, body }
}

pub fn pad(until: u64) -> Expr {
    Pad { until }.into()
}

pub fn align(multiple_of: u64) -> Expr {
    Align { multiple_of }.into()
}

#[allow(unused)]
pub fn serialize_nothing() -> Expr {
    SerializeNothing {}.into()
}

pub fn serialize_object(object: syn::Expr) -> Expr {
    SerializeObject { object }.into()
}

pub fn serialize_composite(members: Vec<Expr>) -> Expr {
    SerializeComposite { members }.into()
}

pub fn enclose(expr: Expr, item: String) -> Expr {
    Enclose { expr: expr.into(), item }.into()
}

pub fn chain(exprs: Vec<Expr>) -> Expr {
    Chain::new_placeholder_vars(exprs).into()
}

pub fn chain_with_vars(exprs: Vec<Expr>, vars: Vec<Option<syn::Ident>>) -> Expr {
    Chain::new(exprs, vars).into()
}

pub fn pack_object(bit_field: syn::Expr, object: syn::Expr, bit_range: Range<u8>) -> Expr {
    PackObject { bit_field, object, bit_range }.into()
}

pub fn pack_bit_field(bit_field: syn::Ident, packed_ty: syn::Type, members: Vec<Expr>) -> Expr {
    PackBitField { bit_field, packed_ty, members }.into()
}
