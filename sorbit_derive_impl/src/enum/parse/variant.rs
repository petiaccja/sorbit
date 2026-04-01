use proc_macro2::Span;
use syn::punctuated::Punctuated;
use syn::{Attribute, DeriveInput, Field, Fields, FieldsNamed, FieldsUnnamed, Generics, Member, Token};
use syn::{Expr, Ident, Type};

use crate::attribute::{as_literal_bool, parse_nvp_attribute_group, path};
use crate::r#struct::parse::Struct;
use crate::utility::check_invalid_parameters;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant {
    pub ident: Ident,
    pub discriminant: Option<Expr>,
    pub catch_all: CatchAll,
    pub content: Option<Struct>,
}

impl TryFrom<syn::Variant> for Variant {
    type Error = syn::Error;
    fn try_from(value: syn::Variant) -> Result<Self, Self::Error> {
        let sorbit_attrs = value.attrs.iter().filter(|attr| attr.path() == &path::sorbit_attribute());
        let parameters = parse_nvp_attribute_group(sorbit_attrs)?;

        let accepted_parameters = [
            path::catch_all(),
            path::byte_order(),
            path::len(),
            path::round(),
        ];
        check_invalid_parameters(&parameters, accepted_parameters.iter())?;

        let discriminant = value.discriminant.map(|(_, expr)| expr);
        let catch_all_tag =
            parameters.get(&path::catch_all()).map(|expr| as_literal_bool(expr)).transpose()?.unwrap_or(false);
        let (catch_all, content) = if !catch_all_tag {
            parse_regular(value.ident.clone(), value.attrs, value.fields)?
        } else {
            parse_catch_all(value.ident.clone(), value.attrs, value.fields)?
        };

        Ok(Self { ident: value.ident, discriminant, catch_all, content })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatchAll {
    None,
    Blanket,
    Discriminant(Member, Type),
}

fn parse_catch_all(
    ident: Ident,
    attrs: Vec<Attribute>,
    fields: Fields,
) -> Result<(CatchAll, Option<Struct>), syn::Error> {
    let (discr_field, rest) = pop_first_field(fields);
    let catch_all = discr_field
        .map(|field| {
            CatchAll::Discriminant(field.ident.map(|ident| Member::from(ident)).unwrap_or(Member::from(0)), field.ty)
        })
        .unwrap_or(CatchAll::Blanket);
    let content = parse_content(ident, attrs, rest)?;
    Ok((catch_all, content))
}

fn parse_regular(
    ident: Ident,
    attrs: Vec<Attribute>,
    fields: Fields,
) -> Result<(CatchAll, Option<Struct>), syn::Error> {
    Ok((CatchAll::None, parse_content(ident, attrs, fields)?))
}

fn parse_content(ident: Ident, attrs: Vec<Attribute>, fields: Fields) -> Result<Option<Struct>, syn::Error> {
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

fn pop_first_field(fields: Fields) -> (Option<Field>, Fields) {
    match fields {
        Fields::Named(FieldsNamed { brace_token, named }) => {
            let (first, named) = pop_first_puncuated(named);
            (first, Fields::Named(FieldsNamed { brace_token, named }))
        }
        Fields::Unnamed(FieldsUnnamed { paren_token, unnamed }) => {
            let (first, unnamed) = pop_first_puncuated(unnamed);
            (first, Fields::Unnamed(FieldsUnnamed { paren_token, unnamed }))
        }
        Fields::Unit => (None, Fields::Unit),
    }
}

fn pop_first_puncuated<T, P: Default>(punctuated: Punctuated<T, P>) -> (Option<T>, Punctuated<T, P>) {
    let mut it = punctuated.into_iter();
    let first = it.next();
    let rest = it.collect();
    (first, rest)
}

#[cfg(test)]
mod tests {
    use crate::{
        attribute::{ByteOrder, Transform},
        r#struct::parse::{Field, FieldLayoutProperties},
    };

    use super::*;

    use syn::parse_quote;

    #[test]
    fn simple() {
        let input: syn::Variant = parse_quote!(A);
        let actual = Variant::try_from(input).unwrap();
        let expected = Variant { ident: parse_quote!(A), discriminant: None, catch_all: CatchAll::None, content: None };
        assert_eq!(actual, expected);
    }

    #[test]
    fn catch_all_empty() {
        let input: syn::Variant = parse_quote!(
            #[sorbit(catch_all)]
            A
        );
        let actual = Variant::try_from(input).unwrap();
        let expected =
            Variant { ident: parse_quote!(A), discriminant: None, catch_all: CatchAll::Blanket, content: None };
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
            catch_all: CatchAll::Discriminant(Member::from(0), parse_quote!(u8)),
            content: None,
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn catch_all_struct() {
        let input: syn::Variant = parse_quote!(
            #[sorbit(catch_all)]
            A { a: u8 }
        );
        let actual = Variant::try_from(input).unwrap();
        let expected = Variant {
            ident: parse_quote!(A),
            discriminant: None,
            catch_all: CatchAll::Discriminant(parse_quote!(a), parse_quote!(u8)),
            content: None,
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn catch_all_content_tuple() {
        let input: syn::Variant = parse_quote!(
            #[sorbit(catch_all)]
            A(u8, u16)
        );
        let actual = Variant::try_from(input).unwrap();
        let expected = Variant {
            ident: parse_quote!(A),
            discriminant: None,
            catch_all: CatchAll::Discriminant(parse_quote!(0), parse_quote!(u8)),
            content: Some(Struct {
                ident: parse_quote!(A),
                generics: Generics::default(),
                byte_order: None,
                len: None,
                round: None,
                fields: vec![Field::Direct {
                    ident: None,
                    ty: parse_quote!(u16),
                    multi_pass: None,
                    transform: Transform::None,
                    layout_properties: Default::default(),
                }],
            }),
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn catch_all_content_struct() {
        let input: syn::Variant = parse_quote!(
            #[sorbit(catch_all)]
            #[sorbit(byte_order = big_endian)]
            A {
                ca: u8,
                #[sorbit(multi_pass)]
                field: u16,
            }
        );
        let actual = Variant::try_from(input).unwrap();
        let expected = Variant {
            ident: parse_quote!(A),
            discriminant: None,
            catch_all: CatchAll::Discriminant(parse_quote!(ca), parse_quote!(u8)),
            content: Some(Struct {
                ident: parse_quote!(A),
                generics: Generics::default(),
                byte_order: Some(ByteOrder::BigEndian),
                len: None,
                round: None,
                fields: vec![Field::Direct {
                    ident: Some(parse_quote!(field)),
                    ty: parse_quote!(u16),
                    multi_pass: Some(true),
                    transform: Transform::None,
                    layout_properties: Default::default(),
                }],
            }),
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn discriminant() {
        let input: syn::Variant = parse_quote!(A = 34);
        let actual = Variant::try_from(input).unwrap();
        let expected = Variant {
            ident: parse_quote!(A),
            discriminant: Some(parse_quote!(34)),
            catch_all: CatchAll::None,
            content: None,
        };
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
            catch_all: CatchAll::None,
            content: Some(Struct {
                ident: parse_quote!(A),
                generics: Generics::default(),
                byte_order: None,
                len: Some(12),
                round: None,
                fields: vec![Field::Direct {
                    ident: parse_quote!(a),
                    ty: parse_quote!(u8),
                    multi_pass: None,
                    transform: Transform::None,
                    layout_properties: FieldLayoutProperties { offset: Some(2), ..Default::default() },
                }],
            }),
        };
        assert_eq!(actual, expected);
    }
}
