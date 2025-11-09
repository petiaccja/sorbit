use std::ops::Range;

use super::nodes::*;

pub fn deserialize_impl(name: syn::Ident, generics: syn::Generics, body: Expr) -> DeserializeImpl {
    DeserializeImpl { name, generics, body }
}

pub fn r#let(ident: Option<syn::Ident>, expr: Expr) -> Let {
    Let { ident, expr }
}

pub fn r#try(expr: Expr) -> Expr {
    Try { expr: expr.into() }.into()
}

pub fn block(statements: Vec<Let>, result: Expr) -> Expr {
    Block { statements, result: result.into() }.into()
}
pub fn construct(ty: syn::Type, args: Vec<syn::Ident>) -> Expr {
    Construct { ty, args }.into()
}
pub fn name(ident: syn::Ident) -> Expr {
    Name { ident }.into()
}

pub fn ok(expr: Expr) -> Expr {
    Ok { expr: expr.into() }.into()
}

pub fn bit_field_from(bits: Expr) -> Expr {
    BitFieldFrom { bits: bits.into() }.into()
}

pub fn enclose(expr: Expr, item: String) -> Expr {
    Enclose { expr: expr.into(), item }.into()
}

pub fn pad(until: u64) -> Expr {
    Pad { until }.into()
}

pub fn align(multiple_of: u64) -> Expr {
    Align { multiple_of }.into()
}

pub fn deserialize_object(ty: syn::Type) -> Expr {
    DeserializeObject { ty }.into()
}

pub fn deserialize_composite(body: Expr) -> Expr {
    DeserializeComposite { body: body.into() }.into()
}

pub fn unpack_object(bit_field: syn::Expr, ty: syn::Type, bit_range: Range<u8>) -> Expr {
    UnpackObject { bit_field, ty, bit_range }.into()
}
