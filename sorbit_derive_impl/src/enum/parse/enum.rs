use std::collections::HashSet;

use syn::{DeriveInput, Generics, Ident, Type, spanned::Spanned as _};

use crate::attribute::{ByteOrder, as_byte_order, as_type, parse_nvp_attribute_group, parse_repr_attribute, path};
use crate::r#enum::parse::Variant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Enum {
    pub ident: Ident,
    pub storage_ty: Option<Type>,
    pub generics: Generics,
    pub byte_order: Option<ByteOrder>,
    pub variants: Vec<Variant>,
}

impl TryFrom<DeriveInput> for Enum {
    type Error = syn::Error;
    fn try_from(value: DeriveInput) -> Result<Self, Self::Error> {
        match value.data {
            syn::Data::Enum(data_enum) => {
                let sorbit_attrs = value.attrs.iter().filter(|attr| attr.path() == &path::sorbit_attribute());
                let parameters = parse_nvp_attribute_group(sorbit_attrs)?;

                let accepted_parameters: HashSet<_> = [path::byte_order(), path::storage_ty()].into_iter().collect();
                for (name, _) in &parameters {
                    if !accepted_parameters.contains(&name) {
                        return Err(syn::Error::new(name.span(), "unrecognized parameter"));
                    }
                }

                let repr = value
                    .attrs
                    .iter()
                    .find(|attr| attr.path().is_ident("repr"))
                    .map(parse_repr_attribute)
                    .transpose()?
                    .flatten();
                let byte_order = parameters.get(&path::byte_order()).map(|expr| as_byte_order(expr)).transpose()?;
                let storage_ty = parameters.get(&path::storage_ty()).map(|expr| as_type(expr)).transpose()?;
                let variants = data_enum
                    .variants
                    .into_iter()
                    .map(|field| Variant::try_from(field))
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(Self {
                    ident: value.ident,
                    storage_ty: storage_ty.or(repr),
                    generics: value.generics,
                    byte_order,
                    variants,
                })
            }
            syn::Data::Struct(_) => Err(syn::Error::new(value.span(), "expected an enum, got an struct")),
            syn::Data::Union(_) => Err(syn::Error::new(value.span(), "expected an enum, got a union")),
        }
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    #[test]
    fn repr_none() {
        let input: DeriveInput = parse_quote!(
            enum Enum {}
        );
        let actual = Enum::try_from(input).unwrap();
        let expected = Enum {
            ident: parse_quote!(Enum),
            storage_ty: None,
            generics: Generics::default(),
            byte_order: None,
            variants: vec![],
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn repr_c() {
        let input: DeriveInput = parse_quote!(
            #[repr(C)]
            enum Enum {}
        );
        let actual = Enum::try_from(input).unwrap();
        let expected = Enum {
            ident: parse_quote!(Enum),
            storage_ty: None,
            generics: Generics::default(),
            byte_order: None,
            variants: vec![],
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn repr_rust() {
        let input: DeriveInput = parse_quote!(
            #[repr(u8)]
            enum Enum {}
        );
        let actual = Enum::try_from(input).unwrap();
        let expected = Enum {
            ident: parse_quote!(Enum),
            storage_ty: Some(parse_quote!(u8)),
            generics: Generics::default(),
            byte_order: None,
            variants: vec![],
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn repr_sorbit() {
        let input: DeriveInput = parse_quote!(
            #[sorbit(repr=u8)]
            enum Enum {}
        );
        let actual = Enum::try_from(input).unwrap();
        let expected = Enum {
            ident: parse_quote!(Enum),
            storage_ty: Some(parse_quote!(u8)),
            generics: Generics::default(),
            byte_order: None,
            variants: vec![],
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn repr_both() {
        let input: DeriveInput = parse_quote!(
            #[repr(u16)]
            #[sorbit(repr=u8)]
            enum Enum {}
        );
        let actual = Enum::try_from(input).unwrap();
        let expected = Enum {
            ident: parse_quote!(Enum),
            storage_ty: Some(parse_quote!(u8)),
            generics: Generics::default(),
            byte_order: None,
            variants: vec![],
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn byte_order() {
        let input: DeriveInput = parse_quote!(
            #[sorbit(byte_order=big_endian)]
            enum Enum {}
        );
        let actual = Enum::try_from(input).unwrap();
        let expected = Enum {
            ident: parse_quote!(Enum),
            storage_ty: None,
            generics: Generics::default(),
            byte_order: Some(ByteOrder::BigEndian),
            variants: vec![],
        };
        assert_eq!(actual, expected);
    }

    #[test]
    #[should_panic]
    fn invalid_key() {
        let input: DeriveInput = parse_quote!(
            #[sorbit(invalid_key = 1)]
            enum Enum {}
        );
        let _ = Enum::try_from(input).unwrap();
    }

    #[test]
    #[should_panic]
    fn invalid_value() {
        let input: DeriveInput = parse_quote!(
            #[sorbit(byte_order = 1)]
            enum Enum {}
        );
        let _ = Enum::try_from(input).unwrap();
    }
}
