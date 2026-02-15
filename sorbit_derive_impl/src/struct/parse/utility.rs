use std::fmt::Display;
use std::ops::{Add, Range};
use std::str::FromStr;

use quote::ToTokens;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Comma;
use syn::{Attribute, Expr, ExprLit, ExprRange, Ident, Lit, Meta, Path, RangeLimits, Type, parse_quote};

pub fn sorbit_layout_path() -> Path {
    parse_quote! {sorbit_layout}
}

pub fn sorbit_bit_field_path() -> Path {
    parse_quote! {sorbit_bit_field}
}

pub fn parse_literal_int_meta<N>(value: &mut Option<N>, meta: &Meta) -> Result<(), syn::Error>
where
    N: FromStr<Err: Display> + Display,
{
    if value.is_some() {
        let path = meta.path().to_token_stream();
        Err(syn::Error::new(meta.span(), format!("the parameter `{path}` has already been defined")))
    } else {
        let meta_nvp = meta.require_name_value()?;
        let Expr::Lit(ExprLit { attrs: _, lit: Lit::Int(lit_int) }) = &meta_nvp.value else {
            return Err(syn::Error::new(meta_nvp.value.span(), "expected an integer literal"));
        };
        *value = Some(lit_int.base10_parse()?);
        Ok(())
    }
}
pub fn parse_type_meta(value: &mut Option<Type>, meta: &Meta) -> Result<(), syn::Error> {
    if value.is_some() {
        let path = meta.path().to_token_stream();
        Err(syn::Error::new(meta.span(), format!("the parameter `{path}` has already been defined")))
    } else {
        let list = meta.require_list()?;
        *value = Some(list.parse_args()?);
        Ok(())
    }
}

pub fn parse_literal_range_meta<N>(value: &mut Option<Range<N>>, meta: &Meta) -> Result<(), syn::Error>
where
    N: FromStr<Err: Display> + Display + Add<Output = N> + TryFrom<u8> + Copy,
{
    if value.is_some() {
        let path = meta.path().to_token_stream();
        Err(syn::Error::new(meta.span(), format!("the parameter `{path}` has already been defined")))
    } else {
        let list = meta.require_list()?;
        let expr = list.parse_args::<Expr>()?;
        let one: N =
            1.try_into().map_err(|_| syn::Error::new(meta.span(), "could not convert 1 to N, this is a bug"))?;
        match expr {
            Expr::Lit(ExprLit { attrs: _, lit: Lit::Int(lit_start) }) => {
                let start: N = lit_start.base10_parse()?;
                *value = Some(start..(start + one));
            }
            Expr::Range(ExprRange { attrs: _, start: Some(start), limits, end: Some(end) }) => {
                let Expr::Lit(ExprLit { attrs: _, lit: Lit::Int(lit_start) }) = *start else {
                    return Err(syn::Error::new(start.span(), "expected a literal integer"));
                };
                let Expr::Lit(ExprLit { attrs: _, lit: Lit::Int(lit_end) }) = *end else {
                    return Err(syn::Error::new(end.span(), "expected a literal integer"));
                };
                let start: N = lit_start.base10_parse()?;
                let end_unadjusted: N = lit_end.base10_parse()?;
                let end = match limits {
                    RangeLimits::HalfOpen(_) => end_unadjusted,
                    RangeLimits::Closed(_) => end_unadjusted + one,
                };
                *value = Some(start..end);
            }
            _ => return Err(syn::Error::new(expr.span(), "expected a literal integer or a bounded literal range")),
        }
        Ok(())
    }
}

pub fn parse_bit_field_name(meta: &Meta) -> Result<Ident, syn::Error> {
    let meta_list = meta.require_list()?;
    let meta_items = meta_list.parse_args_with(|parse_buffer: &syn::parse::ParseBuffer<'_>| {
        Punctuated::<Meta, Comma>::parse_terminated(parse_buffer)
    })?;
    let name = meta_items.first().ok_or(syn::Error::new(meta.span(), "expected non-empty meta list"))?;
    let name_path = name.require_path_only()?;
    name_path.get_ident().cloned().ok_or(syn::Error::new(name_path.span(), "expected an identifier"))
}

pub fn parse_meta_list_attr(
    attr: &Attribute,
    parameters: &mut [(&str, &mut dyn FnMut(&Meta) -> Result<(), syn::Error>)],
) -> Result<(), syn::Error> {
    let meta_list = attr.meta.require_list()?;
    let metas = meta_list.parse_args_with(|parse_buffer: &syn::parse::ParseBuffer<'_>| {
        Punctuated::<Meta, Comma>::parse_terminated(parse_buffer)
    })?;

    for meta in metas {
        let mut parsed = false;
        for (ident, parser) in parameters.iter_mut() {
            if meta.path().is_ident(ident) {
                parser(&meta)?;
                parsed = true;
                break;
            }
        }
        if !parsed {
            let path = meta.path().to_token_stream();
            return Err(syn::Error::new(meta.span(), format!("the parameter `{path}` is not accepted here")));
        }
    }

    Ok(())
}
