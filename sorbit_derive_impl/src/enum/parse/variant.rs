use std::collections::HashSet;

use proc_macro2::Span;
use syn::{Attribute, DeriveInput, Fields, Generics, Token};
use syn::{Expr, Ident, Type, spanned::Spanned};

use crate::attribute::{as_literal_bool, parse_nvp_attribute_group, path};
use crate::r#struct::parse::Struct;

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
    pub content: Option<Struct>,
}

impl TryFrom<syn::Variant> for Variant {
    type Error = syn::Error;
    fn try_from(value: syn::Variant) -> Result<Self, Self::Error> {
        let sorbit_attrs = value.attrs.iter().filter(|attr| attr.path() == &path::sorbit_attribute());
        let parameters = parse_nvp_attribute_group(sorbit_attrs)?;

        let accepted_parameters: HashSet<_> = [
            path::catch_all(),
            path::byte_order(),
            path::len(),
            path::round(),
        ]
        .into_iter()
        .collect();
        for (name, _) in &parameters {
            if !accepted_parameters.contains(&name) {
                return Err(syn::Error::new(name.span(), "unrecognized parameter"));
            }
        }

        let catch_all_tag =
            parameters.get(&path::catch_all()).map(|expr| as_literal_bool(expr)).transpose()?.unwrap_or(false);
        let catch_all = match catch_all(&value, catch_all_tag) {
            Ok(value) => value,
            Err(value) => return value,
        };
        let discriminant = value.discriminant.map(|(_, expr)| expr);
        let content = if !catch_all_tag {
            content(value.ident.clone(), value.attrs, value.fields)?
        } else {
            None
        };

        Ok(Self { ident: value.ident, discriminant, catch_all, content })
    }
}

fn catch_all(value: &syn::Variant, catch_all_tag: bool) -> Result<Option<Option<Type>>, Result<Variant, syn::Error>> {
    Ok(if !catch_all_tag {
        None
    } else {
        if value.fields.len() > 1 {
            return Err(Err(syn::Error::new(
                value.fields.span(),
                "catch all variant must be a unit variant or a tuple variant with exactly one field of the enum's repr type",
            )));
        } else if let Some(catch_all_field) = value.fields.iter().next() {
            if catch_all_field.ident.is_some() {
                return Err(Err(syn::Error::new(
                    catch_all_field.span(),
                    "catch all variant must be a unit variant or a tuple variant with exactly one field of the enum's repr type",
                )));
            };
            Some(Some(catch_all_field.ty.clone()))
        } else {
            Some(None)
        }
    })
}

fn content(ident: Ident, attrs: Vec<Attribute>, fields: Fields) -> Result<Option<Struct>, syn::Error> {
    if fields.is_empty() {
        Ok(None)
    } else {
        let input = DeriveInput {
            attrs,
            vis: syn::Visibility::Public(Token![pub](Span::call_site())),
            ident,
            generics: Generics::default(),
            data: syn::Data::Struct(syn::DataStruct {
                struct_token: Token![struct](Span::call_site()),
                fields,
                semi_token: None,
            }),
        };
        Some(Struct::try_from(input)).transpose()
    }
}

#[cfg(test)]
mod tests {
    use crate::r#struct::parse::{Field, FieldLayoutProperties};

    use super::*;

    use syn::parse_quote;

    #[test]
    fn simple() {
        let input: syn::Variant = parse_quote!(A);
        let actual = Variant::try_from(input).unwrap();
        let expected = Variant { ident: parse_quote!(A), discriminant: None, catch_all: None, content: None };
        assert_eq!(actual, expected);
    }

    #[test]
    fn catch_all_empty() {
        let input: syn::Variant = parse_quote!(
            #[sorbit(catch_all)]
            A
        );
        let actual = Variant::try_from(input).unwrap();
        let expected = Variant { ident: parse_quote!(A), discriminant: None, catch_all: Some(None), content: None };
        assert_eq!(actual, expected);
    }

    #[test]
    fn catch_all_tuple() {
        let input: syn::Variant = parse_quote!(
            #[sorbit(catch_all)]
            A(u8)
        );
        let actual = Variant::try_from(input).unwrap();
        let expected = Variant {
            ident: parse_quote!(A),
            discriminant: None,
            catch_all: Some(Some(parse_quote!(u8))),
            content: None,
        };
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
        let expected =
            Variant { ident: parse_quote!(A), discriminant: Some(parse_quote!(34)), catch_all: None, content: None };
        assert_eq!(actual, expected);
    }

    #[test]
    fn struct_content() {
        let input: syn::Variant = parse_quote!(
            #[sorbit(len = 12)]
            A {
                #[sorbit(offset = 2)]
                a: u8
            }
        );
        let actual = Variant::try_from(input).unwrap();
        let expected = Variant {
            ident: parse_quote!(A),
            discriminant: None,
            catch_all: None,
            content: Some(Struct {
                ident: parse_quote!(A),
                generics: Generics::default(),
                byte_order: None,
                len: Some(12),
                round: None,
                fields: vec![Field::Direct {
                    ident: parse_quote!(a),
                    ty: parse_quote!(u8),
                    layout_properties: FieldLayoutProperties { offset: Some(2), ..Default::default() },
                }],
            }),
        };
        assert_eq!(actual, expected);
    }
}
