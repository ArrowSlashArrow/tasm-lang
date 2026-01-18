use std::{error::Error, fmt::Display};

use gdlib::gdobj::GDObject;

pub const ENTRY_POINT: &str = "_start";

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
    pub handler_fn: fn(Vec<TasmValue>) -> GDObject,
}

#[derive(Debug)]
pub enum TasmParseError {
    InvalidInstruction((String, usize)),
    InvalidArguments((String, usize)),
    NoEntryPoint,
    InvalidNumber(String),
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
            Self::InvalidArguments((reason, line)) => {
                write!(f, "Invalid arguments on line {line}: {reason}")
            }
            Self::InvalidNumber((why)) => {
                write!(f, "Invalid number. {why}")
            }
        }
    }
}

#[derive(Debug)]
pub enum TasmValue {
    Counter(i16),
    Timer(i16),
    Number(f64),
    /// Default
    String(String),
}

#[derive(PartialEq)]
pub enum TasmValueType {
    Counter,
    Timer,
    Float,
    Int,
    String,
}

impl TasmValue {
    pub fn to_value(s: &str) -> Result<Self, TasmParseError> {
        let mut iter = s.clone().chars();
        let pref = iter.next().unwrap();
        let remaining_i16 = iter.into_iter().collect::<String>().parse::<i16>();
        if pref == 'T'
            && let Ok(n) = remaining_i16
        {
            return Ok(Self::Timer(n));
        } else if pref == 'C'
            && let Ok(n) = remaining_i16
        {
            return Ok(Self::Counter(n));
        } else if let Ok(n) = s.parse::<f64>() {
            if !n.is_finite() {
                return Err(TasmParseError::InvalidNumber(
                    "Infinity not allowed.".into(),
                ));
            } else if n.is_nan() {
                return Err(TasmParseError::InvalidNumber("NaN not allowed.".into()));
            }
            return Ok(Self::Number(n));
        } else {
            Ok(Self::String(s.into()))
        }
    }

    pub fn get_type(&self) -> TasmValueType {
        match self {
            Self::Counter(_) => TasmValueType::Counter,
            Self::Timer(_) => TasmValueType::Timer,
            Self::Number(f) => {
                if f.fract() == 0.0 {
                    TasmValueType::Int
                } else {
                    TasmValueType::Float
                }
            }
            Self::String(_) => TasmValueType::String,
        }
    }
}
