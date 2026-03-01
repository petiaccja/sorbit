use std::collections::HashSet;

use syn::{Expr, Ident, Type, spanned::Spanned};

use crate::attribute::{as_literal_bool, parse_nvp_attribute_group, path};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant {
    pub ident: Ident,
    pub discriminant: Option<Expr>,
    /// The variant is a "catch all" for unrecognized discriminants.
    ///
    /// Meaning:
    /// - `Some(Some(ty))`: variant is the catch all, and stores the discriminant as `ty` (i.e. `#[sorbit(catch_all)] CatchAll(u8)`)
    /// - `Some(None)`: variant is the catch all, but stores no discriminant (i.e. `#[sorbit(catch_all)] CatchAll`)
    /// - `None`: variant is not the catch all (i.e. `NotCatchAll`)
    pub catch_all: Option<Option<Type>>,
}

impl TryFrom<syn::Variant> for Variant {
    type Error = syn::Error;
    fn try_from(value: syn::Variant) -> Result<Self, Self::Error> {
        let sorbit_attrs = value.attrs.iter().filter(|attr| attr.path() == &path::sorbit_attribute());
        let parameters = parse_nvp_attribute_group(sorbit_attrs)?;

        let accepted_parameters: HashSet<_> = [path::catch_all()].into_iter().collect();
        for (name, _) in &parameters {
            if !accepted_parameters.contains(&name) {
                return Err(syn::Error::new(name.span(), "unrecognized parameter"));
            }
        }

        let discriminant = value.discriminant.map(|(_, expr)| expr);
        let catch_all_tag =
            parameters.get(&path::catch_all()).map(|expr| as_literal_bool(expr)).transpose()?.unwrap_or(false);

        if !catch_all_tag {
            if !value.fields.is_empty() {
                return Err(syn::Error::new(value.fields.span(), "only fieldless enums are supported"));
            }
            Ok(Self { ident: value.ident, discriminant, catch_all: None })
        } else {
            if value.fields.len() > 1 {
                Err(syn::Error::new(
                    value.fields.span(),
                    "catch all variant must be a unit variant or a tuple variant with exactly one field of the enum's repr type",
                ))
            } else if let Some(catch_all_field) = value.fields.into_iter().next() {
                if catch_all_field.ident.is_some() {
                    return Err(syn::Error::new(
                        catch_all_field.span(),
                        "catch all variant must be a unit variant or a tuple variant with exactly one field of the enum's repr type",
                    ));
                }
                Ok(Self { ident: value.ident, discriminant, catch_all: Some(Some(catch_all_field.ty)) })
            } else {
                Ok(Self { ident: value.ident, discriminant, catch_all: Some(None) })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use syn::parse_quote;

    #[test]
    fn simple() {
        let input: syn::Variant = parse_quote!(A);
        let actual = Variant::try_from(input).unwrap();
        let expected = Variant { ident: parse_quote!(A), discriminant: None, catch_all: None };
        assert_eq!(actual, expected);
    }

    #[test]
    fn catch_all_empty() {
        let input: syn::Variant = parse_quote!(
            #[sorbit(catch_all)]
            A
        );
        let actual = Variant::try_from(input).unwrap();
        let expected = Variant { ident: parse_quote!(A), discriminant: None, catch_all: Some(None) };
        assert_eq!(actual, expected);
    }

    #[test]
    fn catch_all_tuple() {
        let input: syn::Variant = parse_quote!(
            #[sorbit(catch_all)]
            A(u8)
        );
        let actual = Variant::try_from(input).unwrap();
        let expected = Variant { ident: parse_quote!(A), discriminant: None, catch_all: Some(Some(parse_quote!(u8))) };
        assert_eq!(actual, expected);
    }

    #[test]
    #[should_panic]
    fn catch_all_struct() {
        let input: syn::Variant = parse_quote!(
            #[sorbit(catch_all)]
            A { a: u8 }
        );
        let _ = Variant::try_from(input).unwrap();
    }

    #[test]
    fn discriminant() {
        let input: syn::Variant = parse_quote!(A = 34);
        let actual = Variant::try_from(input).unwrap();
        let expected = Variant { ident: parse_quote!(A), discriminant: Some(parse_quote!(34)), catch_all: None };
        assert_eq!(actual, expected);
    }
}
