#![allow(unused)]

use sorbit::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Empty {}

#[derive(Debug, Serialize, Deserialize)]
struct Unconstrained {
    a: u8,
    b: u8,
}

#[derive(Debug, Serialize, Deserialize)]
struct DirectFields {
    #[sorbit_layout(offset = 2)]
    a: u8,
    #[sorbit_layout(align = 4)]
    b: u8,
}
