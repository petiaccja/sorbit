use std::ops::Range;

use quote::{ToTokens, quote};

#[derive(Clone, PartialEq, Eq)]
pub struct SerializeImpl {
    pub name: syn::Ident,
    pub generics: syn::Generics,
    pub body: Expr,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Pad {
    pub until: u64,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Align {
    pub multiple_of: u64,
}

#[derive(Clone, PartialEq, Eq)]
pub struct SerializeNothing;

#[derive(Clone, PartialEq, Eq)]
pub struct SerializeObject {
    pub object: syn::Expr,
}

#[derive(Clone, PartialEq, Eq)]
pub struct SerializeComposite {
    pub members: Vec<Expr>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Enclose {
    pub item: String,
    pub expr: Box<Expr>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Chain {
    exprs: Vec<Expr>,
    vars: Vec<Option<syn::Ident>>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct PackObject {
    pub bit_field: syn::Expr,
    pub object: syn::Expr,
    pub bit_range: Range<u8>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct PackBitField {
    pub bit_field: syn::Ident,
    pub packed_ty: syn::Type,
    pub members: Vec<Expr>,
}

#[derive(Clone, PartialEq, Eq)]
pub enum Expr {
    Pad(Pad),
    Align(Align),
    SerializeNothing(SerializeNothing),
    SerializeObject(SerializeObject),
    SerializeComposite(SerializeComposite),
    Enclose(Enclose),
    Chain(Chain),
    PackObject(PackObject),
    PackBitField(PackBitField),
}

//------------------------------------------------------------------------------
// Implement conversions for Expr.
//------------------------------------------------------------------------------

impl From<Pad> for Expr {
    fn from(value: Pad) -> Self {
        Self::Pad(value)
    }
}

impl From<Align> for Expr {
    fn from(value: Align) -> Self {
        Self::Align(value)
    }
}

impl From<SerializeNothing> for Expr {
    fn from(value: SerializeNothing) -> Self {
        Self::SerializeNothing(value)
    }
}

impl From<SerializeObject> for Expr {
    fn from(value: SerializeObject) -> Self {
        Self::SerializeObject(value)
    }
}

impl From<SerializeComposite> for Expr {
    fn from(value: SerializeComposite) -> Self {
        Self::SerializeComposite(value)
    }
}

impl From<Enclose> for Expr {
    fn from(value: Enclose) -> Self {
        Self::Enclose(value)
    }
}

impl From<Chain> for Expr {
    fn from(value: Chain) -> Self {
        Self::Chain(value)
    }
}

impl From<PackObject> for Expr {
    fn from(value: PackObject) -> Self {
        Self::PackObject(value)
    }
}

impl From<PackBitField> for Expr {
    fn from(value: PackBitField) -> Self {
        Self::PackBitField(value)
    }
}

//------------------------------------------------------------------------------
// Implement flatten.
//------------------------------------------------------------------------------

impl Chain {
    pub fn new(exprs: Vec<Expr>, vars: Vec<Option<syn::Ident>>) -> Self {
        assert_eq!(
            vars.len() + 1,
            exprs.len(),
            "chain: length of vars ({}) does not match length of exprs ({})",
            vars.len(),
            exprs.len()
        );
        Self { exprs, vars }
    }

    pub fn new_placeholder_vars(exprs: Vec<Expr>) -> Self {
        let vars: Vec<_> = std::iter::repeat_n(None, std::cmp::max(1, exprs.len()) - 1).collect();
        Self::new(exprs, vars)
    }

    pub fn flatten(self) -> Self {
        let mut vars = Vec::new();
        let mut exprs = Vec::new();
        let current_vars = self.vars.into_iter();
        let mut current_exprs = self.exprs.into_iter();

        match current_exprs.next() {
            Some(Expr::Chain(chain)) => {
                let Chain { exprs: mut sub_exprs, vars: mut sub_vars } = chain.flatten();
                vars.append(&mut sub_vars);
                exprs.append(&mut sub_exprs);
            }
            Some(expr) => {
                exprs.push(expr);
            }
            None => (),
        };

        for (expr, var) in current_exprs.zip(current_vars) {
            match (expr, var) {
                (Expr::Chain(chain), None) => {
                    let Chain { exprs: mut sub_exprs, vars: mut sub_vars } = chain.flatten();
                    vars.push(None);
                    vars.append(&mut sub_vars);
                    exprs.append(&mut sub_exprs);
                }
                (expr, var) => {
                    exprs.push(expr);
                    vars.push(var);
                }
            };
        }

        Self::new(exprs, vars)
    }
}

impl Expr {
    pub fn flatten(self) -> Self {
        match self {
            Self::Chain(chain) => Self::Chain(chain.flatten()),
            _ => self,
        }
    }
}

//------------------------------------------------------------------------------
// Implement Debug trait.
//------------------------------------------------------------------------------

impl std::fmt::Debug for SerializeImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "impl Serialize for {} {{ {:?} }}", self.name, self.body)
    }
}

impl std::fmt::Debug for Pad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "pad({})", self.until)
    }
}

impl std::fmt::Debug for Align {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "align({})", self.multiple_of)
    }
}
impl std::fmt::Debug for SerializeNothing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "serialize_nothing()")
    }
}
impl std::fmt::Debug for SerializeObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = &self.object;
        write!(f, "serialize({})", quote! {#value})
    }
}
impl std::fmt::Debug for SerializeComposite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "composite{{")?;
        for (index, member) in self.members.iter().enumerate() {
            if index + 1 == self.members.len() {
                write!(f, "{member:?}")?;
            } else {
                write!(f, "{member:?}, ")?;
            }
        }
        write!(f, "}}")
    }
}

impl std::fmt::Debug for Enclose {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}.enclose({})", self.expr, self.item)
    }
}

impl std::fmt::Debug for Chain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "chain(")?;
        for (index, expr) in self.exprs.iter().enumerate() {
            if index + 1 == self.exprs.len() {
                write!(f, "{expr:?}")?;
            } else {
                let var = self.vars[index].as_ref().map(|var| var.to_string()).unwrap_or("_".into());
                write!(f, "{expr:?} -> |{}| ", var)?;
            }
        }
        write!(f, ")")
    }
}

impl std::fmt::Debug for PackObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "pack({} -> {}[{}..{}])",
            self.object.to_token_stream().to_string(),
            self.bit_field.to_token_stream().to_string(),
            self.bit_range.start,
            self.bit_range.end
        )
    }
}

impl std::fmt::Debug for PackBitField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "bit_field{{")?;
        for (index, member) in self.members.iter().enumerate() {
            if index + 1 == self.members.len() {
                write!(f, "{member:?}")?;
            } else {
                write!(f, "{member:?}, ")?;
            }
        }
        write!(f, "}}")
    }
}

impl std::fmt::Debug for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Pad(pad) => write!(f, "{pad:?}"),
            Expr::Align(align) => write!(f, "{align:?}"),
            Expr::SerializeNothing(serialize_nothing) => write!(f, "{serialize_nothing:?}"),
            Expr::SerializeObject(serialize_value) => write!(f, "{serialize_value:?}"),
            Expr::SerializeComposite(serialize_composite) => write!(f, "{serialize_composite:?}"),
            Expr::Enclose(enclose) => write!(f, "{enclose:?}"),
            Expr::Chain(chain) => write!(f, "{chain:?}"),
            Expr::PackObject(pack_object) => write!(f, "{pack_object:?}"),
            Expr::PackBitField(pack_bit_field) => write!(f, "{pack_bit_field:?}"),
        }
    }
}

//------------------------------------------------------------------------------
// Implement ToTokens trait.
//------------------------------------------------------------------------------

struct SerializerTrait;
struct SerializerOutputTrait;
struct SerializerType;
struct SerializeTrait;
struct SerializerObject;
struct ErrorTrait;

const SERIALIZER_TRAIT: SerializerTrait = SerializerTrait {};
const SERIALIZER_OUTPUT_TRAIT: SerializerOutputTrait = SerializerOutputTrait {};
const SERIALIZER_TYPE: SerializerType = SerializerType {};
const SERIALIZE_TRAIT: SerializeTrait = SerializeTrait {};
const SERIALIZER_OBJECT: SerializerObject = SerializerObject {};
const ERROR_TRAIT: ErrorTrait = ErrorTrait {};

impl ToTokens for SerializerTrait {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend(quote! {::sorbit::serialize::Serializer});
    }
}

impl ToTokens for SerializerOutputTrait {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend(quote! {::sorbit::serialize::SerializerOutput});
    }
}

impl ToTokens for SerializerType {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend(quote! {S});
    }
}

impl ToTokens for SerializeTrait {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend(quote! {::sorbit::serialize::Serialize});
    }
}

impl ToTokens for SerializerObject {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend(quote! {serializer});
    }
}

impl ToTokens for ErrorTrait {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend(quote! {::sorbit::error::SerializeError});
    }
}

impl ToTokens for SerializeImpl {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let name = &self.name;
        let body = &self.body;
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();
        tokens.extend(quote! {
            impl #impl_generics #SERIALIZE_TRAIT for #name #ty_generics #where_clause{
                fn serialize<#SERIALIZER_TYPE: #SERIALIZER_TRAIT>(
                    &self,
                    #SERIALIZER_OBJECT: &mut #SERIALIZER_TYPE
                ) -> ::core::result::Result<
                        <#SERIALIZER_TYPE as #SERIALIZER_OUTPUT_TRAIT>::Success,
                        <#SERIALIZER_TYPE as #SERIALIZER_OUTPUT_TRAIT>::Error
                    >
                {
                    #body
                }
            }
        });
    }
}

impl ToTokens for Pad {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let until = self.until;
        tokens.extend(quote! { #SERIALIZER_TRAIT::pad(#SERIALIZER_OBJECT, #until)});
    }
}

impl ToTokens for Align {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let multiple_of = self.multiple_of;
        tokens.extend(quote! { #SERIALIZER_TRAIT::align(#SERIALIZER_OBJECT, #multiple_of)});
    }
}

impl ToTokens for SerializeNothing {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend(quote! { #SERIALIZER_TRAIT::serialize_nothing(#SERIALIZER_OBJECT)});
    }
}

impl ToTokens for SerializeObject {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let object = &self.object;
        tokens.extend(quote! { #SERIALIZE_TRAIT::serialize(#object, #SERIALIZER_OBJECT)});
    }
}

impl ToTokens for SerializeComposite {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let chain = Chain::new_placeholder_vars(if !self.members.is_empty() {
            self.members.clone()
        } else {
            vec![SerializeNothing {}.into()]
        });
        tokens.extend(quote! {
            #SERIALIZER_TRAIT::serialize_composite(#SERIALIZER_OBJECT, |#SERIALIZER_OBJECT| {
                #chain
            })
        });
    }
}

impl ToTokens for Enclose {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let expr = &self.expr;
        let item = &self.item;
        tokens.extend(quote! {
            #expr.map_err(|err| #ERROR_TRAIT::enclose(err, #item))
        });
    }
}

impl ToTokens for Chain {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        for (index, expr) in self.exprs.iter().enumerate() {
            if index == 0 {
                tokens.extend(quote! {#expr});
            } else {
                if let Some(var) = &self.vars[index - 1] {
                    tokens.extend(quote! {.and_then(|#var| #expr)});
                } else {
                    tokens.extend(quote! {.and_then(|_| #expr)});
                }
            }
        }
    }
}

impl ToTokens for Expr {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            Expr::Pad(pad) => pad.to_tokens(tokens),
            Expr::Align(align) => align.to_tokens(tokens),
            Expr::SerializeNothing(serialize_nothing) => serialize_nothing.to_tokens(tokens),
            Expr::SerializeObject(serialize_object) => serialize_object.to_tokens(tokens),
            Expr::SerializeComposite(serialize_composite) => serialize_composite.to_tokens(tokens),
            Expr::Enclose(enclose) => enclose.to_tokens(tokens),
            Expr::Chain(chain) => chain.to_tokens(tokens),
            Expr::PackObject(pack_object) => pack_object.to_tokens(tokens),
            Expr::PackBitField(pack_bit_field) => pack_bit_field.to_tokens(tokens),
        }
    }
}

impl ToTokens for PackObject {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let object = &self.object;
        let bit_field = &self.bit_field;
        let Range { start, end } = &self.bit_range;
        tokens.extend(quote! {
            #bit_field.pack(#object, #start..#end)
                      .map_err(|err| ::sorbit::codegen::bit_error_to_error(#SERIALIZER_OBJECT, err))
        });
    }
}

impl ToTokens for PackBitField {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let bit_field = &self.bit_field;
        let packed_ty = &self.packed_ty;
        let members = &self.members;
        tokens.extend(quote! {
            {
                let mut #bit_field = ::sorbit::bit::BitField::<#packed_ty>::new();
                let results = [
                    #(#members,)*
                ];
                results.into_iter().fold(Ok(()), |acc, result| acc.and(result)).map(|_| bit_field)
            }
        });
    }
}
