use syn::{Expr, Ident, Type, parse_quote};

use crate::derive_struct::bit_field_attribute::BitFieldAttribute;
use crate::derive_struct::field_utils::{lower_de_with_layout, member_to_ident, member_to_string};
use crate::{ir_de, ir_se};

use super::field_utils::lower_se_with_layout;
use super::packed_field::PackedField;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitField {
    pub name: Ident,
    pub attribute: BitFieldAttribute,
    pub members: Vec<PackedField>,
}

impl BitField {
    pub fn lower_se(&self) -> ir_se::Expr {
        ir_se::chain_with_vars(
            vec![
                lower_se_bit_field(&self.attribute.repr, &parse_quote!(self), &self.members),
                lower_se_with_layout(
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

    pub fn lower_de(&self) -> Vec<ir_de::Let> {
        let name = &self.name;
        let bit_field = lower_de_with_layout(
            &self.name,
            &self.attribute.repr,
            Some(self.name.to_string().as_str()),
            self.attribute.offset,
            self.attribute.align,
            self.attribute.round,
        );
        let bit_field_from = ir_de::r#let(Some(name.clone()), ir_de::bit_field_from(ir_de::name(name.clone())));
        let unpacks = lower_de_members(&parse_quote!(#name), self.members.iter());
        bit_field.into_iter().chain(std::iter::once(bit_field_from)).chain(unpacks.into_iter()).collect()
    }
}

fn lower_se_bit_field(repr: &Type, parent: &Expr, members: &[PackedField]) -> ir_se::Expr {
    let members = members.iter().map(|member| {
        let name = &member.name;
        let name_str = member_to_string(&member.name);
        ir_se::enclose(
            ir_se::pack_object(parse_quote!(bit_field), parse_quote!(&#parent.#name), member.attribute.bits.clone()),
            name_str,
        )
    });

    ir_se::pack_bit_field(parse_quote!(bit_field), repr.clone(), members.collect())
}

fn lower_de_members<'a>(bit_field: &syn::Expr, members: impl Iterator<Item = &'a PackedField>) -> Vec<ir_de::Let> {
    members
        .map(|member| {
            ir_de::r#let(
                Some(member_to_ident(&member.name)),
                ir_de::r#try(ir_de::enclose(
                    ir_de::unpack_object(bit_field.clone(), member.ty.clone(), member.attribute.bits.clone()),
                    member_to_string(&member.name),
                )),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    use syn::parse_quote;

    use crate::derive_struct::{
        bit_field::BitField, bit_field_attribute::BitFieldAttribute, packed_field::PackedField,
        packed_field_attribute::PackedFieldAttribute,
    };

    fn make_empty() -> BitField {
        BitField {
            name: parse_quote!(bf),
            attribute: BitFieldAttribute { repr: parse_quote!(u16), ..Default::default() },
            members: vec![],
        }
    }

    fn make_one_member() -> BitField {
        BitField {
            name: parse_quote!(bf),
            attribute: BitFieldAttribute { repr: parse_quote!(u16), ..Default::default() },
            members: vec![PackedField {
                name: parse_quote!(foo),
                ty: parse_quote!(u8),
                attribute: PackedFieldAttribute { storage: parse_quote!(bf), bits: 4..7 },
            }],
        }
    }

    #[test]
    fn lower_se_empty() {
        let input = make_empty();
        let actual = input.lower_se();
        let expected = ir_se::chain_with_vars(
            vec![
                ir_se::pack_bit_field(parse_quote!(bit_field), parse_quote!(u16), vec![]),
                ir_se::enclose(ir_se::serialize_object(parse_quote!(&bit_field.into_bits())), "bf".into()),
            ],
            vec![Some(parse_quote!(bit_field))],
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn lower_de_one_member() {
        let input = make_one_member();
        let actual = input.lower_de();
        let expected = [
            ir_de::r#let(
                Some(parse_quote!(bf)),
                ir_de::r#try(ir_de::enclose(ir_de::deserialize_object(parse_quote!(u16)), "bf".into())),
            ),
            ir_de::r#let(Some(parse_quote!(bf)), ir_de::bit_field_from(ir_de::name(parse_quote!(bf)))),
            ir_de::r#let(
                Some(parse_quote!(foo)),
                ir_de::r#try(ir_de::unpack_object(parse_quote!(&bf), parse_quote!(u8), 4..7)),
            ),
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn lower_se_multiple() {
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
        let actual = lower_se_bit_field(&parse_quote!(u16), &parse_quote!(self), &members);
        let expected = ir_se::pack_bit_field(
            parse_quote!(bit_field),
            parse_quote!(u16),
            vec![
                ir_se::enclose(
                    ir_se::pack_object(parse_quote!(bit_field), parse_quote!(&self.foo), 4..7),
                    "foo".into(),
                ),
                ir_se::enclose(
                    ir_se::pack_object(parse_quote!(bit_field), parse_quote!(&self.bar), 9..10),
                    "bar".into(),
                ),
            ],
        );
        assert_eq!(actual, expected);
    }
}
