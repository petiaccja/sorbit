use quote::format_ident;
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
