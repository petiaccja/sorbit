use std::fmt::Display;
use std::str::FromStr;

use quote::ToTokens;
use syn::meta::ParseNestedMeta;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Comma;
use syn::{Attribute, Expr, ExprLit, Lit, LitInt, Meta, Path, Type, parse_quote};

pub fn sorbit_layout_path() -> Path {
    parse_quote! {sorbit::layout}
}

pub fn sorbit_bit_field_path() -> Path {
    parse_quote! {sorbit::bit_field}
}

pub fn placeholder_type() -> Type {
    parse_quote! {::sorbit::Placeholder}
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
pub fn parse_type_meta(value: &mut Type, meta: &Meta) -> Result<(), syn::Error> {
    if value != &placeholder_type() {
        let path = meta.path().to_token_stream();
        Err(syn::Error::new(meta.span(), format!("the parameter `{path}` has already been defined")))
    } else {
        let list = meta.require_list()?;
        *value = list.parse_args()?;
        Ok(())
    }
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
