use proc_macro2::TokenStream;
use quote::quote;
use syn::{Expr, Ident, Type, parse_quote};

use crate::derive_struct::{
    bit_field_attribute::BitFieldAttribute, direct_field::derive_serialize_with_layout, packed_field::PackedField,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitField {
    pub name: Ident,
    pub attribute: BitFieldAttribute,
    pub members: Vec<PackedField>,
}

impl BitField {
    pub fn derive_serialize(&self, parent: &Expr, serializer: &Expr, serializer_ty: &Type) -> TokenStream {
        if self.members.is_empty() {
            quote! {}
        } else {
            let display_name = self.name.to_string();
            let bit_field = derive_serialize_bit_field(&self.attribute.repr, serializer_ty, parent, &self.members);
            let serialization = derive_serialize_with_layout(
                &parse_quote!(bit_field.into_bits()),
                serializer,
                Some(&display_name),
                self.attribute.offset,
                self.attribute.align,
                self.attribute.round,
            );

            quote! {
                #bit_field.and_then(|bit_field| #serialization)
            }
        }
    }
}

fn derive_serialize_bit_field(
    repr: &Type,
    serializer_ty: &Type,
    parent: &Expr,
    members: &[PackedField],
) -> TokenStream {
    let members = members.iter().map(|member| {
        let name = &member.name;
        let name_str = match &member.name {
            syn::Member::Named(ident) => ident.to_string(),
            syn::Member::Unnamed(index) => index.index.to_string(),
        };
        let start = member.attribute.bits.start;
        let end = member.attribute.bits.end;
        quote! {
            bit_field.pack(&#parent.#name, #start..#end)
                        .map_err(|_| <<#serializer_ty as ::sorbit::serialize::SerializerOutput>::Error as ::sorbit::error::SerializeError>::invalid_bit_field())
                        .map_err(|err| ::sorbit::error::SerializeError::enclose(err, #name_str))
        }
    });

    quote! {
        {
            let mut bit_field = ::sorbit::bit::BitField::<#repr>::new();
            let results = [
                #(#members,)*
            ];
            results.into_iter().fold(Ok(()), |acc, result| acc.and(result)).map(|_| bit_field)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use quote::quote;
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
        let output = derive_serialize_bit_field(&parse_quote!(u16), &parse_quote!(S), &parse_quote!(self), &members);
        let expected = quote! {
            {
                let mut bit_field = ::sorbit::bit::BitField::<u16>::new();
                let results = [
                    bit_field.pack(&self.foo, 4u8..7u8)
                            .map_err(|_| <<S as ::sorbit::serialize::SerializerOutput>::Error as ::sorbit::error::SerializeError>::invalid_bit_field())
                            .map_err(|err| ::sorbit::error::SerializeError::enclose(err, "foo")),
                    bit_field.pack(&self.bar, 9u8..10u8)
                            .map_err(|_| <<S as ::sorbit::serialize::SerializerOutput>::Error as ::sorbit::error::SerializeError>::invalid_bit_field())
                            .map_err(|err| ::sorbit::error::SerializeError::enclose(err, "bar")),
                ];
                results.into_iter().fold(Ok(()), |acc, result| acc.and(result)).map(|_| bit_field)
            }
        };
        assert_eq!(expected.to_string(), output.to_string());
    }

    #[test]
    fn derive_serialize_empty() {
        let input = BitField { name: parse_quote!(bf), attribute: BitFieldAttribute::default(), members: vec![] };
        let output = input.derive_serialize(&parse_quote!(self), &parse_quote!(serializer), &parse_quote!(S));
        let expected = quote! {};
        assert_eq!(output.to_string(), expected.to_string());
    }

    #[test]
    fn derive_serialize_no_parameters() {
        let input = BitField {
            name: parse_quote!(_bf),
            attribute: BitFieldAttribute { repr: parse_quote!(u16), offset: None, align: None, round: None },
            members: vec![PackedField {
                name: parse_quote!(foo),
                ty: parse_quote!(i8),
                attribute: PackedFieldAttribute { storage: parse_quote!(_bf), bits: 4..7 },
            }],
        };
        let output = input.derive_serialize(&parse_quote!(self), &parse_quote!(serializer), &parse_quote!(S));
        let expected = quote! {
            {
                let mut bit_field = ::sorbit::bit::BitField::<u16>::new();
                let results = [
                    bit_field.pack(&self.foo, 4u8..7u8)
                            .map_err(|_| <<S as ::sorbit::serialize::SerializerOutput>::Error as ::sorbit::error::SerializeError>::invalid_bit_field())
                            .map_err(|err| ::sorbit::error::SerializeError::enclose(err, "foo")),
                ];
                results.into_iter().fold(Ok(()), |acc, result| acc.and(result)).map(|_| bit_field)
            }
            .and_then(|bit_field|
                ::sorbit::serialize::Serialize::serialize(&bit_field.into_bits(), serializer)
                    .map_err(|err| ::sorbit::error::SerializeError::enclose(err, "_bf"))
            )
        };
        assert_eq!(output.to_string(), expected.to_string());
    }
}
