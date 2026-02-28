use syn::{Expr, Ident, spanned::Spanned};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant {
    pub ident: Ident,
    pub discriminant: Option<Expr>,
    pub default: bool,
}

impl TryFrom<syn::Variant> for Variant {
    type Error = syn::Error;
    fn try_from(value: syn::Variant) -> Result<Self, Self::Error> {
        if !value.fields.is_empty() {
            return Err(syn::Error::new(value.fields.span(), "only fieldless enums are supported"));
        }
        let discriminant = value.discriminant.map(|(_, expr)| expr);
        let default = value.attrs.iter().find(|attr| attr.meta.path().is_ident("default")).is_some();
        Ok(Self { ident: value.ident, discriminant, default })
    }
}
