use quote::{format_ident, quote};
use syn::parse_quote;

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

/// Return a pattern that deconstructs a structure.
pub fn deconstruct_pattern<'a>(struct_ty: &syn::Type, members: impl Iterator<Item = &'a syn::Member>) -> syn::Pat {
    let members = members.map(|member| match member {
        syn::Member::Named(ident) => quote! { #ident },
        syn::Member::Unnamed(index) => {
            let ident = member_to_ident((*member).clone());
            quote! { #index: #ident }
        }
    });
    parse_quote!(#struct_ty{ #(#members),* })
}
