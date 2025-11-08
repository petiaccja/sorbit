use syn::{Expr, Index, Member, Type, parse_quote};

use crate::{derive_struct::direct_field_attribute::DirectFieldAttribute, hir};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectField {
    pub name: Member,
    pub ty: Type,
    pub attribute: DirectFieldAttribute,
}

impl DirectField {
    pub fn parse(field: &syn::Field, index: usize) -> Result<Self, syn::Error> {
        let attribute = DirectFieldAttribute::parse(field.attrs.iter())?;
        let name = match &field.ident {
            Some(ident) => Member::Named(ident.clone()),
            None => Member::Unnamed(Index::from(index)),
        };
        let ty = field.ty.clone();
        Ok(Self { name, ty, attribute })
    }

    pub fn to_hir(&self) -> hir::Expr {
        let member = &self.name;
        let name = match &self.name {
            Member::Named(ident) => ident.to_string(),
            Member::Unnamed(index) => index.index.to_string(),
        };
        derive_serialize_with_layout(
            &parse_quote!(&self.#member),
            Some(&name),
            self.attribute.offset,
            self.attribute.align,
            self.attribute.round,
        )
    }
}

pub fn derive_serialize_with_layout(
    value: &Expr,
    name: Option<&str>,
    offset: Option<u64>,
    align: Option<u64>,
    round: Option<u64>,
) -> hir::Expr {
    let serialized = hir::serialize_object(value.clone());

    let rounded = match round {
        Some(round) => hir::serialize_composite(vec![serialized, hir::align(round)]),
        None => serialized,
    };

    let aligned = match align {
        Some(align) => hir::chain(vec![hir::align(align), rounded]).flatten(),
        None => rounded,
    };

    let offseted = match offset {
        Some(offset) => hir::chain(vec![hir::pad(offset), aligned]).flatten(),
        None => aligned,
    };

    match name {
        Some(display_name) => hir::enclose(offseted, display_name.into()),
        None => offseted,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use syn::parse_quote;

    #[test]
    fn parse_trivial() -> Result<(), syn::Error> {
        let input: syn::Field = parse_quote! {
            foo: u8
        };
        let field = DirectField::parse(&input, 0)?;
        let expected =
            DirectField { name: parse_quote!(foo), ty: parse_quote!(u8), attribute: DirectFieldAttribute::default() };
        assert_eq!(field, expected);
        Ok(())
    }

    #[test]
    fn parse_layout() -> Result<(), syn::Error> {
        let input: syn::Field = parse_quote! {
            #[sorbit_layout(align=8)]
            foo: u8
        };
        let field = DirectField::parse(&input, 0)?;
        let expected = DirectField {
            name: parse_quote!(foo),
            ty: parse_quote!(u8),
            attribute: DirectFieldAttribute { offset: None, align: Some(8), round: None },
        };
        assert_eq!(field, expected);
        Ok(())
    }

    #[test]
    fn parse_bit_field_decl() {
        let input: syn::Field = parse_quote! {
            #[sorbit_bit_field(A, align=8)]
            foo: u8
        };
        assert!(DirectField::parse(&input, 0).is_err());
    }

    #[test]
    fn parse_bit_field_bits() {
        let input: syn::Field = parse_quote! {
            #[sorbit_bit_field(A, bits(4..8))]
            foo: u8
        };
        assert!(DirectField::parse(&input, 0).is_err());
    }

    #[test]
    fn to_hir() {
        let input =
            DirectField { name: parse_quote!(foo), ty: parse_quote!(i32), attribute: DirectFieldAttribute::default() };
        let expected = hir::enclose(hir::serialize_object(parse_quote!(&self.foo)), "foo".into());
        let actual = input.to_hir();
        assert_eq!(actual, expected);
    }

    #[test]
    fn derive_serialize_display_name() {
        let actual = derive_serialize_with_layout(&parse_quote!(foo), Some("foo"), None, None, None);
        let expected = hir::enclose(hir::serialize_object(parse_quote!(foo)), "foo".into());
        assert_eq!(actual, expected);
    }

    #[test]
    fn derive_serialize_offset_and_align() {
        let actual = derive_serialize_with_layout(&parse_quote!(foo), None, Some(4), Some(6), None);
        let expected = hir::chain(vec![
            hir::pad(4),
            hir::align(6),
            hir::serialize_object(parse_quote!(foo)),
        ]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn derive_serialize_round() {
        let actual = derive_serialize_with_layout(&parse_quote!(foo), None, None, None, Some(6));
        let expected = hir::serialize_composite(vec![hir::serialize_object(parse_quote!(foo)), hir::align(6)]);
        assert_eq!(actual, expected);
    }
}
