use itertools::Itertools;
use petgraph::{algo::toposort, graph::DiGraph};
use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use std::{
    collections::HashMap,
    sync::atomic::{AtomicU64, Ordering},
};

static OPERATION_ID: AtomicU64 = AtomicU64::new(0);

fn unique_id() -> u64 {
    OPERATION_ID.fetch_add(1, Ordering::Relaxed)
}

pub struct Region {
    arguments: Vec<Value>,
    operations: Vec<Operation>,
}

impl Region {
    pub fn new(num_args: usize, operations: impl FnOnce(&[Value]) -> Vec<Operation>) -> Self {
        let id = unique_id();
        let arguments: Vec<_> = (0..num_args).map(|index| Value { parent: id, index }).collect();
        let operations = operations(&arguments);
        Self { arguments, operations }
    }

    pub fn operations(&self) -> &[Operation] {
        &self.operations
    }

    pub fn argument(&self, index: usize) -> Value {
        self.arguments[index].clone()
    }
}

impl ToTokens for Region {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let mut graph = DiGraph::new();
        let mut id_to_node = HashMap::new();
        {
            let mut predecessor = None;
            for op in &self.operations {
                let node = graph.add_node(op);
                id_to_node.insert(op.id, node);
                if let Some(predecessor) = predecessor {
                    graph.add_edge(predecessor, node, ());
                }
                predecessor = Some(node);
            }
        }

        // This dependency graph is incomplete.
        // If a region of an op uses a value from another op, it's not included.
        // Regions must also be traversed recursively for each op.
        for node in graph.node_indices() {
            let op = graph.node_weight(node).unwrap();
            for input in &op.inputs {
                if let Some(predecessor) = id_to_node.get(&input.parent) {
                    graph.add_edge(*predecessor, node, ());
                }
            }
        }

        let ts = match toposort(&graph, None) {
            Ok(order) => {
                let ops = order.iter().map(|op_idx| {
                    let op = graph.node_weight(*op_idx).unwrap();
                    let outputs = (0..op.num_outputs).map(|index| op.output(index));
                    if op.mnemonic == "yield" {
                        quote! { #op }
                    } else if op.num_outputs == 0 {
                        quote! { #op; }
                    } else if op.num_outputs == 1 {
                        quote! { let #(#outputs),* = #op; }
                    } else {
                        quote! { let (#(#outputs),*) = #op; }
                    }
                });
                quote! {{
                    #(#ops)*
                }}
            }
            Err(cycle) => {
                syn::Error::new(proc_macro2::Span::call_site(), format!("cycle at node {}", cycle.node_id().index()))
                    .into_compile_error()
            }
        };
        tokens.extend(ts);
    }
}

impl std::fmt::Debug for Region {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "|{}| ", self.arguments.iter().map(|value| format!("{value:?}")).join(", "))?;
        f.debug_list().entries(self.operations.iter()).finish()
    }
}

pub struct Operation {
    id: u64,
    mnemonic: String,
    attributes: Vec<String>,
    to_token_stream: Box<dyn Fn(&Operation) -> TokenStream>,
    pub num_outputs: usize,
    pub inputs: Vec<Value>,
    pub regions: Vec<Region>,
}

impl Operation {
    pub fn new(
        mnemonic: String,
        attributes: Vec<String>,
        to_token_stream: Box<dyn Fn(&Operation) -> TokenStream>,
        num_outputs: usize,
        inputs: Vec<Value>,
        regions: Vec<Region>,
    ) -> Self {
        Self { id: unique_id(), attributes, to_token_stream, mnemonic, num_outputs, inputs, regions }
    }

    pub fn mnemonic(&self) -> &str {
        &self.mnemonic
    }

    pub fn output(&self, index: usize) -> Value {
        assert!(index < self.num_outputs);
        Value { parent: self.id, index }
    }
}

impl ToTokens for Operation {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend((self.to_token_stream)(self));
    }
}

impl std::fmt::Debug for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.num_outputs > 0 {
            for (index, output) in (0..self.num_outputs).map(|index| self.output(index)).enumerate() {
                if index == 0 {
                    write!(f, "{output:?}")?;
                } else {
                    write!(f, ", {output:?}")?;
                }
            }
            write!(f, " = ")?;
        }
        write!(f, "{}", self.mnemonic)?;
        if !self.attributes.is_empty() {
            write!(f, "[{}]", self.attributes.join(", "))?;
        }
        for input in &self.inputs {
            write!(f, " {input:?}")?;
        }
        for region in &self.regions {
            if f.alternate() {
                write!(f, " {region:#?}")?;
            } else {
                write!(f, " {region:?}")?;
            }
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct Value {
    parent: u64,
    index: usize,
}

impl Value {
    pub fn new_standalone() -> Self {
        Self { parent: unique_id(), index: 0 }
    }

    pub fn to_ident(&self) -> syn::Ident {
        format_ident!("v{}_{}", self.parent, self.index)
    }
}

impl ToTokens for Value {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.to_ident().to_tokens(tokens);
    }
}

impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "%{}_{}", self.parent, self.index)
    }
}

#[cfg(test)]
macro_rules! assert_matches {
    ($operation:expr, $pattern:expr) => {
        match crate::ssa_ir::ir::matches($operation, $pattern) {
            Ok(_) => (),
            Err(message) => assert!(false, "{message}"),
        }
    };
}

#[cfg(test)]
pub(crate) use assert_matches;

#[cfg(test)]
pub fn matches(operation: impl AsRef<str>, pattern: impl AsRef<str>) -> Result<(), String> {
    let op_tokens = tokenize(&operation)?;
    let pat_tokens = tokenize(&pattern)?;
    let mut sym_pat_to_op = HashMap::new();
    let mut sym_op_to_pat = HashMap::new();
    for (token_idx, (op_token, pat_token)) in op_tokens.iter().zip(pat_tokens.iter()).enumerate() {
        use Token::*;
        let is_content_same = match (op_token, pat_token) {
            (Empty, Empty) => Ok(()),
            (Pipe, Pipe) => Ok(()),
            (BracketOpen, BracketOpen) => Ok(()),
            (BracketClose, BracketClose) => Ok(()),
            (Symbol(o), Symbol(p)) if o == p => Ok(()),
            (Value(o), Value(p)) => {
                if sym_pat_to_op.get(p) == Some(&o) && sym_op_to_pat.get(o) == Some(&p) {
                    Ok(())
                } else if sym_pat_to_op.get(p) == None && sym_op_to_pat.get(o) == None {
                    sym_pat_to_op.insert(p, o);
                    sym_op_to_pat.insert(o, p);
                    Ok(())
                } else {
                    Err((token_idx, &Token::Value(sym_pat_to_op.get(p).unwrap_or(&p).to_string())))
                }
            }
            (_, token) => Err((token_idx, token)),
        };
        if let Err((token_idx, expected)) = is_content_same {
            let start_idx = std::cmp::max(6, token_idx) - 6;
            let matching_section = op_tokens[start_idx..token_idx].iter().join(" ");
            let following_op_section =
                op_tokens[token_idx + 1..std::cmp::min(op_tokens.len(), token_idx + 6)].iter().join(" ");
            let following_pat_section =
                pat_tokens[token_idx + 1..std::cmp::min(pat_tokens.len(), token_idx + 6)].iter().join(" ");
            let found = op_tokens[token_idx].to_string();
            let padding = String::from_iter(std::iter::repeat(' ').take(matching_section.len()));
            let caret = String::from_iter(std::iter::repeat('^').take(found.len()));
            let message = format!(
                "operation does not match pattern:\n   expected: {matching_section} {expected} {following_pat_section}\n   found:    {matching_section} {found} {following_op_section}\n             {padding} {caret}",
            );
            return Err(message);
        }
    }
    if op_tokens.len() != pat_tokens.len() {
        return Err(format!(
            "operation differs from pattern in length\noperation:\n{}\npattern:\n{}",
            operation.as_ref(),
            pattern.as_ref()
        ));
    }
    Ok(())
}

#[cfg(test)]
#[derive(Debug, Clone)]
enum Token {
    Empty,
    Pipe,
    BracketOpen,
    BracketClose,
    Symbol(String),
    Value(String),
}

#[cfg(test)]
impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Empty => write!(f, ""),
            Token::Pipe => write!(f, "|"),
            Token::BracketOpen => write!(f, "["),
            Token::BracketClose => write!(f, "]"),
            Token::Symbol(s) => write!(f, "{s}"),
            Token::Value(s) => write!(f, "%{s}"),
        }
    }
}

#[cfg(test)]
fn tokenize(pattern: impl AsRef<str>) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    for char in pattern.as_ref().chars() {
        match char {
            ' ' | '\t' | '\n' | ',' => tokens.push(Token::Empty),
            '|' => tokens.push(Token::Pipe),
            '[' => tokens.push(Token::BracketOpen),
            ']' => tokens.push(Token::BracketClose),
            '%' => tokens.push(Token::Value(String::new())),
            ch => match tokens.last_mut() {
                Some(Token::Symbol(s)) => s.push(ch),
                Some(Token::Value(s)) => s.push(ch),
                _ => tokens.push(Token::Symbol(ch.into())),
            },
        };
    }
    tokens.retain(|token| !matches!(token, Token::Empty));
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    #[test]
    fn matches_same_value() {
        let operation = "%a %a";
        let pattern = "%a %a";
        assert_matches!(operation, pattern);
    }

    #[test]
    #[should_panic]
    fn matches_different_value() {
        let operation = "%a %a";
        let pattern = "%a %b";
        assert_matches!(operation, pattern);
    }

    #[test]
    #[should_panic]
    fn matches_different_value_reverse() {
        let operation = "%a %b";
        let pattern = "%a %a";
        assert_matches!(operation, pattern);
    }

    #[test]
    fn matches_same_symbol() {
        let operation = "a";
        let pattern = "a";
        assert_matches!(operation, pattern);
    }

    #[test]
    #[should_panic]
    fn matches_different_symbol() {
        let operation = "a";
        let pattern = "b";
        assert_matches!(operation, pattern);
    }
}
