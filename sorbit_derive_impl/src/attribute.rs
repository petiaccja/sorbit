use std::collections::HashMap;
use std::fmt::Display;
use std::ops::{Add, Range};
use std::str::FromStr;

use quote::ToTokens;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Comma;
use syn::{Attribute, Expr, ExprLit, ExprRange, Ident, Lit, Meta, Path, RangeLimits, Type, TypePath};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ByteOrder {
    BigEndian,
    LittleEndian,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BitNumbering {
    MSB0,
    #[default]
    LSB0,
}

pub mod path {
    use syn::Path;
    use syn::parse_quote;

    pub fn sorbit_attribute() -> Path {
        parse_quote!(sorbit)
    }

    pub fn storage_id() -> Path {
        parse_quote!(bit_field)
    }

    pub fn storage_ty() -> Path {
        parse_quote!(repr)
    }

    pub fn bit_range() -> Path {
        parse_quote!(bits)
    }

    pub fn offset() -> Path {
        parse_quote!(offset)
    }

    pub fn align() -> Path {
        parse_quote!(align)
    }

    pub fn round() -> Path {
        parse_quote!(round)
    }

    pub fn len() -> Path {
        parse_quote!(len)
    }

    pub fn byte_order() -> Path {
        parse_quote!(byte_order)
    }

    pub fn bit_numbering() -> Path {
        parse_quote!(bit_numbering)
    }
}

pub fn parse_nvp_attribute(attribute: &Attribute) -> Result<HashMap<Path, Expr>, syn::Error> {
    let meta_list = attribute.meta.require_list()?;
    let metas = meta_list.parse_args_with(|parse_buffer: &syn::parse::ParseBuffer<'_>| {
        Punctuated::<Meta, Comma>::parse_terminated(parse_buffer)
    })?;

    let mut name_values = HashMap::new();
    for meta in metas {
        let name_value = meta.require_name_value()?;
        name_values.insert(name_value.path.clone(), name_value.value.clone());
    }

    Ok(name_values)
}

pub fn parse_nvp_attribute_group<'attr>(
    attributes: impl Iterator<Item = &'attr Attribute>,
) -> Result<HashMap<Path, Expr>, syn::Error> {
    use std::collections::hash_map::Entry::*;

    let mut merged = HashMap::new();
    for attribute in attributes {
        let nvps = parse_nvp_attribute(attribute)?;
        for (name, value) in nvps {
            match merged.entry(name) {
                Occupied(entry) => {
                    if entry.get() != &value {
                        return Err(syn::Error::new(
                            value.span(),
                            format!(
                                "parameter `{}` redefined with a different value",
                                entry.key().to_token_stream().to_string()
                            ),
                        ));
                    }
                }
                Vacant(entry) => {
                    entry.insert(value);
                }
            };
        }
    }
    Ok(merged)
}

pub fn as_ident(expr: &Expr) -> Result<Ident, syn::Error> {
    match expr {
        Expr::Path(path) => path.path.require_ident().cloned(),
        _ => Err(syn::Error::new(expr.span(), "expected an identifier")),
    }
}

pub fn as_type(expr: &Expr) -> Result<Type, syn::Error> {
    match expr {
        Expr::Path(path) => Ok(Type::from(TypePath { qself: None, path: path.path.clone() })),
        _ => Err(syn::Error::new(expr.span(), "expected a type")),
    }
}

pub fn as_literal_int<N>(expr: &Expr) -> Result<N, syn::Error>
where
    N: FromStr<Err: Display> + Display,
{
    match expr {
        Expr::Lit(ExprLit { attrs: _, lit: Lit::Int(int) }) => int.base10_parse(),
        _ => Err(syn::Error::new(expr.span(), "expected a literal integer")),
    }
}

pub fn as_literal_int_range<N>(expr: &Expr) -> Result<Range<N>, syn::Error>
where
    N: FromStr<Err: Display> + Display + Add<Output = N> + TryFrom<u8> + Copy,
{
    match expr {
        Expr::Range(ExprRange { attrs: _, start: Some(start_expr), limits, end: Some(end_expr) }) => {
            let one: N =
                1.try_into().map_err(|_| syn::Error::new(expr.span(), "could not convert 1 to N, this is a bug"))?;
            let start = as_literal_int(start_expr)?;
            let end_raw = as_literal_int(end_expr)?;
            let end = match limits {
                RangeLimits::HalfOpen(_) => end_raw,
                RangeLimits::Closed(_) => end_raw + one,
            };
            Ok(start..end)
        }
        _ => return Err(syn::Error::new(expr.span(), "expected a bounded literal integer range (e.g. 1..4, 1..=3")),
    }
}

pub fn as_byte_order(expr: &Expr) -> Result<ByteOrder, syn::Error> {
    let ident = as_ident(expr)?;
    match ident.to_string().as_str() {
        "big_endian" => Ok(ByteOrder::BigEndian),
        "little_endian" => Ok(ByteOrder::LittleEndian),
        _ => Err(syn::Error::new(expr.span(), "byte order may be `big_endian`, `little_endian`, or `inherited`")),
    }
}

pub fn as_bit_numbering(expr: &Expr) -> Result<BitNumbering, syn::Error> {
    let ident = as_ident(expr)?;
    match ident.to_string().to_uppercase().as_str() {
        "MSB0" => Ok(BitNumbering::MSB0),
        "LSB0" => Ok(BitNumbering::LSB0),
        _ => Err(syn::Error::new(expr.span(), "bit numbering may be `MSB0` or `LSB0`")),
    }
}

impl std::fmt::Display for ByteOrder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::fmt::Display for BitNumbering {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
