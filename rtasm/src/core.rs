use std::{error::Error, fmt::Display, num::ParseIntError};

use gdlib::{
    gdlevel::Level,
    gdobj::{GDObjConfig, GDObject, Item, ItemType, misc::text},
};

use crate::instr::{get_item_spec, ioblock};

pub const ENTRY_POINT: &str = "_start";
pub const INIT_ROUTINE: &str = "_init";
pub const GROUP_LIMIT: i16 = 9_999;

#[derive(Debug, Clone)]
pub enum MemType {
    Float,
    Int,
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
}

#[derive(Debug, Default)]
pub struct Tasm {
    pub routines: Vec<Routine>,
    pub errors: Vec<TasmParseError>,
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
}
#[macro_export]
macro_rules! verbose_log {
    ($this:expr, $($arg:tt)*) => {
        if $this.logs_enabled {
            println!($($arg)*);
        }
    };
}

/// Aliases lookup container
#[derive(Debug, Default)]
pub struct Aliases {
    pub memreg: TasmValue,
    pub ptrpos_id: i16,
    pub memsize: i16,
}

impl Aliases {
    pub fn get_value(&self, v: AliasType) -> TasmValue {
        match v {
            AliasType::MEMREG => self.memreg.clone(),
            AliasType::PTRPOS => TasmValue::Counter(self.ptrpos_id),
            AliasType::MEMSIZE => TasmValue::Number(self.memsize as f64),
            AliasType::ATTEMPTS => TasmValue::GDItem(Item::Attempts),
            AliasType::MAINTIME => TasmValue::GDItem(Item::MainTime),
            AliasType::POINTS => TasmValue::GDItem(Item::Points),
        }
    }
}

impl Tasm {
    pub fn handle_routines(&mut self, level_name: &String) -> Result<Level, Vec<TasmParseError>> {
        // clear errors
        self.errors = vec![];

        let spacing = match self.release_mode {
            true => 1.0,
            false => 30.0,
        };

        // setup state
        self.aliases.ptrpos_id = self.mem_end_counter;
        let mut level = Level::new(level_name, &"tasm".to_owned(), None, None);

        let routine_count = self.routines.len();
        self.curr_group = routine_count as i16 + self.group_offset + 1;

        for routine in self.routines.iter() {
            // setup position variables
            let mut obj_pos = 0.0;
            // subtracting from group offset ensures that high group IDs are still placed close to y=0
            let rtn_ypos = 75.0 + ((routine.group - self.group_offset) as f64) * 30.0;
            if self.curr_group > GROUP_LIMIT {
                self.errors.push(TasmParseError::ExceedsGroupLimit);
                break;
            }

            // keep track of entry group
            if routine.ident == ENTRY_POINT {
                self.start_rtn_group = routine.group;
            }

            // routine marker
            level.add_object(text(
                &GDObjConfig::new().pos(0.0, rtn_ypos).scale(0.6, 0.6),
                format!("{}: {}", routine.group, routine.ident),
                0,
            ));

            // starting position of objects: (15, 75 + curr_group * 15)
            for instr in routine.instructions.iter() {
                let mut instr_args = instr.args.clone();
                instr_args.iter_mut().for_each(|v| {
                    if let TasmValue::Alias(alias) = v {
                        *v = self.aliases.get_value(alias.clone())
                    }
                });

                // check that we are not accessing memory in init routine
                if instr._type == InstrType::Memory {
                    if routine.ident == INIT_ROUTINE {
                        self.errors
                            .push(TasmParseError::InitRoutineMemoryAccess(instr.line_number));
                        continue;
                    }
                    if let None = self.mem_info {
                        self.errors
                            .push(TasmParseError::NonexistentMemoryAccess(instr.line_number));
                        continue;
                    }
                }

                // check that any bad assignments aren't happening
                if instr._type == InstrType::Arithmetic {
                    // first argument is always the result
                    let counter_type = get_item_spec(&instr_args[0]).unwrap().get_type();
                    if counter_type == ItemType::Attempts || counter_type == ItemType::MainTime {
                        self.errors.push(TasmParseError::InvalidAssignment((
                            instr.line_number,
                            counter_type,
                        )));
                        continue;
                    }
                }

                let cfg = if routine.ident == INIT_ROUTINE {
                    if let InstrType::Init = instr._type {
                        GDObjConfig::default()
                    } else {
                        GDObjConfig::default().pos(-15.0 - obj_pos, rtn_ypos)
                    }
                } else {
                    GDObjConfig::default()
                        .pos(105.0 + obj_pos, rtn_ypos)
                        .groups([routine.group])
                };

                let handler = instr.handler_fn;
                let args = HandlerArgs {
                    args: instr_args,
                    cfg: cfg.spawnable(true).multitrigger(true),
                    curr_group: self.curr_group, // used as auxiliary group
                    ptr_group: self.ptr_group,
                    ptr_reset_group: self.ptr_reset_group,
                    line: instr.line_number,
                    // these two are set only once a MALLOC instruction is processed
                    // if there is no malloc, there is no memory access allowed
                    // and therefore these fields are never read
                    // TODO: throw err if any memory ops are used but no memory exists
                    // TODO: throw err if memory is created more than once (>1 malloc call)
                    // therefore it does not matter if there is junk data in there
                    // since it will either be overwritten or never read
                    memreg: self.aliases.memreg.clone(),
                    ptrpos_id: self.aliases.ptrpos_id,
                    displayed_items: self.displayed_items,
                    routine_count,
                    mem_end_counter: self.mem_end_counter,
                    mem_info: self.mem_info.clone(),
                };

                let data = match handler(args) {
                    Ok(data) => data,
                    Err(e) => {
                        self.errors.push(e);
                        continue;
                    }
                };
                for obj in data.objects.into_iter() {
                    level.add_object(obj);
                }

                let skip_spaces = data.skip_spaces;
                self.curr_group += data.used_extra_groups;
                obj_pos += skip_spaces as f64 * spacing;

                if data.added_item_display {
                    self.displayed_items += 1;
                }

                // these two if statements handle the logic of keeping track of the ptr group
                // it is necessary for instructions such as MRESET and MPTR which move the pointer
                // this information is only updated if it is set. this information is set
                // only in the malloc methods, which would usually be parsed first.

                if let Some(m) = data.new_mem {
                    // check that memory does not already exist
                    if let Some(_) = self.mem_info {
                        self.errors
                            .push(TasmParseError::MultipleMemoryInstances(instr.line_number));
                        continue;
                    }

                    // assigning new mem info, also assign the aliases

                    self.mem_info = Some(m.clone());
                    // assign to alias map
                    self.aliases.memreg = m.memreg;
                    self.aliases.ptrpos_id = m.ptrpos.to_counter_id().unwrap();
                    self.aliases.memsize = m.size;
                    // assign aliases themselves
                }

                if data.ptr_group != 0 {
                    self.ptr_group = data.ptr_group
                }

                if data.ptr_reset_group != 0 {
                    self.ptr_reset_group = data.ptr_reset_group
                }
            }
        }

        if self.start_rtn_group != 0 {
            let ioblock_result = ioblock(HandlerArgs {
                args: vec![
                    TasmValue::Group(self.start_rtn_group),
                    TasmValue::Number(0.0),
                    TasmValue::String("start".into()),
                ],
                cfg: GDObjConfig::new(),
                displayed_items: self.displayed_items,
                curr_group: self.curr_group,
                ptr_group: 0,
                ptr_reset_group: 0,
                memreg: TasmValue::default(),
                ptrpos_id: 0,
                routine_count: 0,
                mem_end_counter: 0,
                mem_info: None,
                line: 0,
            })
            .unwrap();

            // add starting block
            for obj in ioblock_result.objects.into_iter() {
                level.add_object(obj);
            }
        }

        if self.errors.len() > 0 {
            Err(self.errors.clone())
        } else {
            Ok(level)
        }
    }
}

#[derive(Debug, Default, Clone)]
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

#[derive(Clone)]
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

impl HandlerData {
    #[inline(always)]
    pub fn default() -> Self {
        Self {
            skip_spaces: 1, // always advance one space
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

#[derive(Debug, Clone)]
pub struct Instruction {
    pub ident: String,
    pub _type: InstrType,
    pub line_number: usize,
    pub args: Vec<TasmValue>,
    pub handler_fn: HandlerFn,
}

#[derive(Debug, Clone)]
pub enum TasmParseError {
    InvalidInstruction((String, usize)),
    InvalidArguments((String, usize)),
    InvalidAssignment((usize, ItemType)),
    InvalidWaitAmount((usize, i32)),
    BadID((String, usize)),
    BadToken((String, usize)),
    NoEntryPoint,
    InvalidNumber((String, String, usize)),
    InvalidGroup(ParseIntError),
    ExceedsGroupLimit,
    InitRoutineSpawnError(usize),
    MultipleMemoryInstances(usize),
    InvalidPointerMove(String, usize),
    MultipleRoutineDefintions(String, usize, usize),
    InitRoutineMemoryAccess(usize),
    NonexistentMemoryAccess(usize),
    TrailingComma(usize),
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
            Self::InvalidAssignment((line, _type)) => {
                write!(f, "Cannot assign to {_type:?} on line {}", line + 1)
            }
            Self::InvalidWaitAmount((line, wait)) => {
                write!(
                    f,
                    "Cannot wait a negative amount of ticks ({wait}) on line {line}."
                )
            }
            Self::BadID((msg, line)) => {
                write!(f, "Bad ID on line {}: {msg}.", line + 1)
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
            Self::InvalidNumber((why, num, line)) => {
                write!(f, "Invalid number {num} on line {}. {why}", line + 1)
            }
            Self::InvalidGroup(why) => {
                write!(f, "Invalid group. {why}")
            }
            Self::ExceedsGroupLimit => {
                write!(f, "Input file exceeds group limit of {GROUP_LIMIT} groups.")
            }
            Self::MultipleMemoryInstances(line) => {
                write!(f, "Multiple memory instances are not allowed: line {line}")
            }
            Self::InvalidPointerMove(reason, line) => {
                write!(f, "{reason} at line {line}.")
            }
            Self::MultipleRoutineDefintions(rtn, line, prev_line) => {
                write!(
                    f,
                    "Routine {rtn}, on line {line}, has already been declared on line {prev_line}."
                )
            }
            Self::InitRoutineMemoryAccess(line) => {
                write!(
                    f,
                    "Memory access attempt on line {line} is forbidden, due to being in the initializer routine."
                )
            }
            Self::NonexistentMemoryAccess(line) => {
                write!(
                    f,
                    "Attempted to access memory while none exists on line {line}."
                )
            }
            Self::TrailingComma(line) => {
                write!(f, "Trailing comma found at line {}.", line + 1)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum TasmValue {
    Counter(i16),
    Timer(i16),
    GDItem(Item),
    Number(f64),
    Group(i16),
    Alias(AliasType),
    /// Default
    String(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum AliasType {
    MEMREG,
    PTRPOS,
    MEMSIZE,
    POINTS,
    ATTEMPTS,
    MAINTIME,
}

#[derive(Debug)]
pub struct Alias {
    pub _type: AliasType,
    pub value: TasmValue,
}

impl AliasType {
    pub fn to_alias(s: &str) -> Option<Self> {
        match s {
            "MEMREG" => Some(Self::MEMREG),
            "PTRPOS" => Some(Self::PTRPOS),
            "MEMSIZE" => Some(Self::MEMSIZE),
            "POINTS" => Some(Self::POINTS),
            "ATTEMPTS" => Some(Self::ATTEMPTS),
            "MAINTIME" => Some(Self::MAINTIME),
            _ => None,
        }
    }

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

pub enum ParseErrorType {
    BadID,
    TrailingComma,
    InvalidNumber,
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
        let remaining_i16 = iter.into_iter().collect::<String>().parse::<i16>();

        // aliases are parsed before anything
        if let Some(a) = AliasType::to_alias(s) {
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
                    "Infinity not allowed.".into(),
                ));
            } else if n.is_nan() {
                return Err((ParseErrorType::InvalidNumber, "NaN not allowed.".into()));
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

pub fn fits_arg_signature(args: &Vec<TasmValue>, sig: &[TasmValueType]) -> bool {
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
        0 => args.len() == 0,
        1 => match &sig[0] {
            TasmValueType::List(l_type) => {
                // check that all arguments are of the type in the list
                args.iter().all(|arg| check_primitive(&l_type, arg))
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

            return true;
        }
    }
}

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
        "INITMEM" | "MALLOC" | "FMALLOC" | "PERS" | "DISPLAY" | "IOBLOCK" => Some(InstrType::Init),
        "MFUNC" | "MREAD" | "MWRITE" | "MPTR" | "MRESET" => Some(InstrType::Memory),
        "NOP" | "WAIT" => Some(InstrType::Wait),
        "TSPAWN" | "TSTART" | "TSTOP" => Some(InstrType::Timer),
        "STOP" | "PAUSE" | "RESUME" => Some(InstrType::Process),
        "BREAKPOINT" => Some(InstrType::Debug),
        _ => None,
    }
}

pub fn show_errors(es: Vec<TasmParseError>, err_msg: &str) {
    println!("{err_msg} with {} errors:", es.len());
    for e in es {
        println!("{e}");
    }
}
