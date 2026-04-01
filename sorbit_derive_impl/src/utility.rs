use std::collections::{HashMap, HashSet};

use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{Expr, GenericArgument, Path, PathArguments, PathSegment, TypePath, parse_quote, spanned::Spanned as _};

/// Convert a type which is single ident into an actual type.
pub fn ident_to_type(ident: syn::Ident) -> syn::Type {
    parse_quote!(#ident)
}

/// Return a variable name corresponding to the struct member.
///
/// For index members, the index is prefixed to make a valid variable name.
pub fn member_to_ident(member: syn::Member) -> syn::Ident {
    match member {
        syn::Member::Named(ident) => ident,
        syn::Member::Unnamed(syn::Index { index, span: _ }) => format_ident!("m{index}"),
    }
}

pub fn to_member(ident: Option<syn::Ident>, index: usize, span: Span) -> syn::Member {
    ident
        .map(|ident| syn::Member::from(ident))
        .unwrap_or_else(|| syn::Member::Unnamed(syn::Index { index: index as u32, span }))
}

/// Return a pattern that deconstructs a structure.
pub fn deconstruct_pattern(struct_ty: &syn::Type, members: impl Iterator<Item = syn::Member>) -> syn::Pat {
    let members = members.map(|member| match member {
        syn::Member::Named(ident) => quote! { #ident },
        syn::Member::Unnamed(index) => {
            let ident = member_to_ident(syn::Member::Unnamed(index.clone()));
            quote! { #index: #ident }
        }
    });
    parse_quote!(#struct_ty{ #(#members),* })
}

/// Return a pattern that deconstructs a structure with the identifier explicitly given.
pub fn deconstruct_pattern_explicit(
    struct_ty: &syn::Type,
    members: impl Iterator<Item = (syn::Member, syn::Ident)>,
) -> syn::Pat {
    let members = members.map(|(member, ident)| match member {
        syn::Member::Named(field) => {
            if field == ident {
                quote! { #ident }
            } else {
                quote! { #field: #ident }
            }
        }
        syn::Member::Unnamed(index) => {
            quote! { #index: #ident }
        }
    });
    parse_quote!(#struct_ty{ #(#members),* })
}

pub fn check_invalid_parameters<'a>(
    parameters: &HashMap<Path, Expr>,
    accepted_parameters: impl Iterator<Item = &'a Path>,
) -> Result<(), syn::Error> {
    let accepted_parameters: HashSet<_> = accepted_parameters.cloned().collect();
    for (parameter, _) in parameters {
        if !accepted_parameters.contains(parameter) {
            return Err(syn::Error::new(parameter.span(), "parameter is not accepted here"));
        }
    }
    Ok(())
}

pub trait PhantomType {
    fn is_phantom(&self) -> bool;
    fn phantom_underlying_type(&self) -> &syn::Type;
}

impl PhantomType for syn::Type {
    fn is_phantom(&self) -> bool {
        match self {
            syn::Type::Path(TypePath { qself: _, path }) => {
                let mut rev_segments = path.segments.iter().rev();
                match rev_segments.next() {
                    Some(segment) if segment.ident == "PhantomData" => (),
                    _ => return false,
                };
                match rev_segments.next() {
                    Some(segment) if segment.ident == "marker" => (),
                    None if path.leading_colon.is_none() => (),
                    _ => return false,
                };
                match rev_segments.next() {
                    Some(segment) if segment.ident == "std" || segment.ident == "core" => (),
                    None if path.leading_colon.is_none() => (),
                    _ => return false,
                };
                !path.segments.is_empty()
            }
            _ => false,
        }
    }

    fn phantom_underlying_type(&self) -> &syn::Type {
        match self {
            syn::Type::Path(TypePath { qself: _, path }) => {
                let mut rev_segments = path.segments.iter().rev();
                match rev_segments.next() {
                    Some(segment) if segment.ident == "PhantomData" => (),
                    _ => return self,
                };
                match rev_segments.next() {
                    Some(segment) if segment.ident == "marker" => (),
                    None if path.leading_colon.is_none() => (),
                    _ => return self,
                };
                match rev_segments.next() {
                    Some(segment) if segment.ident == "std" || segment.ident == "core" => (),
                    None if path.leading_colon.is_none() => (),
                    _ => return self,
                };
                if let Some(PathSegment { arguments: PathArguments::AngleBracketed(args), .. }) = path.segments.last() {
                    if let (Some(GenericArgument::Type(ty)), 1) = (args.args.first(), args.args.len()) {
                        return ty;
                    }
                };
                self
            }
            _ => self,
        }
    }
}

#[cfg(test)]
mod tests {
    use quote::ToTokens;
    use rstest::rstest;
    use syn::Type;

    use super::*;

    #[rstest]
    #[case(parse_quote!(u8), parse_quote!(u8), false)]
    #[case(parse_quote!(PhantomData<u8>), parse_quote!(u8), true)]
    #[case(parse_quote!(marker::PhantomData<u8>), parse_quote!(u8), true)]
    #[case(parse_quote!(std::marker::PhantomData<u8>), parse_quote!(u8), true)]
    #[case(parse_quote!(core::marker::PhantomData<u8>), parse_quote!(u8), true)]
    #[case(parse_quote!(::std::marker::PhantomData<u8>), parse_quote!(u8), true)]
    #[case(parse_quote!(::core::marker::PhantomData<u8>), parse_quote!(u8), true)]
    #[case(parse_quote!(::marker::PhantomData<u8>), parse_quote!(::marker::PhantomData<u8>), false)]
    #[case(parse_quote!(special::PhantomData<u8>), parse_quote!(special::PhantomData<u8>), false)]
    #[case(parse_quote!(::special::PhantomData<u8>), parse_quote!(::special::PhantomData<u8>), false)]
    #[case(parse_quote!(special::marker::PhantomData<u8>), parse_quote!(special::marker::PhantomData<u8>), false)]
    #[case(parse_quote!(::special::marker::PhantomData<u8>), parse_quote!(::special::marker::PhantomData<u8>), false)]
    fn phantom_type(#[case] ty: Type, #[case] underlying_ty: Type, #[case] is_phantom: bool) {
        assert_eq!(
            ty.phantom_underlying_type().to_token_stream().to_string(),
            underlying_ty.to_token_stream().to_string()
        );
        assert_eq!(ty.is_phantom(), is_phantom);
    }
}
