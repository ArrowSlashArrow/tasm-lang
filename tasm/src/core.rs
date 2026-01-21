use std::{error::Error, fmt::Display};

use gdlib::gdobj::{GDObjConfig, GDObject};

pub const ENTRY_POINT: &str = "_start";
pub const INIT_ROUTINE: &str = "_init";
pub const GROUP_LIMIT: i16 = 9_999;

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

    pub fn add_instruction(&mut self, instr: Instruction) {
        self.instructions.push(instr);
    }
}

pub type HandlerReturn = Result<HandlerData, TasmParseError>;

pub struct HandlerData {
    object: GDObject,
    skip_spaces: i32,
    used_extra_groups: i32,
}

impl HandlerData {
    pub fn object(object: GDObject) -> Self {
        Self {
            object,
            skip_spaces: 0,
            used_extra_groups: 0,
        }
    }

    pub fn skip_spaces(mut self, spaces: i32) -> Self {
        self.skip_spaces = spaces;
        self
    }

    pub fn extra_groups(mut self, groups: i32) -> Self {
        self.used_extra_groups = groups;
        self
    }
}

#[derive(Debug)]
pub struct Instruction {
    pub ident: String,
    pub _type: InstrType,
    pub line_number: usize,
    pub args: Vec<TasmValue>,
    pub handler_fn: fn(Vec<TasmValue>) -> HandlerReturn,
}

#[derive(Debug)]
pub enum TasmParseError {
    InvalidInstruction((String, usize)),
    InvalidArguments((String, usize)),
    NoEntryPoint,
    InvalidNumber(String),
    InconsistentIndent((String, usize, usize, usize)),
    ExceedsGroupLimit,
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
                write!(f, "Bad command: {cmd} at line {line}")
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
            Self::InvalidNumber(why) => {
                write!(f, "Invalid number. {why}")
            }
            Self::ExceedsGroupLimit => {
                write!(f, "Input file exceeds group limit of {GROUP_LIMIT} groups.")
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
    Primitive(TasmPrimitive),
    List(TasmPrimitive),
}

#[derive(PartialEq)]
pub enum TasmPrimitive {
    Counter,
    Timer,
    Int,
    Float,
    String,
}

impl TasmValue {
    pub fn to_value(s: &str) -> Result<Self, TasmParseError> {
        let mut iter = s.chars();
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

    pub fn get_type(&self) -> TasmPrimitive {
        match self {
            Self::Counter(_) => TasmPrimitive::Counter,
            Self::Timer(_) => TasmPrimitive::Timer,
            Self::Number(f) => {
                if f.fract() == 0.0 {
                    TasmPrimitive::Int
                } else {
                    TasmPrimitive::Float
                }
            }
            Self::String(_) => TasmPrimitive::String,
        }
    }
}

impl Tasm {
    pub fn from_routines(routines: Vec<Routine>) -> Self {
        Self { routines }
    }

    pub fn handle_routines(&mut self) -> Result<(), Vec<TasmParseError>> {
        let mut errors: Vec<TasmParseError> = vec![];

        let mut curr_group = 0i16;
        for routine in self.routines.iter() {
            curr_group += 1;
            if curr_group > GROUP_LIMIT {
                errors.push(TasmParseError::ExceedsGroupLimit);
                break;
            }

            // starting position of objects: (15, 75 + curr_group * 15)
            let mut obj_config = GDObjConfig::default()
                .pos(15.0, 75.0 + (curr_group as f64) * 15.0)
                .groups([curr_group]);

            for instr in routine.instructions.iter() {}
        }

        if errors.len() > 0 {
            Err(errors)
        } else {
            Ok(())
        }
    }
}

#[derive(Debug)]
pub enum InstrType {
    Arithmetic, // any instruction that deals specifically with operations between counters
    Init,       // any instruction that can only go into the _init routine.
    Memory,     // any instruction that requires/interfaces with memory
    Timer,      // any instruction that interacts with timers non-arithmetically.
    Spawner, // any instruction that spawns a group. comparison instructions fall into this category.
    Stopper, // any instruction that stops a group's execution (RET, STOP)
    Wait,    // any instruction that waits (NOP, WAIT)
    Debug, // any instruction that is only used by the emulator, and ignored when parsing to GD objects.
}

pub fn get_instr_type(instr: &str) -> Option<InstrType> {
    match instr {
        "SPAWN" | "SRAND" | "FRAND" | "SE" | "SNE" | "SL" | "SLE" | "SG" | "SGE" | "FE" | "FNE"
        | "FL" | "FLE" | "FG" | "FGE" => Some(InstrType::Spawner),
        "ADD" | "SUB" | "MUL" | "DIV" | "FLDIV" | "MOV" => Some(InstrType::Arithmetic),
        "INITMEM" | "MALLOC" | "FMALLOC" | "PERS" | "DISPLAY" | "IOBLOCK" => Some(InstrType::Init),
        "MFUNC" | "MREAD" | "MWRITE" | "MPTR" | "MRESET" => Some(InstrType::Memory),
        "NOP" | "WAIT" => Some(InstrType::Wait),
        "TSPAWN" | "TSTART" | "TSTOP" => Some(InstrType::Timer),
        "RET" | "STOP" => Some(InstrType::Stopper),
        "BREAKPOINT" => Some(InstrType::Debug),
        _ => None,
    }
}
