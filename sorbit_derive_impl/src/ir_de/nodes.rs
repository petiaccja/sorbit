use std::ops::Range;

use quote::{ToTokens, quote};

#[derive(Clone, PartialEq, Eq)]
pub struct DeserializeImpl {
    pub name: syn::Ident,
    pub generics: syn::Generics,
    pub body: Expr,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Let {
    pub ident: Option<syn::Ident>,
    pub expr: Expr,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Try {
    pub expr: Box<Expr>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Enclose {
    pub item: String,
    pub expr: Box<Expr>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Block {
    pub statements: Vec<Let>,
    pub result: Box<Expr>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Construct {
    pub ty: syn::Type,
    pub args: Vec<syn::Ident>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Name {
    pub ident: syn::Ident,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Ok {
    pub expr: Box<Expr>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct BitFieldFrom {
    pub bits: Box<Expr>,
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
pub struct DeserializeNothing;

#[derive(Clone, PartialEq, Eq)]
pub struct DeserializeObject {
    pub ty: syn::Type,
}

#[derive(Clone, PartialEq, Eq)]
pub struct DeserializeComposite {
    pub body: Box<Expr>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct UnpackObject {
    pub bit_field: syn::Expr,
    pub ty: syn::Type,
    pub bit_range: Range<u8>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Todo;

#[derive(Clone, PartialEq, Eq)]
pub enum Expr {
    Try(Try),
    Enclose(Enclose),
    Block(Block),
    Construct(Construct),
    Name(Name),
    Ok(Ok),
    BitFieldFrom(BitFieldFrom),
    Pad(Pad),
    Align(Align),
    DeserializeNothing(DeserializeNothing),
    DeserializeObject(DeserializeObject),
    DeserializeComposite(DeserializeComposite),
    UnpackObject(UnpackObject),
    Todo(Todo),
}

//------------------------------------------------------------------------------
// Implement conversions for Expr.
//------------------------------------------------------------------------------

impl From<Try> for Expr {
    fn from(value: Try) -> Self {
        Self::Try(value)
    }
}

impl From<Enclose> for Expr {
    fn from(value: Enclose) -> Self {
        Self::Enclose(value)
    }
}

impl From<Block> for Expr {
    fn from(value: Block) -> Self {
        Self::Block(value)
    }
}

impl From<Construct> for Expr {
    fn from(value: Construct) -> Self {
        Self::Construct(value)
    }
}

impl From<Name> for Expr {
    fn from(value: Name) -> Self {
        Self::Name(value)
    }
}

impl From<Ok> for Expr {
    fn from(value: Ok) -> Self {
        Self::Ok(value)
    }
}

impl From<BitFieldFrom> for Expr {
    fn from(value: BitFieldFrom) -> Self {
        Self::BitFieldFrom(value)
    }
}

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

impl From<DeserializeNothing> for Expr {
    fn from(value: DeserializeNothing) -> Self {
        Self::DeserializeNothing(value)
    }
}

impl From<DeserializeObject> for Expr {
    fn from(value: DeserializeObject) -> Self {
        Self::DeserializeObject(value)
    }
}

impl From<DeserializeComposite> for Expr {
    fn from(value: DeserializeComposite) -> Self {
        Self::DeserializeComposite(value)
    }
}

impl From<UnpackObject> for Expr {
    fn from(value: UnpackObject) -> Self {
        Self::UnpackObject(value)
    }
}

impl From<Todo> for Expr {
    fn from(value: Todo) -> Self {
        Self::Todo(value)
    }
}

//------------------------------------------------------------------------------
// Implement Debug trait.
//------------------------------------------------------------------------------

impl std::fmt::Debug for DeserializeImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "impl Deserialize for {} {{ {:?} }}", self.name, &self.body)
    }
}

impl std::fmt::Debug for Let {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} = {:?}", self.ident.as_ref().map(|n| n.to_string()).unwrap_or("_".into()), self.expr)
    }
}

impl std::fmt::Debug for Try {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}?", &self.expr)
    }
}

impl std::fmt::Debug for Enclose {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}.enclose({})", self.expr, self.item)
    }
}

impl std::fmt::Debug for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{ ")?;
        for statement in &self.statements {
            write!(f, "{statement:?}; ")?;
        }
        write!(f, "{:?}", self.result)?;
        write!(f, " }}")
    }
}

impl std::fmt::Debug for Construct {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {{ ", self.ty.to_token_stream())?;
        for arg in &self.args {
            write!(f, "{arg:?}, ")?;
        }
        write!(f, "}}")
    }
}

impl std::fmt::Debug for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.ident.to_token_stream())
    }
}

impl std::fmt::Debug for Ok {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ok({:?})", self.expr)
    }
}

impl std::fmt::Debug for BitFieldFrom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "bit_field({:?})", self.bits)
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

impl std::fmt::Debug for DeserializeNothing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "deserialize_nothing()")
    }
}

impl std::fmt::Debug for DeserializeObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "deserialize::<{}>()", self.ty.to_token_stream())
    }
}

impl std::fmt::Debug for DeserializeComposite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "composite{{ {:?} }}", &self.body)
    }
}

impl std::fmt::Debug for UnpackObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "unpack({}[{}..{}] -> {})",
            self.bit_field.to_token_stream().to_string(),
            self.bit_range.start,
            self.bit_range.end,
            self.ty.to_token_stream(),
        )
    }
}

impl std::fmt::Debug for Todo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "todo!()")
    }
}

impl std::fmt::Debug for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Try(r#try) => write!(f, "{:?}", r#try),
            Expr::Enclose(enclose) => write!(f, "{enclose:?}"),
            Expr::Block(block) => write!(f, "{block:?}"),
            Expr::Construct(construct) => write!(f, "{construct:?}"),
            Expr::Name(name) => write!(f, "{name:?}"),
            Expr::Ok(ok) => write!(f, "{ok:?}"),
            Expr::BitFieldFrom(from_bits) => write!(f, "{from_bits:?}"),
            Expr::Pad(pad) => write!(f, "{pad:?}"),
            Expr::Align(align) => write!(f, "{align:?}"),
            Expr::DeserializeNothing(deserialize_nothing) => write!(f, "{deserialize_nothing:?}"),
            Expr::DeserializeObject(deserialize_object) => write!(f, "{deserialize_object:?}"),
            Expr::DeserializeComposite(deserialize_composite) => write!(f, "{deserialize_composite:?}"),
            Expr::UnpackObject(unpack_object) => write!(f, "{unpack_object:?}"),
            Expr::Todo(todo) => write!(f, "{todo:?}"),
        }
    }
}

//------------------------------------------------------------------------------
// Implement ToTokens trait.
//------------------------------------------------------------------------------

struct DeserializerTrait;
struct DeserializerType;
struct DeserializeTrait;
struct DeserializerObject;
struct ErrorTrait;
struct BitFieldType;

const DESERIALIZER_TRAIT: DeserializerTrait = DeserializerTrait {};
const DESERIALIZER_TYPE: DeserializerType = DeserializerType {};
const DESERIALIZE_TRAIT: DeserializeTrait = DeserializeTrait {};
const DESERIALIZER_OBJECT: DeserializerObject = DeserializerObject {};
const ERROR_TRAIT: ErrorTrait = ErrorTrait {};
const BIT_FIELD_TYPE: BitFieldType = BitFieldType {};

impl ToTokens for DeserializerTrait {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend(quote! {::sorbit::deserialize::Deserializer});
    }
}

impl ToTokens for DeserializerType {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend(quote! {D});
    }
}

impl ToTokens for DeserializeTrait {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend(quote! {::sorbit::deserialize::Deserialize});
    }
}

impl ToTokens for DeserializerObject {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend(quote! {deserializer});
    }
}

impl ToTokens for BitFieldType {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend(quote! {::sorbit::bit::BitField});
    }
}

impl ToTokens for DeserializeImpl {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let name = &self.name;
        let body = &self.body;
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();
        tokens.extend(quote! {
            #[automatically_derived]
            impl #impl_generics #DESERIALIZE_TRAIT for #name #ty_generics #where_clause{
                fn deserialize<#DESERIALIZER_TYPE: #DESERIALIZER_TRAIT>(
                    #DESERIALIZER_OBJECT: &mut #DESERIALIZER_TYPE
                ) -> ::core::result::Result<
                        Self,
                        <#DESERIALIZER_TYPE as #DESERIALIZER_TRAIT>::Error
                    >
                {
                    #body
                }
            }
        });
    }
}

impl ToTokens for ErrorTrait {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend(quote! {::sorbit::error::SerializeError});
    }
}

impl ToTokens for Let {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let expr = &self.expr;
        if let Some(ident) = self.ident.as_ref() {
            tokens.extend(quote! { let #ident = #expr; });
        } else {
            tokens.extend(quote! { let _ = #expr; });
        };
    }
}

impl ToTokens for Try {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let expr = &self.expr;
        tokens.extend(quote! { #expr? });
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

impl ToTokens for Block {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let statements = &self.statements;
        let result = &self.result;
        tokens.extend(quote! { #(#statements)* #result });
    }
}

impl ToTokens for Construct {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let ty = &self.ty;
        let args = &self.args;
        tokens.extend(quote! { #ty { #(#args),* } });
    }
}

impl ToTokens for Name {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.ident.to_tokens(tokens);
    }
}

impl ToTokens for Ok {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let expr = &self.expr;
        tokens.extend(quote! { ::core::result::Result::Ok(#expr) });
    }
}

impl ToTokens for BitFieldFrom {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let bits = &self.bits;
        tokens.extend(quote! { #BIT_FIELD_TYPE::from_bits(#bits) });
    }
}

impl ToTokens for Pad {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let until = self.until;
        tokens.extend(quote! { #DESERIALIZER_TRAIT::pad(#DESERIALIZER_OBJECT, #until)});
    }
}

impl ToTokens for Align {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let multiple_of = self.multiple_of;
        tokens.extend(quote! { #DESERIALIZER_TRAIT::align(#DESERIALIZER_OBJECT, #multiple_of)});
    }
}

impl ToTokens for DeserializeNothing {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend(quote! { #DESERIALIZER_TRAIT::deserialize_nothing(#DESERIALIZER_OBJECT)});
    }
}

impl ToTokens for DeserializeObject {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let ty = &self.ty;
        tokens.extend(quote! { <#ty as #DESERIALIZE_TRAIT>::deserialize(#DESERIALIZER_OBJECT)});
    }
}

impl ToTokens for DeserializeComposite {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let body = &self.body;
        tokens.extend(quote! {
            #DESERIALIZER_TRAIT::deserialize_composite(#DESERIALIZER_OBJECT, |#DESERIALIZER_OBJECT| {
                #body
            })
        });
    }
}

impl ToTokens for UnpackObject {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let ty = &self.ty;
        let bit_field = &self.bit_field;
        let Range { start, end } = &self.bit_range;
        tokens.extend(quote! {
            #bit_field.unpack::<#ty, _, _>(#start..#end)
                      .map_err(|err| ::sorbit::codegen::bit_error_to_error_de(#DESERIALIZER_OBJECT, err))
        });
    }
}

impl ToTokens for Expr {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            Expr::Try(r#try) => r#try.to_tokens(tokens),
            Expr::Enclose(enclose) => enclose.to_tokens(tokens),
            Expr::Block(block) => block.to_tokens(tokens),
            Expr::Construct(construct) => construct.to_tokens(tokens),
            Expr::Name(name) => name.to_tokens(tokens),
            Expr::Ok(ok) => ok.to_tokens(tokens),
            Expr::BitFieldFrom(bit_field) => bit_field.to_tokens(tokens),
            Expr::Pad(pad) => pad.to_tokens(tokens),
            Expr::Align(align) => align.to_tokens(tokens),
            Expr::DeserializeNothing(deserialize_nothing) => deserialize_nothing.to_tokens(tokens),
            Expr::DeserializeObject(deserialize_object) => deserialize_object.to_tokens(tokens),
            Expr::DeserializeComposite(deserialize_composite) => deserialize_composite.to_tokens(tokens),
            Expr::UnpackObject(unpack_object) => unpack_object.to_tokens(tokens),
            Expr::Todo(todo) => todo.to_tokens(tokens),
        }
    }
}

impl ToTokens for Todo {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend(quote! { todo!() });
    }
}
