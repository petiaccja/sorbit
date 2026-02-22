use std::collections::HashMap;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use petgraph::{algo::toposort, graph::DiGraph};
use proc_macro2::TokenStream;
use quote::ToTokens;
use quote::format_ident;
use quote::quote;

//------------------------------------------------------------------------------
// Operation trait
//------------------------------------------------------------------------------

pub trait Operation {
    fn name(&self) -> &str;
    fn id(&self) -> Id;
    fn is_terminator(&self) -> bool {
        false
    }
    fn inputs(&self) -> Vec<Value>;
    fn outputs(&self) -> Vec<Value>;
    fn regions(&self) -> Vec<&Region>;
    fn attributes(&self) -> Vec<String>;
    fn to_token_stream(&self) -> TokenStream;
    fn to_string(&self, alternate: bool) -> String {
        let outputs = self.outputs().iter().map(|output| format!("{output}")).collect::<Vec<_>>().join(", ");
        let inputs = self.inputs().iter().map(|input| format!("{input}")).collect::<Vec<_>>().join(", ");
        let attributes = self.attributes().join(", ");
        let regions = self
            .regions()
            .iter()
            .map(|region| if alternate { format!("{region:#}") } else { format!("{region}") })
            .collect::<Vec<_>>()
            .join(" ");
        let mut s = String::new();
        if !outputs.is_empty() {
            s.push_str(&format!("{outputs} = "));
        };
        s.push_str(self.name());
        if !attributes.is_empty() {
            s.push_str(&format!(" [{attributes}]"));
        };
        if !inputs.is_empty() {
            s.push_str(&format!(" {inputs}"));
        };
        if !regions.is_empty() {
            s.push_str(&format!(" {regions}"));
        }
        s
    }
}

impl ToTokens for dyn Operation {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(self.to_token_stream());
    }
}

//------------------------------------------------------------------------------
// Id structure
//------------------------------------------------------------------------------

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

fn next_id() -> usize {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(usize);

impl Id {
    pub fn new() -> Self {
        Self(next_id())
    }

    pub fn value(&self, index: usize) -> Value {
        Value { owner: *self, index }
    }
}

impl std::fmt::Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

//------------------------------------------------------------------------------
// Value structure
//------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Value {
    pub owner: Id,
    pub index: usize,
}

impl Value {
    pub fn new(owner: Id, index: usize) -> Self {
        Self { owner, index }
    }

    pub fn to_ident(&self) -> syn::Ident {
        format_ident!("v{}_{}", self.owner.0, self.index)
    }
}

impl ToTokens for Value {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.to_ident().to_tokens(tokens);
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "%{}_{}", self.owner, self.index)
    }
}

//------------------------------------------------------------------------------
// Region structure
//------------------------------------------------------------------------------

pub struct Region {
    #[allow(unused)]
    id: Id,
    arguments: Vec<Value>,
    operations: Vec<Box<dyn Operation>>,
}

impl Region {
    pub fn new(num_arguments: usize) -> Self {
        let id = Id::new();
        let arguments: Vec<_> = (0..num_arguments).map(|index| Value::new(id, index)).collect();
        Self { id, arguments, operations: Vec::new() }
    }

    pub fn arguments(&self) -> &[Value] {
        &self.arguments
    }

    pub fn push(&mut self, operation: impl Operation + 'static) -> Vec<Value> {
        if self.operations.last().is_some_and(|last| last.is_terminator()) {
            panic!("region has already been terminated")
        }
        let outputs = operation.outputs();
        self.operations.push(Box::new(operation) as Box<dyn Operation>);
        outputs
    }

    pub fn to_token_stream_formatted(&self, with_braces: bool) -> TokenStream {
        let mut graph = DiGraph::new();
        let mut id_to_node = HashMap::new();
        {
            let mut predecessor = None;
            for operation in &self.operations {
                let node = graph.add_node(operation);
                id_to_node.insert(operation.id(), node);
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
            for input in op.inputs() {
                if let Some(predecessor) = id_to_node.get(&input.owner) {
                    graph.add_edge(*predecessor, node, ());
                }
            }
        }

        match toposort(&graph, None) {
            Ok(order) => {
                let ops = order.iter().map(|op_idx| {
                    let op = graph.node_weight(*op_idx).unwrap();
                    let outputs = op.outputs();
                    if op.is_terminator() {
                        quote! { #op }
                    } else if outputs.is_empty() {
                        quote! { #op; }
                    } else if outputs.len() == 1 {
                        quote! { let #(#outputs),* = #op; }
                    } else {
                        quote! { let (#(#outputs),*) = #op; }
                    }
                });
                match with_braces {
                    true => quote! { { #(#ops)* } },
                    false => quote! { #(#ops)* },
                }
            }
            Err(cycle) => {
                syn::Error::new(proc_macro2::Span::call_site(), format!("cycle at node {}", cycle.node_id().index()))
                    .into_compile_error()
            }
        }
    }
}

impl std::fmt::Display for Region {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let arguments = self.arguments.iter().map(|arg| format!("{arg}")).collect::<Vec<_>>().join(", ");
        let prefix = if f.alternate() { "    " } else { "" };
        let operations = textwrap::indent(
            &self
                .operations
                .iter()
                .map(|operation| operation.to_string(f.alternate()))
                .collect::<Vec<_>>()
                .join("\n"),
            prefix,
        );
        if !arguments.is_empty() {
            write!(f, "|{arguments}| ")?;
        };
        write!(f, "{{\n{operations}\n}}")
    }
}

impl ToTokens for Region {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(self.to_token_stream_formatted(true));
    }
}
