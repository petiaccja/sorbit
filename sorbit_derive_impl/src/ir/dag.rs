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

    #[allow(unused)]
    pub fn operations(&self) -> &[Operation] {
        &self.operations
    }

    pub fn num_outputs(&self) -> usize {
        if let Some(last_op) = self.operations.last() {
            if last_op.mnemonic() == "yield" { last_op.inputs.len() } else { 0 }
        } else {
            0
        }
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

    #[allow(unused)]
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
    #[allow(unused)] // Only used in tests at the moment.
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
