use std::fmt::Debug;
use std::fmt::Formatter;
use std::fmt::Result;

use itertools::Itertools;
use quote::ToTokens as _;

use super::{
    AndThen, Block, DeserializeComposite, DeserializeNothing, DeserializeObject, Enclose, Expr, ImplDeserialize,
    ImplSerialize, IntoBitField, Layout, Let, MakeStruct, MakeTuple, NewBitField, Ok, PackBitField, PackObject,
    SerializeComposite, SerializeNothing, SerializeObject, Statement, SymRef, Try, UnpackObject,
};

//------------------------------------------------------------------------------
// Trait implementation nodes.
//------------------------------------------------------------------------------

impl Debug for ImplSerialize {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "impl Serialize for {} => {:?}", self.name, self.body)
    }
}

impl Debug for ImplDeserialize {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "impl Deserialize for {} => {:?}", self.name, self.body)
    }
}

//------------------------------------------------------------------------------
// Polymorphic expression and statement nodes.
//------------------------------------------------------------------------------

impl Debug for Expr {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Expr::Try(value) => write!(f, "{value:?}"),
            Expr::MakeTuple(value) => write!(f, "{value:?}"),
            Expr::MakeStruct(value) => write!(f, "{value:?}"),
            Expr::AndThen(value) => write!(f, "{value:?}"),
            Expr::Ok(value) => write!(f, "{value:?}"),
            Expr::Block(value) => write!(f, "{value:?}"),
            Expr::Symref(value) => write!(f, "{value:?}"),
            Expr::Enclose(value) => write!(f, "{value:?}"),
            Expr::Layout(value) => write!(f, "{value:?}"),
            Expr::SerializeNothing(value) => write!(f, "{value:?}"),
            Expr::SerializeObject(value) => write!(f, "{value:?}"),
            Expr::SerializeComposite(value) => write!(f, "{value:?}"),
            Expr::DeserializeNothing(value) => write!(f, "{value:?}"),
            Expr::DeserializeObject(value) => write!(f, "{value:?}"),
            Expr::DeserializeComposite(value) => write!(f, "{value:?}"),
            Expr::NewBitField(value) => write!(f, "{value:?}"),
            Expr::IntoBitField(value) => write!(f, "{value:?}"),
            Expr::PackObject(value) => write!(f, "{value:?}"),
            Expr::PackBitField(value) => write!(f, "{value:?}"),
            Expr::UnpackObject(value) => write!(f, "{value:?}"),
        }
    }
}

impl Debug for Statement {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Statement::Let(value) => write!(f, "{value:?}"),
        }
    }
}

//------------------------------------------------------------------------------
// Language expression nodes.
//------------------------------------------------------------------------------

impl Debug for Try {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{:?}?", self.expr)
    }
}

impl Debug for MakeTuple {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "(")?;
        for (index, element) in self.elements.iter().enumerate() {
            match index {
                0 => write!(f, "{element:?}"),
                _ => write!(f, ", {element:?}"),
            }?;
        }
        write!(f, ")")
    }
}

impl Debug for MakeStruct {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}{{ ", self.name)?;
        for (index, element) in self.members.iter().enumerate() {
            match index {
                0 => write!(f, "{element:?}"),
                _ => write!(f, ", {element:?}"),
            }?;
        }
        write!(f, " }}")
    }
}

impl Debug for AndThen {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "{:?}.and_then(|{}| {:?})",
            self.result,
            self.value.as_ref().map(|ident| ident.to_string()).unwrap_or("_".to_owned()),
            self.expr
        )
    }
}

impl Debug for Ok {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "Ok({:?})", self.expr)
    }
}

impl Debug for Block {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{{ ")?;
        for statement in &self.statements {
            write!(f, "{statement:?}; ")?;
        }
        write!(f, "{:?}", self.result)?;
        write!(f, " }}")
    }
}

impl Debug for SymRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self.ident)
    }
}

//------------------------------------------------------------------------------
// Language statement nodes.
//------------------------------------------------------------------------------

impl Debug for Let {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "let {} = {:?}",
            self.ident.as_ref().map(|ident| ident.to_string()).unwrap_or("_".to_owned()),
            self.expr
        )
    }
}

//------------------------------------------------------------------------------
// Serialization expression nodes.
//------------------------------------------------------------------------------

impl Debug for Enclose {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "enclose({:?}, {})", self.expr, self.item)
    }
}

impl Debug for Layout {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let offset = self.align.map(|offset| format!("--{offset}-")).into_iter();
        let align = self.align.map(|align| format!("{align}*n|")).into_iter();
        let value = std::iter::once(format!("{:?}", self.expr));
        let len = self.round.map(|len| format!("--{len}-")).into_iter();
        let round = self.round.map(|round| format!("{round}*n|")).into_iter();
        let s = offset.chain(align).chain(value).chain(len).chain(round).join(" ");
        write!(f, "layout({s})")
    }
}

impl Debug for SerializeNothing {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "serialize_nothing()")
    }
}

impl Debug for SerializeObject {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "serialize_object({:?})", self.object)
    }
}

impl Debug for SerializeComposite {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "serialize_composite({:?})", self.expr)
    }
}

impl Debug for DeserializeNothing {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "deserialize_nothing()")
    }
}

impl Debug for DeserializeObject {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "deserialize_object::<{}>()", self.ty.to_token_stream())
    }
}

impl Debug for DeserializeComposite {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "deserialize_composite({:?})", self.expr)
    }
}

impl Debug for NewBitField {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "new_bit_field::<{}>()", self.ty.to_token_stream())
    }
}

impl Debug for IntoBitField {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "into_bit_field({:?})", self.packed)
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
