use syn::{Expr, Ident};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant {
    pub ident: Ident,
    pub discriminant: Expr,
}
