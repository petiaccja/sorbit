use std::{
    collections::{HashMap, HashSet},
    ops::Range,
};
use syn::{Expr, Ident, Path, Type, spanned::Spanned};

use crate::attribute::{as_ident, as_literal_int, as_literal_int_range, as_type, parse_nvp_attribute_group, path};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Field {
    Direct {
        ident: Option<Ident>,
        ty: Type,
        offset: Option<u64>,
        align: Option<u64>,
        round: Option<u64>,
    },
    Bit {
        ident: Option<Ident>,
        ty: Type,
        storage_id: Ident,
        storage_ty: Option<Type>,
        bits: Range<u8>,
        offset: Option<u64>,
        align: Option<u64>,
        round: Option<u64>,
    },
}

impl TryFrom<syn::Field> for Field {
    type Error = syn::Error;
    fn try_from(value: syn::Field) -> Result<Self, Self::Error> {
        let sorbit_attrs = value.attrs.iter().filter(|attr| attr.path() == &path::sorbit_attribute());
        let parameters = parse_nvp_attribute_group(sorbit_attrs)?;
        let is_bit_field = parameters.contains_key(&path::storage_id()) || parameters.contains_key(&path::bit_range());

        if !is_bit_field {
            Self::parse_direct_field(value.ident, value.ty, parameters)
        } else {
            Self::parse_bit_field(value.ident, value.ty, parameters)
        }
    }
}

impl Field {
    fn parse_direct_field(
        ident: Option<Ident>,
        ty: Type,
        parameters: HashMap<Path, Expr>,
    ) -> Result<Field, syn::Error> {
        let accepted_parameters: HashSet<_> = [path::offset(), path::align(), path::round()].into_iter().collect();
        for (name, _) in &parameters {
            if !accepted_parameters.contains(&name) {
                return Err(syn::Error::new(name.span(), "invalid parameter"));
            }
        }

        let offset = parameters.get(&path::offset()).map(|expr| as_literal_int(expr)).transpose()?;
        let align = parameters.get(&path::align()).map(|expr| as_literal_int(expr)).transpose()?;
        let round = parameters.get(&path::round()).map(|expr| as_literal_int(expr)).transpose()?;
        Ok(Self::Direct { ident, ty, offset, align, round })
    }

    fn parse_bit_field(ident: Option<Ident>, ty: Type, parameters: HashMap<Path, Expr>) -> Result<Field, syn::Error> {
        const MISSING_STORAGE_ID: &str =
            "this bit field is missing the storage identifier, add `bit_field=<IDENTIFIER>` to the attribute";
        const MISSING_BITS: &str =
            "this bit field is missing the bit range, add `bits=<S>..<E>` or `bits=<B>` to the attribute";

        let accepted_parameters: HashSet<_> = [
            path::storage_id(),
            path::storage_ty(),
            path::bit_range(),
            path::offset(),
            path::align(),
            path::round(),
        ]
        .into_iter()
        .collect();
        for (name, _) in &parameters {
            if !accepted_parameters.contains(&name) {
                return Err(syn::Error::new(name.span(), "invalid parameter"));
            }
        }

        let storage_id = parameters
            .get(&path::storage_id())
            .map(|expr| as_ident(expr))
            .ok_or(syn::Error::new(ident.span(), MISSING_STORAGE_ID))??;
        let storage_ty = parameters.get(&path::storage_ty()).map(|expr| as_type(expr)).transpose()?;
        let bits = parameters
            .get(&path::bit_range())
            .map(|expr| {
                as_literal_int_range(expr)
                    .or(as_literal_int(expr).map(|bit| bit..(bit + 1)))
                    .map_err(|err| syn::Error::new(err.span(), "expected either a literal range or an integer literal"))
            })
            .ok_or(syn::Error::new(ident.span(), MISSING_BITS))??;
        let offset = parameters.get(&path::offset()).map(|expr| as_literal_int(expr)).transpose()?;
        let align = parameters.get(&path::align()).map(|expr| as_literal_int(expr)).transpose()?;
        let round = parameters.get(&path::round()).map(|expr| as_literal_int(expr)).transpose()?;

        Ok(Self::Bit { ident, ty, storage_id, storage_ty, bits, offset, align, round })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use syn::parse_quote;

    #[test]
    fn direct_default() {
        let input: syn::Field = parse_quote! {
            field: u8
        };
        let actual = Field::try_from(input);
        let expected =
            Field::Direct { ident: parse_quote!(field), ty: parse_quote!(u8), offset: None, align: None, round: None };
        assert_eq!(actual.unwrap(), expected);
    }

    #[test]
    fn direct_foreign_attribute() {
        let input: syn::Field = parse_quote! {
            #[derive(Debug)]
            field: u8
        };
        let actual = Field::try_from(input);
        let expected =
            Field::Direct { ident: parse_quote!(field), ty: parse_quote!(u8), offset: None, align: None, round: None };
        assert_eq!(actual.unwrap(), expected);
    }

    #[test]
    fn direct_with_layout_merged() {
        let input: syn::Field = parse_quote! {
            #[sorbit(offset=1, align=2, round=3)]
            field: u8
        };
        let actual = Field::try_from(input);
        let expected = Field::Direct {
            ident: parse_quote!(field),
            ty: parse_quote!(u8),
            offset: Some(1),
            align: Some(2),
            round: Some(3),
        };
        assert_eq!(actual.unwrap(), expected);
    }

    #[test]
    fn direct_with_layout_split() {
        let input: syn::Field = parse_quote! {
            #[sorbit(offset=1)]
            #[sorbit(align=2)]
            #[sorbit(round=3)]
            field: u8
        };
        let actual = Field::try_from(input);
        let expected = Field::Direct {
            ident: parse_quote!(field),
            ty: parse_quote!(u8),
            offset: Some(1),
            align: Some(2),
            round: Some(3),
        };
        assert_eq!(actual.unwrap(), expected);
    }

    #[test]
    #[should_panic]
    fn direct_with_layout_redefined() {
        let input: syn::Field = parse_quote! {
            #[sorbit(align=2)]
            #[sorbit(align=3)]
            field: u8
        };
        Field::try_from(input).unwrap();
    }

    #[test]
    #[should_panic]
    fn direct_invalid_meta_key() {
        let input: syn::Field = parse_quote! {
            #[sorbit(align=2, invalid_key=4)]
            field: u8
        };
        Field::try_from(input).unwrap();
    }

    #[test]
    #[should_panic]
    fn direct_invalid_meta_value() {
        let input: syn::Field = parse_quote! {
            #[sorbit(align=invalid_value)]
            field: u8
        };
        Field::try_from(input).unwrap();
    }

    #[test]
    fn bit_default_merged() {
        let input: syn::Field = parse_quote! {
            #[sorbit(bit_field=_bit_field, bits=1..3)]
            field: u8
        };
        let actual = Field::try_from(input);
        let expected = Field::Bit {
            ident: parse_quote!(field),
            ty: parse_quote!(u8),
            storage_id: parse_quote!(_bit_field),
            storage_ty: None,
            bits: 1..3,
            offset: None,
            align: None,
            round: None,
        };
        assert_eq!(actual.unwrap(), expected);
    }

    #[test]
    fn bit_default_split() {
        let input: syn::Field = parse_quote! {
            #[sorbit(bit_field=_bit_field)]
            #[sorbit(bits=1..3)]
            field: u8
        };
        let actual = Field::try_from(input);
        let expected = Field::Bit {
            ident: parse_quote!(field),
            ty: parse_quote!(u8),
            storage_id: parse_quote!(_bit_field),
            storage_ty: None,
            bits: 1..3,
            offset: None,
            align: None,
            round: None,
        };
        assert_eq!(actual.unwrap(), expected);
    }

    #[test]
    fn bit_foreign_attribute() {
        let input: syn::Field = parse_quote! {
            #[derive(Debug)]
            #[sorbit(bit_field=_bit_field, bits=1..3)]
            field: u8
        };
        let actual = Field::try_from(input);
        let expected = Field::Bit {
            ident: parse_quote!(field),
            ty: parse_quote!(u8),
            storage_id: parse_quote!(_bit_field),
            storage_ty: None,
            bits: 1..3,
            offset: None,
            align: None,
            round: None,
        };
        assert_eq!(actual.unwrap(), expected);
    }

    #[test]
    fn bit_with_layout_merged() {
        let input: syn::Field = parse_quote! {
            #[sorbit(bit_field=_bit_field, bits=1..3, offset=1, align=2, round=3)]
            field: u8
        };
        let actual = Field::try_from(input);
        let expected = Field::Bit {
            ident: parse_quote!(field),
            ty: parse_quote!(u8),
            storage_id: parse_quote!(_bit_field),
            storage_ty: None,
            bits: 1..3,
            offset: Some(1),
            align: Some(2),
            round: Some(3),
        };
        assert_eq!(actual.unwrap(), expected);
    }

    #[test]
    fn bit_with_layout_split() {
        let input: syn::Field = parse_quote! {
            #[sorbit(bit_field=_bit_field, bits=1..3)]
            #[sorbit(offset=1)]
            #[sorbit(align=2)]
            #[sorbit(round=3)]
            field: u8
        };
        let actual = Field::try_from(input);
        let expected = Field::Bit {
            ident: parse_quote!(field),
            ty: parse_quote!(u8),
            storage_id: parse_quote!(_bit_field),
            storage_ty: None,
            bits: 1..3,
            offset: Some(1),
            align: Some(2),
            round: Some(3),
        };
        assert_eq!(actual.unwrap(), expected);
    }

    #[test]
    #[should_panic]
    fn bit_bits_redefined() {
        let input: syn::Field = parse_quote! {
            #[sorbit(bit_field=_bit_field, bits=1..3)]
            #[sorbit(bits=1..4)]
            field: u8
        };
        Field::try_from(input).unwrap();
    }

    #[test]
    #[should_panic]
    fn bit_name_missing() {
        let input: syn::Field = parse_quote! {
            #[sorbit(bits=1..3)]
            field: u8
        };
        Field::try_from(input).unwrap();
    }

    #[test]
    #[should_panic]
    fn bit_bits_missing() {
        let input: syn::Field = parse_quote! {
            #[sorbit(bit_field=_bit_field)]
            field: u8
        };
        Field::try_from(input).unwrap();
    }

    #[test]
    #[should_panic]
    fn bit_with_layout_redefined() {
        let input: syn::Field = parse_quote! {
            #[sorbit(bit_field=_bit_field, bits=1..3, offset=1)]
            #[sorbit(offset=2)]
            field: u8
        };
        Field::try_from(input).unwrap();
    }

    #[test]
    #[should_panic]
    fn bit_invalid_meta_key() {
        let input: syn::Field = parse_quote! {
            #[sorbit(bit_field=_bit_field, bits=1..3, invalid_key=1)]
            field: u8
        };
        Field::try_from(input).unwrap();
    }

    #[test]
    #[should_panic]
    fn bit_invalid_meta_value() {
        let input: syn::Field = parse_quote! {
            #[sorbit(bit_field=_bit_field, bits=1..3, offset=invalid_value)]
            field: u8
        };
        Field::try_from(input).unwrap();
    }
}
