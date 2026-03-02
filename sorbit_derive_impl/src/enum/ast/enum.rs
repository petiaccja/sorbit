use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::spanned::Spanned as _;
use syn::{BinOp, Expr, ExprBinary, ExprLit, Generics, Ident, Lit, LitInt, Token, Type, parse_quote};

use crate::attribute::ByteOrder;
use crate::r#enum::ast::variant::Variant;
use crate::r#enum::parse;
use crate::ir::algorithm::with_maybe_byte_order;
use crate::ir::dag::{Region, ToDeserializeOp, ToSerializeOp, Value};
use crate::ir::ops::{
    custom_expr, deserialize_object, error, impl_deserialize, impl_serialize, match_, ok, ref_, self_,
    serialize_object, struct_, try_, yield_,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Enum {
    pub ident: Ident,
    pub storage_ty: Type,
    pub generics: Generics,
    pub byte_order: Option<ByteOrder>,
    pub catch_all: Option<(Ident, bool)>,
    pub variants: Vec<Variant>,
}

impl Enum {
    fn normal_variants(&self) -> impl Iterator<Item = &Variant> {
        self.variants.iter().filter(|variant| {
            self.catch_all.as_ref().is_some_and(|(ident, _)| ident != &variant.ident) || self.catch_all.is_none()
        })
    }

    fn catch_all_variant(&self) -> Option<(&Variant, bool)> {
        self.variants
            .iter()
            .find(|variant| self.catch_all.as_ref().is_some_and(|(ident, _)| ident == &variant.ident))
            .map(|variant| (variant, self.catch_all.as_ref().expect("find ensures Some").1))
    }
}

impl TryFrom<parse::Enum> for Enum {
    type Error = syn::Error;
    fn try_from(mut value: parse::Enum) -> Result<Self, Self::Error> {
        let storage_ty = value.storage_ty.unwrap_or(parse_quote!(isize));
        let mut catch_all_it = value
            .variants
            .iter_mut()
            .filter_map(|variant| variant.catch_all.take().map(|ty| (variant.ident.clone(), ty)));
        let catch_all = catch_all_it.next();
        if let Some((_ident, ty)) = &catch_all {
            if let Some((ident, _)) = catch_all_it.next() {
                return Err(syn::Error::new(ident.span(), "there may be at most one catch all variant"));
            }
            if let Some(ty) = ty {
                if ty != &storage_ty {
                    return Err(syn::Error::new(
                        ty.span(),
                        format!(
                            "catch all type ({}) differs from the enum's repr ({})",
                            ty.to_token_stream(),
                            storage_ty.to_token_stream()
                        ),
                    ));
                }
            }
        }

        let discriminants = compute_discriminants(value.variants.iter_mut().map(|variant| variant.discriminant.take()));
        let variants = std::iter::zip(value.variants.into_iter(), discriminants.into_iter())
            .map(|(variant, discriminant)| Variant { ident: variant.ident, discriminant })
            .collect();
        Ok(Self {
            ident: value.ident,
            storage_ty,
            generics: value.generics,
            byte_order: value.byte_order,
            catch_all: catch_all.map(|(ident, ty)| (ident, ty.is_some())),
            variants,
        })
    }
}

impl ToSerializeOp for Enum {
    type Args = ();
    fn to_serialize_op(&self, region: &mut Region, _: Self::Args) -> Vec<Value> {
        impl_serialize(region, self.ident.clone(), self.generics.clone(), |region, serializer| {
            let storage_ty = &self.storage_ty;

            let result = with_maybe_byte_order(region, serializer, self.byte_order, true, |region, serializer| {
                let self_ = self_(region);
                let arms = self
                    .normal_variants()
                    .map(|variant| {
                        let self_ident = &self.ident;
                        let variant_ident = &variant.ident;
                        let pat = parse_quote!(#self_ident::#variant_ident);
                        let discriminant_expr = variant.discriminant.clone();
                        let discriminant_cast = parse_quote!( (#discriminant_expr) as #storage_ty );
                        let body = move |region: &mut Region| {
                            let discriminant = custom_expr(region, discriminant_cast);
                            let disc_ref = ref_(region, discriminant);
                            serialize_object(region, serializer, disc_ref)
                        };
                        (pat, None, Box::new(body) as Box<_>)
                    })
                    .chain(self.catch_all_variant().map(|(variant, store_disc)| {
                        let self_ident = &self.ident;
                        let variant_ident = &variant.ident;
                        if store_disc {
                            let pat = parse_quote!(#self_ident::#variant_ident(value));
                            let value_expr = parse_quote!(value);
                            let body = move |region: &mut Region| {
                                let discriminant = custom_expr(region, value_expr);
                                let disc_ref = ref_(region, discriminant);
                                serialize_object(region, serializer, disc_ref)
                            };
                            (pat, None, Box::new(body) as Box<_>)
                        } else {
                            let pat = parse_quote!(#self_ident::#variant_ident);
                            let discriminant_expr = variant.discriminant.clone();
                            let discriminant_cast = parse_quote!( (#discriminant_expr) as #storage_ty );
                            let body = move |region: &mut Region| {
                                let discriminant = custom_expr(region, discriminant_cast);
                                let disc_ref = ref_(region, discriminant);
                                serialize_object(region, serializer, disc_ref)
                            };
                            (pat, None, Box::new(body) as Box<_>)
                        }
                    }));
                match_(region, self_, arms)
            });
            let _ = yield_(region, vec![result]);
        });
        vec![]
    }
}

impl ToDeserializeOp for Enum {
    type Args = ();
    fn to_deserialize_op(&self, region: &mut Region, _: Self::Args) -> Vec<Value> {
        impl_deserialize(region, self.ident.clone(), self.generics.clone(), |region, deserializer| {
            let result = with_maybe_byte_order(region, deserializer, self.byte_order, false, |region, deserializer| {
                let maybe_discriminant = deserialize_object(region, deserializer, self.storage_ty.clone());
                let discriminant = try_(region, maybe_discriminant);
                let unmatched = self
                    .catch_all_variant()
                    .map(|(variant, store_disc)| {
                        let self_ident = &self.ident;
                        let variant_ident = &variant.ident;
                        let pat = parse_quote!(_);
                        if store_disc {
                            let struct_ty = parse_quote!(#self_ident::#variant_ident);
                            let body = move |region: &mut Region| {
                                let value = struct_(region, struct_ty, vec![(syn::Member::from(0), discriminant)]);
                                ok(region, value)
                            };
                            (pat, None, Box::new(body) as Box<_>)
                        } else {
                            let struct_ty = parse_quote!(#self_ident::#variant_ident);
                            let body = move |region: &mut Region| {
                                let value = struct_(region, struct_ty, vec![]);
                                ok(region, value)
                            };
                            (pat, None, Box::new(body) as Box<_>)
                        }
                    })
                    .unwrap_or_else(|| {
                        let pat = parse_quote!(_);
                        let body =
                            move |region: &mut Region| error(region, deserializer, "invalid enum discriminant".into());
                        (pat, None, Box::new(body) as Box<_>)
                    });
                let arms = self
                    .normal_variants()
                    .map(|variant| {
                        let self_ident = &self.ident;
                        let variant_ident = &variant.ident;
                        let pat = parse_quote!(discriminant);
                        let discriminant_expr = &variant.discriminant;
                        let guard_expr = parse_quote!(discriminant == #discriminant_expr);
                        let struct_ty = parse_quote!(#self_ident::#variant_ident);
                        let body = move |region: &mut Region| {
                            let value = struct_(region, struct_ty, vec![]);
                            ok(region, value)
                        };
                        (pat, Some(guard_expr), Box::new(body) as Box<_>)
                    })
                    .chain(std::iter::once(unmatched));
                match_(region, discriminant, arms)
            });
            let _ = yield_(region, vec![result]);
        });
        vec![]
    }
}

impl Enum {
    pub fn to_pack_into_tokens(&self) -> TokenStream {
        let ident = &self.ident;
        let storage_ty = &self.storage_ty;

        let exact_arms = self.normal_variants().map(|variant| {
            let ident = &variant.ident;
            let discr_expr = &variant.discriminant;
            quote! { Self::#ident => { ((#discr_expr) as #storage_ty).pack_into(num_bits) } }
        });
        let catch_all_arm = self.catch_all_variant().map(|(variant, store_disc)| {
            let ident = &variant.ident;
            let discr_expr = &variant.discriminant;
            if store_disc {
                quote! { Self::#ident(value) => { value.pack_into(num_bits) } }
            } else {
                quote! { Self::#ident => { ((#discr_expr) as #storage_ty).pack_into(num_bits) } }
            }
        });
        let arms = exact_arms.chain(catch_all_arm);

        quote! {
            impl<Packed> ::sorbit::bit::PackInto<Packed> for #ident
            where
                #storage_ty: ::sorbit::bit::PackInto<Packed>,
            {
                fn pack_into(&self, num_bits: usize) -> ::core::option::Option<Packed> {
                    match self {
                        #(#arms)*
                    }
                }
            }
        }
    }

    pub fn to_unpack_from_tokens(&self) -> TokenStream {
        let ident = &self.ident;
        let storage_ty = &self.storage_ty;

        let exact_arms = self.normal_variants().map(|variant| {
            let ident = &variant.ident;
            let discr_expr = &variant.discriminant;
            quote! { n if n == (#discr_expr) as u8 => { ::core::result::Result::Ok(Self::#ident) } }
        });
        let catch_all_arm = self
            .catch_all_variant()
            .map(|(variant, store_disc)| {
                let ident = &variant.ident;
                if store_disc {
                    quote! { n => { ::core::result::Result::Ok(Self::#ident(n)) } }
                } else {
                    quote! { _ => { ::core::result::Result::Ok(Self::#ident) } }
                }
            })
            .unwrap_or_else(|| {
                quote! { _ => { Err(value) } }
            });
        let arms = exact_arms.chain(std::iter::once(catch_all_arm));

        quote! {
            impl<Packed> ::sorbit::bit::UnpackFrom<Packed> for #ident
            where
                #storage_ty: ::sorbit::bit::UnpackFrom<Packed>,
                Packed: Clone,
            {
                fn unpack_from(value: Packed, num_bits: usize) -> ::core::result::Result<Self, Packed> {
                    match u8::unpack_from(value.clone(), num_bits)? {
                        #(#arms)*
                    }
                }
            }
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
    use crate::ir::pattern_match::assert_matches;

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

    fn create_simple() -> Enum {
        Enum {
            ident: parse_quote!(Test),
            storage_ty: parse_quote!(u16),
            generics: Generics::default(),
            byte_order: None,
            catch_all: None,
            variants: vec![
                Variant { ident: parse_quote!(A), discriminant: parse_quote!(0) },
                Variant { ident: parse_quote!(B), discriminant: parse_quote!(1) },
            ],
        }
    }

    fn create_catch_all_empty() -> Enum {
        Enum {
            ident: parse_quote!(Test),
            storage_ty: parse_quote!(u16),
            generics: Generics::default(),
            byte_order: None,
            catch_all: Some((parse_quote!(CatchAll), false)),
            variants: vec![
                Variant { ident: parse_quote!(A), discriminant: parse_quote!(0) },
                Variant { ident: parse_quote!(CatchAll), discriminant: parse_quote!(1) },
            ],
        }
    }

    fn create_catch_all_tuple() -> Enum {
        Enum {
            ident: parse_quote!(Test),
            storage_ty: parse_quote!(u16),
            generics: Generics::default(),
            byte_order: None,
            catch_all: Some((parse_quote!(CatchAll), true)),
            variants: vec![
                Variant { ident: parse_quote!(A), discriminant: parse_quote!(0) },
                Variant { ident: parse_quote!(CatchAll), discriminant: parse_quote!(1) },
            ],
        }
    }

    #[test]
    fn to_serialize_op_simple() {
        let input = create_simple();

        let mut region = Region::new(0);
        input.to_serialize_op(&mut region, ());
        let op = format!("{:#}", region);

        let pattern = "
        {
            impl_serialize [ Test ] |%serializer| {
                %self = self
                %span = match %self {
                    Test :: A => {
                        %disc_a = custom_expr [(0) as u16]
                        %disc_a_ref = ref %disc_a
                        %result_a = serialize_object %serializer, %disc_a_ref
                        yield %result_a
                    }
                    Test :: B => {
                        %disc_b = custom_expr [(1) as u16]
                        %disc_b_ref = ref %disc_b
                        %result_b = serialize_object %serializer, %disc_b_ref
                        yield %result_b
                    }
                }
                yield %span
            }
        }
        ";
        assert_matches!(op, pattern);
    }

    #[test]
    fn to_deserialize_op_simple() {
        let input = create_simple();

        let mut region = Region::new(0);
        input.to_deserialize_op(&mut region, ());
        let op = format!("{:#}", region);

        let pattern = "
        {
            impl_deserialize [ Test ] |%deserializer| {
                %maybe_discriminant = deserialize_object [u16] %deserializer
                %discriminant = try %maybe_discriminant
                %result = match %discriminant {
                    discriminant if discriminant == 0 => {
                        %result_a = struct [Test :: A]
                        %result_a_ok = ok %result_a
                        yield %result_a_ok
                    }
                    discriminant if discriminant == 1 => {
                        %result_b = struct [Test :: B]
                        %result_b_ok = ok %result_b
                        yield %result_b_ok
                    }
                    _ => {
                        %result_err = error [invalid enum discriminant] %deserializer
                        yield %result_err
                    }
                }
                yield %result
            }
        }
        ";
        println!("{}", region.to_token_stream_formatted(false).to_string());

        assert_matches!(op, pattern);
    }

    #[test]
    fn to_serialize_op_catch_all_empty() {
        let input = create_catch_all_empty();

        let mut region = Region::new(0);
        input.to_serialize_op(&mut region, ());
        let op = format!("{:#}", region);

        let pattern = "
        {
            impl_serialize [ Test ] |%serializer| {
                %self = self
                %span = match %self {
                    Test :: A => {
                        %disc_a = custom_expr [(0) as u16]
                        %disc_a_ref = ref %disc_a
                        %result_a = serialize_object %serializer, %disc_a_ref
                        yield %result_a
                    }
                    Test :: CatchAll => {
                        %disc_b = custom_expr [(1) as u16]
                        %disc_b_ref = ref %disc_b
                        %result_b = serialize_object %serializer, %disc_b_ref
                        yield %result_b
                    }
                }
                yield %span
            }
        }
        ";
        assert_matches!(op, pattern);
    }

    #[test]
    fn to_serialize_op_catch_all_tuple() {
        let input = create_catch_all_tuple();

        let mut region = Region::new(0);
        input.to_serialize_op(&mut region, ());
        let op = format!("{:#}", region);

        let pattern = "
        {
            impl_serialize [ Test ] |%serializer| {
                %self = self
                %span = match %self {
                    Test :: A => {
                        %disc_a = custom_expr [(0) as u16]
                        %disc_a_ref = ref %disc_a
                        %result_a = serialize_object %serializer, %disc_a_ref
                        yield %result_a
                    }
                    Test :: CatchAll (value) => {
                        %disc_ca = custom_expr [value]
                        %disc_ca_ref = ref %disc_ca
                        %result_ca = serialize_object %serializer, %disc_ca_ref
                        yield %result_ca
                    }
                }
                yield %span
            }
        }
        ";
        assert_matches!(op, pattern);
    }

    #[test]
    fn to_deserialize_op_catch_all_empty() {
        let input = create_catch_all_empty();

        let mut region = Region::new(0);
        input.to_deserialize_op(&mut region, ());
        let op = format!("{:#}", region);

        let pattern = "
        {
            impl_deserialize [ Test ] |%deserializer| {
                %maybe_discriminant = deserialize_object [u16] %deserializer
                %discriminant = try %maybe_discriminant
                %result = match %discriminant {
                    discriminant if discriminant == 0 => {
                        %result_a = struct [Test :: A]
                        %result_a_ok = ok %result_a
                        yield %result_a_ok
                    }
                    _ => {
                        %result_ca = struct [Test :: CatchAll]
                        %result_ca_ok = ok %result_ca
                        yield %result_ca_ok
                    }
                }
                yield %result
            }
        }
        ";
        assert_matches!(op, pattern);
    }

    #[test]
    fn to_deserialize_op_catch_all_tuple() {
        let input = create_catch_all_tuple();

        let mut region = Region::new(0);
        input.to_deserialize_op(&mut region, ());
        let op = format!("{:#}", region);

        let pattern = "
        {
            impl_deserialize [ Test ] |%deserializer| {
                %maybe_discriminant = deserialize_object [u16] %deserializer
                %discriminant = try %maybe_discriminant
                %result = match %discriminant {
                    discriminant if discriminant == 0 => {
                        %result_a = struct [Test :: A]
                        %result_a_ok = ok %result_a
                        yield %result_a_ok
                    }
                    _ => {
                        %result_ca = struct [Test :: CatchAll, 0] %discriminant
                        %result_ca_ok = ok %result_ca
                        yield %result_ca_ok
                    }
                }
                yield %result
            }
        }
        ";
        assert_matches!(op, pattern);
    }
}
