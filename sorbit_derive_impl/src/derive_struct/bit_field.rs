use syn::{Expr, Ident, Type, parse_quote};

use crate::{
    derive_struct::{
        bit_field_attribute::BitFieldAttribute, direct_field::derive_serialize_with_layout, packed_field::PackedField,
    },
    hir,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitField {
    pub name: Ident,
    pub attribute: BitFieldAttribute,
    pub members: Vec<PackedField>,
}

impl BitField {
    pub fn to_hir(&self) -> hir::ast::Expr {
        hir::chain_with_vars(
            vec![
                derive_serialize_bit_field(&self.attribute.repr, &parse_quote!(self), &self.members),
                derive_serialize_with_layout(
                    &parse_quote!(&bit_field.into_bits()),
                    Some(&self.name.to_string()),
                    self.attribute.offset,
                    self.attribute.align,
                    self.attribute.round,
                ),
            ],
            vec![Some(parse_quote!(bit_field))],
        )
    }
}

fn derive_serialize_bit_field(repr: &Type, parent: &Expr, members: &[PackedField]) -> hir::ast::Expr {
    let members = members.iter().map(|member| {
        let name = &member.name;
        let name_str = match &member.name {
            syn::Member::Named(ident) => ident.to_string(),
            syn::Member::Unnamed(index) => index.index.to_string(),
        };
        hir::enclose(
            hir::pack_object(parse_quote!(bit_field), parse_quote!(&#parent.#name), member.attribute.bits.clone()),
            name_str,
        )
    });

    hir::pack_bit_field(parse_quote!(bit_field), repr.clone(), members.collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    use syn::parse_quote;

    use crate::derive_struct::{
        bit_field::BitField, bit_field_attribute::BitFieldAttribute, packed_field::PackedField,
        packed_field_attribute::PackedFieldAttribute,
    };

    #[test]
    fn derive_serialize_bit_field_multiple() {
        let members = [
            PackedField {
                name: parse_quote!(foo),
                ty: parse_quote!(u8),
                attribute: PackedFieldAttribute { storage: parse_quote!(_bf), bits: 4..7 },
            },
            PackedField {
                name: parse_quote!(bar),
                ty: parse_quote!(bool),
                attribute: PackedFieldAttribute { storage: parse_quote!(_bf), bits: 9..10 },
            },
        ];
        let actual = derive_serialize_bit_field(&parse_quote!(u16), &parse_quote!(self), &members);
        let expected = hir::pack_bit_field(
            parse_quote!(bit_field),
            parse_quote!(u16),
            vec![
                hir::enclose(hir::pack_object(parse_quote!(bit_field), parse_quote!(&self.foo), 4..7), "foo".into()),
                hir::enclose(hir::pack_object(parse_quote!(bit_field), parse_quote!(&self.bar), 9..10), "bar".into()),
            ],
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn to_hir_empty() {
        let input = BitField {
            name: parse_quote!(bf),
            attribute: BitFieldAttribute { repr: parse_quote!(u16), ..Default::default() },
            members: vec![],
        };
        let actual = input.to_hir();
        let expected = hir::chain_with_vars(
            vec![
                hir::pack_bit_field(parse_quote!(bit_field), parse_quote!(u16), vec![]),
                hir::enclose(hir::serialize_object(parse_quote!(&bit_field.into_bits())), "bf".into()),
            ],
            vec![Some(parse_quote!(bit_field))],
        );
        assert_eq!(actual, expected);
    }
}
