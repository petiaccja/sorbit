pub mod constants;
mod debug;
mod lower;

//------------------------------------------------------------------------------
// Trait implementation nodes.
//------------------------------------------------------------------------------

use std::ops::Range;

#[derive(Clone, PartialEq, Eq)]
pub struct ImplSerialize {
    pub name: syn::Ident,
    pub generics: syn::Generics,
    pub body: Expr,
}

#[derive(Clone, PartialEq, Eq)]
pub struct ImplDeserialize {
    pub name: syn::Ident,
    pub generics: syn::Generics,
    pub body: Expr,
}

//------------------------------------------------------------------------------
// Polymorphic expression and statement nodes.
//------------------------------------------------------------------------------

#[derive(Clone, PartialEq, Eq)]
pub enum Expr {
    Try(Box<Try>),
    MakeTuple(MakeTuple),
    MakeStruct(MakeStruct),
    AndThen(Box<AndThen>),
    Ok(Box<Ok>),
    Block(Box<Block>),
    Symref(SymRef),
    Enclose(Box<Enclose>),
    Layout(Box<Layout>),
    SerializeNothing(SerializeNothing),
    SerializeObject(SerializeObject),
    SerializeComposite(Box<SerializeComposite>),
    DeserializeNothing(DeserializeNothing),
    DeserializeObject(DeserializeObject),
    DeserializeComposite(Box<DeserializeComposite>),
    NewBitField(NewBitField),
    IntoBitField(Box<IntoBitField>),
    PackObject(PackObject),
    PackBitField(PackBitField),
    UnpackObject(UnpackObject),
}

#[derive(Clone, PartialEq, Eq)]
pub enum Statement {
    Let(Let),
}

//------------------------------------------------------------------------------
// Language expression nodes.
//------------------------------------------------------------------------------

#[derive(Clone, PartialEq, Eq)]
pub struct Try {
    pub expr: Expr,
}

#[derive(Clone, PartialEq, Eq)]
pub struct MakeTuple {
    pub elements: Vec<Expr>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct MakeStruct {
    pub name: syn::Ident,
    pub members: Vec<(syn::Member, Expr)>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct AndThen {
    pub result: Expr,
    pub value: Option<syn::Ident>,
    pub expr: Expr,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Ok {
    pub expr: Expr,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Block {
    pub statements: Vec<Statement>,
    pub result: Expr,
}

#[derive(Clone, PartialEq, Eq)]
pub struct SymRef {
    pub ident: syn::Ident,
}

//------------------------------------------------------------------------------
// Language statement nodes.
//------------------------------------------------------------------------------

#[derive(Clone, PartialEq, Eq)]
pub struct Let {
    pub ident: Option<syn::Ident>,
    pub expr: Expr,
}

//------------------------------------------------------------------------------
// Serialization expression nodes.
//------------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Direction {
    Serialize,
    Deserialize,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Enclose {
    pub item: String,
    pub expr: Expr,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Layout {
    pub expr: Expr,
    pub offset: Option<u64>,
    pub align: Option<u64>,
    pub len: Option<u64>,
    pub round: Option<u64>,
    pub direction: Direction,
}

#[derive(Clone, PartialEq, Eq)]
pub struct SerializeNothing;

#[derive(Clone, PartialEq, Eq)]
pub struct SerializeObject {
    pub object: syn::Expr,
}

#[derive(Clone, PartialEq, Eq)]
pub struct SerializeComposite {
    pub expr: Expr,
}

#[derive(Clone, PartialEq, Eq)]
pub struct DeserializeNothing;

#[derive(Clone, PartialEq, Eq)]
pub struct DeserializeObject {
    pub ty: syn::Type,
}

#[derive(Clone, PartialEq, Eq)]
pub struct DeserializeComposite {
    pub expr: Expr,
}

#[derive(Clone, PartialEq, Eq)]
pub struct NewBitField {
    pub ty: syn::Type,
}

#[derive(Clone, PartialEq, Eq)]
pub struct IntoBitField {
    pub packed: Expr,
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
pub struct UnpackObject {
    pub bit_field: syn::Expr,
    pub ty: syn::Type,
    pub bit_range: Range<u8>,
}
