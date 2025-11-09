use quote::format_ident;
use syn::parse_quote;

use crate::{ir_de, ir_se};

pub fn member_to_ident(member: &syn::Member) -> syn::Ident {
    match member {
        syn::Member::Named(ident) => ident.clone(),
        syn::Member::Unnamed(index) => format_ident!("_{}", index),
    }
}

pub fn member_to_string(member: &syn::Member) -> String {
    match member {
        syn::Member::Named(ident) => ident.to_string(),
        syn::Member::Unnamed(index) => index.index.to_string(),
    }
}

pub fn lower_se_with_layout(
    value: &syn::Expr,
    name: Option<&str>,
    offset: Option<u64>,
    align: Option<u64>,
    round: Option<u64>,
) -> ir_se::Expr {
    let serialized = ir_se::serialize_object(value.clone());

    let rounded = match round {
        Some(round) => ir_se::serialize_composite(vec![serialized, ir_se::align(round)]),
        None => serialized,
    };

    let aligned = match align {
        Some(align) => ir_se::chain(vec![ir_se::align(align), rounded]).flatten(),
        None => rounded,
    };

    let offseted = match offset {
        Some(offset) => ir_se::chain(vec![ir_se::pad(offset), aligned]).flatten(),
        None => aligned,
    };

    match name {
        Some(display_name) => ir_se::enclose(offseted, display_name.into()),
        None => offseted,
    }
}

pub fn lower_de_with_layout(
    ident: &syn::Ident,
    ty: &syn::Type,
    name: Option<&str>,
    offset: Option<u64>,
    align: Option<u64>,
    round: Option<u64>,
) -> Vec<ir_de::Let> {
    let deserialized = ir_de::deserialize_object(ty.clone());

    let rounded = match round {
        Some(round) => ir_de::deserialize_composite(ir_de::block(
            vec![
                ir_de::r#let(Some(parse_quote!(value)), ir_de::r#try(deserialized)),
                ir_de::r#let(None, ir_de::r#try(ir_de::align(round))),
            ],
            ir_de::ok(ir_de::name(parse_quote!(value))),
        )),
        None => deserialized,
    };

    let statements = [
        offset.map(|offset| (None, ir_de::pad(offset))),
        align.map(|align| (None, ir_de::align(align))),
        Some((Some(ident.clone()), rounded)),
    ];

    statements
        .into_iter()
        .filter_map(|s| s)
        .map(|(ident, expr)| match name {
            Some(name) => (ident, ir_de::enclose(expr, name.into())),
            None => (ident, expr),
        })
        .map(|(ident, expr)| ir_de::r#let(ident, ir_de::r#try(expr)))
        .collect()
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    #[test]
    fn lower_se_display_name() {
        let actual = lower_se_with_layout(&parse_quote!(foo), Some("foo"), None, None, None);
        let expected = ir_se::enclose(ir_se::serialize_object(parse_quote!(foo)), "foo".into());
        assert_eq!(actual, expected);
    }

    #[test]
    fn lower_se_offset_and_align() {
        let actual = lower_se_with_layout(&parse_quote!(foo), None, Some(4), Some(6), None);
        let expected = ir_se::chain(vec![
            ir_se::pad(4),
            ir_se::align(6),
            ir_se::serialize_object(parse_quote!(foo)),
        ]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn lower_se_round() {
        let actual = lower_se_with_layout(&parse_quote!(foo), None, None, None, Some(6));
        let expected = ir_se::serialize_composite(vec![ir_se::serialize_object(parse_quote!(foo)), ir_se::align(6)]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn lower_de_display_name() {
        let actual = lower_de_with_layout(&parse_quote!(foo), &parse_quote!(i32), Some("foo"), None, None, None);
        let expected = [ir_de::r#let(
            Some(parse_quote!(foo)),
            ir_de::r#try(ir_de::enclose(ir_de::deserialize_object(parse_quote!(i32)), "foo".into())),
        )];
        assert_eq!(actual, expected);
    }

    #[test]
    fn lower_de_offset_and_align() {
        let actual = lower_de_with_layout(&parse_quote!(foo), &parse_quote!(i32), None, Some(4), Some(6), None);
        let expected = [
            ir_de::r#let(None, ir_de::r#try(ir_de::pad(4))),
            ir_de::r#let(None, ir_de::r#try(ir_de::align(6))),
            ir_de::r#let(Some(parse_quote!(foo)), ir_de::r#try(ir_de::deserialize_object(parse_quote!(i32)))),
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn lower_de_round() {
        let actual = lower_de_with_layout(&parse_quote!(foo), &parse_quote!(i32), None, None, None, Some(6));
        let expected = [ir_de::r#let(
            Some(parse_quote!(foo)),
            ir_de::r#try(ir_de::deserialize_composite(ir_de::block(
                vec![
                    ir_de::r#let(Some(parse_quote!(value)), ir_de::r#try(ir_de::deserialize_object(parse_quote!(i32)))),
                    ir_de::r#let(None, ir_de::r#try(ir_de::align(6))),
                ],
                ir_de::ok(ir_de::name(parse_quote!(value))),
            ))),
        )];
        assert_eq!(actual, expected);
    }
}
