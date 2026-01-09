//! Lexer for tasm.
//!
//! # tasm format
//! ## Instruction
//! An instruction is a 1:1 correspondence to a specific action that can be done
//! by a trigger object in Geometry Dash. For example, adding 1 to counter C1.
//! It is denoted by an instruction command, separated by a space,
//! followed by comma-separated arguments. Example: `ADD C1, 1`
//! All instructions must be inside of a routine, and indiented by 4 spaces.
//!
//! ## Routine
//! A routine is a container for sequential instructions. All instructions in a
//! given routine will be placed as objects in order from left to right,
//! separated by 1 unit. It is denoted with the routine identifier, followed by a colon.
//! Any line that does not have any indentation and ends with a colon is assumed to be a routine.
//!
//! ### `_init` routine
//! The _init routine is a special routine which is parsed and may contain special
//! initialization instructions that may not be used elsewhere,
//! such as `MALLOC`, `DISPLAY`, or `PERS`. These commands do not correspond to actual
//! objects in GD, they are instead used to tell the compiler to place certain structures
//! required for the program to function, such as the memory cell for `MALLOC`.
//!
//! ### `_start` routine
//! The _start routine is the entry point of the program, and is assigned group 1
//! unless a group offset > 0 is specified. An IO-block will be automatically placed
//! for this group, unless otherwise specified. If the compiled program will be used
//! as part of the larger project, then the _start routine should be considered
//! the entry point for the program.
//!
//! ## Comment
//! A comment is completely ignored during parsing. It is anything that follows a `;`.
//!
//! # Parsing
//! All lines are stripped for whitespace on the right-hand side before tokenisation.
//! Following that, all routines are indexed and their group determined.
//! Finally, all instructions are parsed in each group sequentially.

use std::{error::Error, fmt::Display, path::PathBuf};

pub struct Tasm {
    pub routines: Vec<Routine>,
}

pub struct Routine {
    pub instructions: Vec<Instruction>,
}

pub struct Instruction {
    pub ident: String,
    pub args: Vec<TasmValue>,
}

pub enum TasmValue {
    Counter(i16),
    Timer(i16),
    Number(f64),
    /// Default
    String(String),
}

#[derive(Debug)]
pub enum TasmParseError {
    BadCommand((String, usize, usize)),
}

impl Error for TasmParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        return None;
    }
}

impl Display for TasmParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadCommand((cmd, line, char)) => {
                write!(f, "Bad command `{cmd}` at line {line}, position {char}")
            }
        }
    }
}

pub fn parse_file<T: AsRef<PathBuf>>(infile: T) -> Result<Tasm, TasmParseError> {
    Ok(Tasm { routines: vec![] })
}
