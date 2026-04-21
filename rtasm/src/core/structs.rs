use gdlib::gdobj::{GDObjConfig, GDObject, Item};

use crate::core::{
    HandlerFn,
    consts::GROUP_LIMIT,
    error::{ParseErrorType, TasmError},
    flags::Flag,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub enum InstrType {
    Arithmetic, // any instruction that performs mathematical operations with counters
    Init,       // any instruction that can only go into the _init routine.
    Memory,     // any instruction that requires/interfaces with memory
    Timer,      // any instruction that interacts with timers non-arithmetically.
    Spawner, // any instruction that spawns a group. comparison instructions fall into this category.
    Process, // any instruction that modifies the process flow (PAUSE, RESUME, STOP)
    Wait,    // any instruction that waits (NOP, WAIT)
    Debug, // any instruction that is only used by the emulator, and ignored when parsing to GD objects.
}

pub fn get_instr_type(ident: &str) -> Option<InstrType> {
    match ident {
        "SPAWN" | "SRAND" | "FRAND" | "SE" | "SNE" | "SL" | "SLE" | "SG" | "SGE" | "FE" | "FNE"
        | "FL" | "FLE" | "FG" | "FGE" => Some(InstrType::Spawner),
        "ADD" | "SUB" | "ADDM" | "SUBM" | "ADDD" | "SUBD" | "MUL" | "DIV" | "FLDIV" | "MOV" => {
            Some(InstrType::Arithmetic)
        }
        "INITMEM" | "MALLOC" | "FMALLOC" | "PERS" | "DISPLAY" | "IOBLOCK" | "ALIAS" => {
            Some(InstrType::Init)
        }
        "MFUNC" | "MREAD" | "MWRITE" | "MPTR" | "MRESET" => Some(InstrType::Memory),
        "NOP" | "WAIT" => Some(InstrType::Wait),
        "TSPAWN" | "TSTART" | "TSTOP" => Some(InstrType::Timer),
        "STOP" | "PAUSE" | "RESUME" => Some(InstrType::Process),
        "BREAKPOINT" => Some(InstrType::Debug),
        _ => None,
    }
}

#[derive(Debug, Clone)]
pub enum TasmValue {
    Counter(i16),
    Timer(i16),
    GDItem(Item),
    Number(f64),
    Group(i16),
    Alias(BuiltinAlias), // use ident instead of alias type
    /// Default
    String(String),
}

#[derive(Debug, Clone)]
pub struct Alias {
    pub ident: String,
    pub value: TasmValue,
}

impl Alias {
    pub fn to_alias(s: &str, v: TasmValue) -> Self {
        Self {
            ident: s.to_owned(),
            value: v,
        }
    }

    pub fn get_type(&self) -> TasmPrimitive {
        self.value.get_type()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltinAlias {
    MEMREG,
    PTRPOS,
    POINTS,
    ATTEMPTS,
    MAINTIME,
    MEMSIZE,
}

impl BuiltinAlias {
    pub fn from_ident(s: &str) -> Option<Self> {
        match s {
            "MEMREG" => Some(Self::MEMREG),
            "PTRPOS" => Some(Self::PTRPOS),
            "POINTS" => Some(Self::POINTS),
            "ATTEMPTS" => Some(Self::ATTEMPTS),
            "MAINTIME" => Some(Self::MAINTIME),
            "MEMSIZE" => Some(Self::MEMSIZE),
            _ => None,
        }
    }

    /// only works for builtin aliases
    pub fn get_type(&self) -> TasmPrimitive {
        match self {
            Self::MEMREG | Self::PTRPOS | Self::POINTS | Self::ATTEMPTS | Self::MAINTIME => {
                TasmPrimitive::Item
            }
            Self::MEMSIZE => TasmPrimitive::Number, // cannot be Int, since otherwise it isn;t recognized as a number
        }
    }
}

impl Default for TasmValue {
    fn default() -> Self {
        Self::Number(0.0)
    }
}

#[derive(PartialEq, Debug)]
pub enum TasmValueType {
    Primitive(TasmPrimitive),
    List(TasmPrimitive),
}

#[derive(PartialEq, Debug)]
pub enum TasmPrimitive {
    Item,
    Timer,  // subset of item
    Number, // also a float.
    Int,    // subset of number
    Group,
    String,
}

pub fn is_builtin_alias(s: &str) -> bool {
    matches!(
        s,
        "MEMREG" | "PTRPOS" | "MEMSIZE" | "POINTS" | "ATTEMPTS" | "MAINTIME"
    )
}

impl Aliases {
    pub fn get_value(&self, ident: BuiltinAlias) -> TasmValue {
        match ident {
            BuiltinAlias::MEMREG => self.memreg.clone(),
            BuiltinAlias::PTRPOS => TasmValue::Counter(self.ptrpos_id),
            BuiltinAlias::MEMSIZE => TasmValue::Number(self.memsize as f64),
            BuiltinAlias::ATTEMPTS => TasmValue::GDItem(Item::Attempts),
            BuiltinAlias::MAINTIME => TasmValue::GDItem(Item::MainTime),
            BuiltinAlias::POINTS => TasmValue::GDItem(Item::Points),
        }
    }
}

impl TasmValue {
    pub fn to_value(s: &str) -> Result<Self, (ParseErrorType, String)> {
        let mut iter = s.chars();
        let pref = match iter.next() {
            Some(c) => c,
            None => {
                // there's nothing in this string
                return Err((
                    ParseErrorType::TrailingComma,
                    "Got a 0-length string. Perhaps there is a trailing comma".into(),
                ));
            }
        };
        let remaining_i16 = iter.collect::<String>().parse::<i16>();

        // aliases are parsed before anything
        if let Some(a) = BuiltinAlias::from_ident(s) {
            // since values are parsed as lexing stage, only builtin ones are available
            // user-defined aliases are determined at semantic analysis
            Ok(Self::Alias(a))
        } else if (pref == 'T' || pref == 'C' || pref == 'g')
            && let Ok(id) = remaining_i16
        {
            // check that the ID is in range
            if id <= 0 || id > GROUP_LIMIT {
                return Err((
                    ParseErrorType::BadID,
                    format!("Item/group must be within the range [1, {GROUP_LIMIT}]"),
                ));
            }
            match pref {
                'T' => Ok(Self::Timer(id)),
                'C' => Ok(Self::Counter(id)),
                'g' => Ok(Self::Group(id)),
                _ => unreachable!(),
            }
        } else if let Ok(n) = s.parse::<f64>() {
            // sanity checks
            if !n.is_finite() {
                return Err((
                    ParseErrorType::InvalidNumber,
                    "Infinity is not allowed.".into(),
                ));
            } else if n.is_nan() {
                return Err((ParseErrorType::InvalidNumber, "NaN is not allowed.".into()));
            }

            Ok(Self::Number(n))
        } else {
            Ok(Self::String(s.into()))
        }
    }

    pub fn get_type(&self) -> TasmPrimitive {
        match self {
            Self::Counter(_) | Self::Timer(_) | Self::GDItem(_) => TasmPrimitive::Item,
            Self::Number(_) => TasmPrimitive::Number,
            Self::Group(_) => TasmPrimitive::Group,
            Self::String(_) => TasmPrimitive::String,
            Self::Alias(a) => a.get_type(),
        }
    }

    pub fn is_int(&self) -> bool {
        match self {
            Self::Number(n) => n.fract() == 0.0,
            Self::Alias(a) => a.get_type() == TasmPrimitive::Int,
            _ => false,
        }
    }

    pub fn is_timer(&self) -> bool {
        match self {
            Self::Timer(_) => true,
            Self::Alias(a) => a.get_type() == TasmPrimitive::Timer,
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

pub fn fits_arg_signature(args: &[TasmValue], sig: &[TasmValueType]) -> bool {
    // helper fn
    fn check_primitive(p: &TasmPrimitive, arg: &TasmValue) -> bool {
        // check if an int is required here
        // get_type returns `Number` for a `Number` even if it is an `Int`
        match p {
            TasmPrimitive::Int => arg.is_int(),
            TasmPrimitive::Timer => arg.is_timer(),
            // TasmPrimitive::String => true, // everything can be a string
            // ^ can't use this because TasmValue::to_string doesn't support it
            _ => &arg.get_type() == p,
        }
    }
    match sig.len() {
        0 => args.is_empty(),
        1 => match &sig[0] {
            TasmValueType::List(l_type) => {
                // check that all arguments are of the type in the list
                args.iter().all(|arg| check_primitive(l_type, arg))
            }
            TasmValueType::Primitive(p) => {
                if args.len() != 1 {
                    return false;
                }
                // check that the argument matches the specified type
                check_primitive(p, &args[0])
            }
        },
        n => {
            if args.len() != n {
                return false;
            }
            for (arg, t) in args.iter().zip(sig) {
                // skip list args, because we don't allow hybrid argsets
                match t {
                    TasmValueType::List(_) => continue,
                    TasmValueType::Primitive(p) => {
                        if !check_primitive(p, arg) {
                            // println!("{arg:?} is not {p:?}");
                            return false;
                        }
                    }
                }
            }

            true
        }
    }
}

#[derive(Clone, Default)]
pub struct HandlerArgs {
    /// Arguments to this function. e.g. Counter(C1), Number(2.5)
    pub args: Vec<TasmValue>,
    /// Config (specifically position and group) of the resulting object(s)
    pub cfg: GDObjConfig,
    /// Next available group to use for the objects
    pub curr_group: i16,
    /// Group of the pointer collision block
    pub ptr_group: i16,
    pub ptr_reset_group: i16,
    pub memreg: TasmValue,
    pub ptrpos_id: i16,
    pub displayed_items: usize,
    pub mem_end_counter: i16,
    pub routine_count: usize,
    pub mem_info: Option<MemInfo>,
    pub flags: Vec<Flag>,
    pub line: usize,
}

#[derive(Default)]
pub struct HandlerData {
    pub objects: Vec<GDObject>,
    // skip this amount of obj (default: 1)
    pub skip_spaces: i32,
    // extra used groups
    pub used_extra_groups: i16,
    // both set in malloc, keeps track of the groups of the respective objects
    pub ptr_group: i16,
    pub ptr_reset_group: i16,
    // set in display instr handler to tell the tasm object to bump displays counter
    pub added_item_display: bool,
    pub new_mem: Option<MemInfo>,
}

#[derive(Debug, Clone)]
pub struct MemInfo {
    pub _type: MemType,
    pub memreg: TasmValue,
    pub ptrpos: TasmValue,
    pub size: i16,
    pub read_group: i16,
    pub write_group: i16,
    pub start_counter_id: i16,
    pub line: usize, // where is was created
}

#[derive(Debug, Default)]
pub struct Tasm {
    pub routines: Vec<Routine>,
    pub errors: Vec<TasmError>,
    pub routine_data: Vec<(usize, String, i16, Vec<(usize, String)>)>,
    pub routine_group_map: Vec<(String, i16)>,
    pub group_offset: i16,
    pub has_entry_point: bool,
    pub lines: Vec<String>,
    pub mem_end_counter: i16,
    pub curr_group: i16,
    pub ptr_group: i16,
    pub ptr_reset_group: i16,
    pub displayed_items: usize,
    pub start_rtn_group: i16,
    pub mem_info: Option<MemInfo>,
    // aliases get resolved through the map:
    pub aliases: Aliases,
    pub logs_enabled: bool,
    pub release_mode: bool,
    pub defined_aliases: Vec<(String, String)>,
    pub fname: String,
}

/// Aliases lookup container
#[derive(Debug, Default, Clone)]
pub struct Aliases {
    pub memreg: TasmValue,
    pub ptrpos_id: i16,
    pub memsize: i16,
}

#[derive(Debug, Clone)]
pub enum MemType {
    Float,
    Int,
    LegacyFloat,
    LegacyInt,
}

#[derive(Debug, Default, Clone)]
pub struct Routine {
    pub ident: String,
    pub group: i16,
    pub instructions: Vec<Instruction>,
}

#[derive(Debug, Clone)]
pub struct Instruction {
    pub ident: String,
    pub _type: InstrType,
    pub line_number: usize,
    pub args: Vec<TasmValue>,
    pub flags: Vec<Flag>,
    pub handler_fn: HandlerFn,
    pub is_concurrent: bool,
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

impl HandlerData {
    #[inline(always)]
    pub fn default() -> Self {
        Self {
            skip_spaces: 1, // advance one space by default
            ..Default::default()
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

    #[inline(always)]
    pub fn added_item_display(mut self) -> Self {
        self.added_item_display = true;
        self
    }
}
