//! Lexer for tasm.
//!
//! # tasm format
//! ## Instruction
//! An instruction is a 1:1 correspondence to a specific action that can be done
//! by a trigger object in Geometry Dash. For example, adding 1 to counter C1.
//! It is denoted by an instruction command, separated by a space,
//! followed by comma-separated arguments. Example: `ADD C1, 1`
//! All instructions must be inside of a routine, and should be indented by
//! at least 2 or 4 spaces.
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

use std::{error::Error, fmt::Display};

pub const ENTRY_POINT: &str = "_start";

// todo: dynamic load from commands file
pub const INSTRUCTIONS: &[&str] = &[
    "INITMEM",
    "MALLOC",
    "FMALLOC",
    "MFUNC",
    "MREAD",
    "MWRITE",
    "MPTR",
    "MRESET",
    "MOV",
    "DISPLAY",
    "IOBLOCK",
    "BREAKPOINT",
    "NOP",
    "SPAWN",
    "PERS",
    "ADD",
    "SUB",
    "MUL",
    "DIV",
    "FLDIV",
    "SE",
    "SNE",
    "SL",
    "SLE",
    "SG",
    "SGE",
    "FE",
    "FNE",
    "FL",
    "FLE",
    "FG",
    "FGE",
];

#[derive(Debug)]
pub struct Tasm {
    pub routines: Vec<Routine>,
}

#[derive(Debug)]
pub struct Routine {
    pub ident: String,
    pub instructions: Vec<Instruction>,
}

impl Routine {
    pub fn empty() -> Self {
        Routine {
            ident: String::new(),
            instructions: vec![],
        }
    }

    pub fn add_isntruction(&mut self, instr: Instruction) {
        self.instructions.push(instr);
    }
}

#[derive(Debug)]
pub struct Instruction {
    pub ident: String,
    pub args: Vec<TasmValue>,
}

#[derive(Debug)]
pub enum TasmValue {
    Counter(i16),
    Timer(i16),
    Number(f64),
    /// Default
    String(String),
}

impl TasmValue {
    pub fn to_value(s: &str) -> Self {
        let mut iter = s.clone().chars();
        let pref = iter.next().unwrap();
        let remaining_i16 = iter.into_iter().collect::<String>().parse::<i16>();
        if pref == 'T'
            && let Ok(n) = remaining_i16
        {
            return Self::Timer(n);
        } else if pref == 'C'
            && let Ok(n) = remaining_i16
        {
            return Self::Counter(n);
        } else if let Ok(n) = s.parse::<f64>() {
            return Self::Number(n);
        } else {
            Self::String(s.into())
        }
    }
}

#[derive(Debug)]
pub enum TasmParseError {
    InvalidInstruction((String, usize)),
    NoEntryPoint,
    InconsistentIndent((String, usize, usize, usize)),
}

impl Error for TasmParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

impl Display for TasmParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInstruction((cmd, line)) => {
                write!(f, "Bad command `{cmd}` at line {line}")
            }
            Self::NoEntryPoint => write!(f, "No entry point found. ({ENTRY_POINT} routine)"),
            Self::InconsistentIndent((reason, line, expected, got)) => {
                write!(
                    f,
                    "{reason} on line {line}. Expected {expected} spaces, got {got} spaces."
                )
            }
        }
    }
}

pub fn parse_file<T: AsRef<str>>(f_str: T) -> Result<Tasm, Vec<TasmParseError>> {
    let file = f_str.as_ref();

    let mut routines = vec![];
    let mut seen_entry_point = false;

    let mut curr_routine = Routine::empty();

    let mut errors = vec![];

    let mut indent_size = 0;
    for (idx, raw_line) in file.lines().into_iter().enumerate() {
        let line = raw_line.split(";").next().unwrap();
        if raw_line.trim() == "" {
            continue;
        }

        if !line.starts_with(' ') {
            // no space, check for routine identifier.
            let mut strip = line.trim().to_string();
            if strip.ends_with(':') && !strip.contains(' ') {
                // now we are certain that this is a routine ident
                strip.pop();
                let routine_ident = strip;
                if routine_ident == ENTRY_POINT {
                    seen_entry_point = true;
                }

                // save current routine and load this one + reset indent size
                routines.push(curr_routine);
                indent_size = 0;
                curr_routine = Routine::empty();
            }
        } else {
            let indentation = line.chars().take_while(|c| *c == ' ').count();
            // check or set indent size
            if indent_size == 0 {
                // first line after routine ident
                indent_size = indentation;
            } else if indentation != indent_size {
                errors.push(TasmParseError::InconsistentIndent((
                    "Inconsistent indentation amount".into(),
                    idx + 1,
                    indentation,
                    indent_size,
                )));
            }

            // 0 is the cmd (MUST be there), 1.. are args (optional)
            let mut instr_split = line.trim().clone().split(" ");
            // this should never be blank, due to all blank lines being skipped.
            let instr = instr_split.next().unwrap();
            if !INSTRUCTIONS.contains(&instr) {
                errors.push(TasmParseError::InvalidInstruction((instr.into(), idx)));
            }
            let args = instr_split.into_iter().map(TasmValue::to_value).collect();

            curr_routine.add_isntruction(Instruction {
                ident: instr.into(),
                args,
            });
        }
    }

    if !seen_entry_point {
        errors.push(TasmParseError::NoEntryPoint);
    }

    if errors.len() > 0 {
        Err(errors)
    } else {
        Ok(Tasm { routines })
    }
}
