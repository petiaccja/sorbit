use proc_macro2::Span;
use syn::{BinOp, Expr, ExprBinary, ExprLit, Generics, Ident, Lit, LitInt, Token, Type, parse_quote};

use crate::attribute::ByteOrder;
use crate::r#enum::ast::variant::Variant;
use crate::r#enum::parse;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Enum {
    pub ident: Ident,
    pub storage_ty: Type,
    pub generics: Generics,
    pub byte_order: Option<ByteOrder>,
    pub default: Option<Ident>,
    pub variants: Vec<Variant>,
}

impl From<parse::Enum> for Enum {
    fn from(mut value: parse::Enum) -> Self {
        let default = value.variants.iter().find(|variant| variant.default).map(|variant| variant.ident.clone());
        let discriminants = compute_discriminants(value.variants.iter_mut().map(|variant| variant.discriminant.take()));
        let variants = std::iter::zip(value.variants.into_iter(), discriminants.into_iter())
            .map(|(variant, discriminant)| Variant { ident: variant.ident, discriminant })
            .collect();
        Self {
            ident: value.ident,
            storage_ty: value.storage_ty.unwrap_or(parse_quote!(isize)),
            generics: value.generics,
            byte_order: value.byte_order,
            default,
            variants,
        }
    }
}

fn compute_discriminants(variants: impl Iterator<Item = Option<Expr>>) -> Vec<Expr> {
    variants
        .scan((None, 0isize), |(prev, increment), current| match (&prev, current) {
            (_, Some(current)) => {
                *prev = Some(current.clone());
                *increment = 0;
                Some(current)
            }
            (Some(prev), None) => {
                *increment += 1;
                Some(Expr::Binary(ExprBinary {
                    attrs: vec![],
                    left: Box::new(prev.clone()),
                    op: BinOp::Add(Token![+](Span::call_site())),
                    right: Box::new(literal_int_expr(*increment)),
                }))
            }
            (None, None) => {
                *prev = Some(literal_int_expr(0));
                prev.clone()
            }
        })
        .collect()
}

fn literal_int_expr(value: isize) -> Expr {
    Expr::Lit(ExprLit { attrs: vec![], lit: Lit::Int(LitInt::new(&format!("{value}"), Span::call_site())) })
}

#[cfg(test)]
mod tests {
    use super::*;

    use syn::parse_quote;

    #[test]
    fn compute_discriminants_implicit() {
        let input = [None, None];
        let expected: [Expr; 2] = [parse_quote!(0), parse_quote!(0 + 1)];
        let result = compute_discriminants(input.into_iter());
        assert_eq!(result, expected);
    }

    #[test]
    fn compute_discriminants_explicit() {
        let input = [Some(parse_quote!(754)), None, Some(parse_quote!(854)), None];
        let expected: [Expr; 4] = [
            parse_quote!(754),
            parse_quote!(754 + 1),
            parse_quote!(854),
            parse_quote!(854 + 1),
        ];
        let result = compute_discriminants(input.into_iter());
        assert_eq!(result, expected);
    }
}
