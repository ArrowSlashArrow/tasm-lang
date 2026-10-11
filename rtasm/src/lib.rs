//! Library half of `tasmc`. The compiler binary (`main.rs`) and external tooling
//! such as the language server (`tasm-lsp/`) share the lexer, the core types and
//! the instruction spec through this crate.
#![warn(clippy::std_instead_of_core, clippy::std_instead_of_alloc)]

extern crate alloc;

pub mod core;
pub mod instr;
pub mod lexer;
