use std::{error::Error, fmt::Display, num::ParseIntError};

use gdlib::{
    gdlevel::Level,
    gdobj::{GDObjConfig, GDObject},
};

pub const ENTRY_POINT: &str = "_start";
pub const INIT_ROUTINE: &str = "_init";
pub const GROUP_LIMIT: i16 = 9_999;

#[derive(Debug)]
pub struct Tasm {
    pub routines: Vec<Routine>,
}

#[derive(Debug, Default)]
pub struct Routine {
    pub ident: String,
    pub group: i16,
    pub instructions: Vec<Instruction>,
}

impl Routine {
    pub fn empty() -> Self {
        Routine {
            ident: String::new(),
            group: 0,
            instructions: vec![],
        }
    }

    pub fn group(mut self, group: i16) -> Self {
        self.group = group;
        self
    }

    pub fn ident(mut self, ident: &String) -> Self {
        self.ident = ident.into();
        self
    }

    pub fn add_instruction(&mut self, instr: Instruction) {
        self.instructions.push(instr);
    }
}

pub type HandlerReturn = Result<HandlerData, TasmParseError>;
pub type HandlerFn = fn(HandlerArgs) -> HandlerReturn;

pub struct HandlerArgs {
    /// Arguments to this function. e.g. Counter(C1), Number(2.5)
    pub args: Vec<TasmValue>,
    /// Config (specifically position and group) of the resulting object(s)
    pub cfg: GDObjConfig,
    /// Next available group to use for the objects
    pub curr_group: i16,
}

pub struct HandlerData {
    objects: Vec<GDObject>,
    // skip this amount of obj
    skip_spaces: i32,
    // extra used groups
    used_extra_groups: i16,
}

impl HandlerData {
    #[inline(always)]
    pub fn default() -> Self {
        Self {
            objects: vec![],
            skip_spaces: 1, // always advance one space
            used_extra_groups: 0,
        }
    }

    #[inline(always)]
    pub fn set_objects(mut self, objects: Vec<GDObject>) -> Self {
        self.objects = objects;
        self
    }

    pub fn from_objects(objects: Vec<GDObject>) -> Self {
        let mut new = Self::default();
        new.objects = objects;
        new
    }

    #[inline(always)]
    pub fn skip_spaces(mut self, spaces: i32) -> Self {
        self.skip_spaces = spaces;
        self
    }

    #[inline(always)]
    pub fn extra_groups(mut self, groups: i16) -> Self {
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
    pub handler_fn: HandlerFn,
}

#[derive(Debug)]
pub enum TasmParseError {
    InvalidInstruction((String, usize)),
    InvalidArguments((String, usize)),
    BadToken((String, usize)),
    NoEntryPoint,
    InvalidNumber(String),
    InvalidGroup(ParseIntError),
    ExceedsGroupLimit,
    InitRoutineSpawnError(usize),
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
                write!(f, "Bad command: {cmd} on line {}", line + 1)
            }
            Self::NoEntryPoint => write!(f, "No entry point found. ({ENTRY_POINT} routine)"),
            Self::InvalidArguments((reason, line)) => {
                write!(f, "Invalid arguments on line {}: {reason}", line + 1)
            }
            Self::BadToken((tok, line)) => {
                write!(
                    f,
                    "Bad token on line {}: {tok}. If this is an instruction, it must be indented.",
                    line + 1
                )
            }
            Self::InitRoutineSpawnError(line) => {
                write!(
                    f,
                    "Spawning the initialiser routine is not allowed (line {}).",
                    line + 1
                )
            }
            Self::InvalidNumber(why) => {
                write!(f, "Invalid number. {why}")
            }
            Self::InvalidGroup(why) => {
                write!(f, "Invalid number. {why}")
            }
            Self::ExceedsGroupLimit => {
                write!(f, "Input file exceeds group limit of {GROUP_LIMIT} groups.")
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum TasmValue {
    Counter(i16),
    Timer(i16),
    Number(f64),
    Group(i16),
    /// Default
    String(String),
}

#[derive(PartialEq, Debug)]
pub enum TasmValueType {
    Primitive(TasmPrimitive),
    List(TasmPrimitive),
}

#[derive(PartialEq, Debug)]
pub enum TasmPrimitive {
    Item,
    Number, // also a float.
    Int,
    Group,
    String,
}

impl TasmValue {
    pub fn to_value(s: &str) -> Result<Self, TasmParseError> {
        let mut iter = s.chars();
        let pref = iter.next().unwrap();
        let postf = s.chars().last().unwrap();
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
            // sanity checks
            if !n.is_finite() {
                return Err(TasmParseError::InvalidNumber(
                    "Infinity not allowed.".into(),
                ));
            } else if n.is_nan() {
                return Err(TasmParseError::InvalidNumber("NaN not allowed.".into()));
            }

            // if this is an int and postfixed by 'g', consider it a group literal
            if postf == 'g' {
                // chop off one char
                let mut chopped = s.to_string();
                chopped.pop();
                match chopped.parse::<i16>() {
                    Ok(n) => Ok(Self::Group(n)),
                    Err(e) => Err(TasmParseError::InvalidGroup(e)),
                }
            } else {
                Ok(Self::Number(n))
            }
        } else {
            Ok(Self::String(s.into()))
        }
    }

    pub fn get_type(&self) -> TasmPrimitive {
        match self {
            Self::Counter(_) => TasmPrimitive::Item,
            Self::Timer(_) => TasmPrimitive::Item,
            Self::Number(_) => TasmPrimitive::Number,
            Self::Group(_) => TasmPrimitive::Group,
            Self::String(_) => TasmPrimitive::String,
        }
    }

    pub fn is_int(&self) -> bool {
        match self {
            Self::Number(n) => n.fract() == 0.0,
            _ => false,
        }
    }

    pub fn to_int(&self) -> Option<i32> {
        match self {
            Self::Number(n) => Some(*n as i32),
            _ => None,
        }
    }

    pub fn to_float(&self) -> Option<f64> {
        match self {
            Self::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn to_counter_id(&self) -> Option<i16> {
        match self {
            Self::Counter(n) => Some(*n),
            _ => None,
        }
    }

    pub fn to_timer_id(&self) -> Option<i16> {
        match self {
            Self::Timer(n) => Some(*n),
            _ => None,
        }
    }

    pub fn to_group_id(&self) -> Option<i16> {
        match self {
            Self::Group(n) => Some(*n),
            _ => None,
        }
    }

    pub fn to_string(&self) -> Option<String> {
        match self {
            Self::String(s) => Some(s.to_owned()),
            _ => None,
        }
    }
}

impl Tasm {
    pub fn from_routines(routines: Vec<Routine>) -> Self {
        Self { routines }
    }

    pub fn handle_routines(&mut self) -> Result<Level, Vec<TasmParseError>> {
        let mut errors: Vec<TasmParseError> = vec![];

        let mut level = Level::new("tasm level", "tasm", None, None);
        let mut curr_group = 0i16;

        let mut obj_pos = 0.0;

        for routine in self.routines.iter() {
            curr_group += 1;
            if curr_group > GROUP_LIMIT {
                errors.push(TasmParseError::ExceedsGroupLimit);
                break;
            }

            // starting position of objects: (15, 75 + curr_group * 15)
            for instr in routine.instructions.iter() {
                let cfg = if routine.ident == INIT_ROUTINE {
                    if let InstrType::Init = instr._type {
                        GDObjConfig::default()
                    } else {
                        curr_group -= 1;
                        GDObjConfig::default()
                            .pos(-15.0 - obj_pos, 75.0 + (curr_group as f64) * 15.0)
                    }
                } else {
                    GDObjConfig::default()
                        .pos(15.0 + obj_pos, 75.0 + (curr_group as f64) * 15.0)
                        .groups([curr_group])
                };

                let handler = instr.handler_fn;
                let args = HandlerArgs {
                    args: instr.args.clone(),
                    cfg: cfg.spawnable(true).multitrigger(true),
                    curr_group,
                };

                let data = match handler(args) {
                    Ok(data) => data,
                    Err(e) => {
                        errors.push(e);
                        continue;
                    }
                };
                for obj in data.objects.into_iter() {
                    level.add_object(obj);
                }
                let skip_spaces = data.skip_spaces;
                curr_group += data.used_extra_groups;
                obj_pos += skip_spaces as f64;
            }
        }

        if errors.len() > 0 {
            Err(errors)
        } else {
            Ok(level)
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

pub fn get_instr_type(ident: &str) -> Option<InstrType> {
    match ident {
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
