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

use crate::{
    core::{
        ENTRY_POINT, INIT_ROUTINE, Instruction, Routine, Tasm, TasmParseError, TasmPrimitive,
        TasmValue, TasmValueType, get_instr_type,
    },
    instr::INSTR_SPEC,
};

// todo: dynamic load from commands file
pub const INSTRUCTIONS: &[&str] = &[
    "INITMEM", // init
    "MALLOC",
    "FMALLOC",
    "MFUNC", // memory
    "MREAD",
    "MWRITE",
    "MPTR",
    "MRESET",
    "DISPLAY", // init
    "IOBLOCK",
    "BREAKPOINT", // debug
    "NOP",        // wait
    // "WAIT",
    // "TSPAWN", // timer
    // "TSTART",
    // "TSTOP",
    // "SRAND", // process
    // "FRAND",
    // "RET",
    // "STOP",
    "SPAWN",
    "PERS", // init
    "ADD",  // arithmetic
    "SUB",
    "MUL",
    "DIV",
    "FLDIV",
    "MOV",
    "SE", // comparison
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

pub const ALIASES: &[(&'static str, i16)] = &[
    ("MEMREG", -1), // Counter or timer (depends on memory type) at the end of malloc'ed counters + 1
    ("PTRPOS", 0),  // Always a counter at the end of the malloc'ed counters + 2
];

const INIT_PLACEHOLDER_GROUP: i16 = -1i16;

pub fn fits_arg_sig(args: &Vec<TasmValue>, sig: &[TasmValueType]) -> bool {
    // helper fn
    fn check_primitive(p: &TasmPrimitive, arg: &TasmValue) -> bool {
        // check if an int is required here
        // get_type returns `Number` for a `Number` even if it is an `Int`
        match p {
            TasmPrimitive::Int => arg.is_int(),
            _ => &arg.get_type() == p,
        }
    }
    match args.len() {
        0 => sig.len() == 0,
        1 => match &sig[0] {
            TasmValueType::List(l_type) => {
                // check that all arguments are of the type in the list
                args.iter().all(|arg| check_primitive(&l_type, arg))
            }
            TasmValueType::Primitive(p) => {
                // check that the argument matches the specified type
                check_primitive(p, &args[0])
            }
        },
        n => {
            if sig.len() != n {
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

pub fn parse_tasm_value(
    t: TasmValue,
    routine_group_map: &Vec<(String, i16)>,
    errors: &mut Vec<TasmParseError>,
    mem_end_counter: i16,
    curr_line: usize,
) -> Option<TasmValue> {
    let alias_lookup = |s: &str| -> Option<TasmValue> {
        match s {
            // TODO: make MEMREG a timer if the memory is a timer
            "MEMREG" => Some(TasmValue::Counter(mem_end_counter - 1)),
            "PTRPOS" => Some(TasmValue::Counter(mem_end_counter)),
            _ => None,
        }
    };

    // if this is a routine ident, add corresponding group
    if let TasmValue::String(s) = t.clone() {
        match routine_group_map
            .iter()
            .find(|(ident, _)| *ident == s)
            .and_then(|data| Some(data.1))
        {
            Some(group) => {
                if group != INIT_PLACEHOLDER_GROUP {
                    Some(TasmValue::Group(group))
                } else {
                    // only throw err if the group is the _init group
                    errors.push(TasmParseError::InitRoutineSpawnError(curr_line + 1));
                    None
                }
            }
            None => match alias_lookup(&s) {
                Some(v) => Some(v),
                None => Some(TasmValue::String(s)),
            },
        }
    } else {
        Some(t)
    }
}

pub fn parse_file<T: AsRef<str>>(
    f_str: T,
    mem_end_counter: i16,
) -> Result<Tasm, Vec<TasmParseError>> {
    let file = f_str.as_ref();
    let mut errors = vec![];
    let lines = file
        .lines()
        .map(|l| l.split(';').next().unwrap().trim_end())
        .collect::<Vec<&str>>();

    // [(line, ident, group, instruction_lines: (idx, line))]
    let mut routine_data = vec![];
    // [(group, ident)]: mapping between what routine has which group
    let mut routine_group_map = vec![];

    let mut seen_entry_point = false;

    let mut curr_group = 0i16;
    let mut curr_routine_data = (0usize, String::new(), 0i16, vec![]);

    // index all routines
    let mut in_routine = false;
    for (line_idx, line) in lines.iter().enumerate() {
        if line.trim().is_empty() {
            continue;
        }

        if !line.starts_with(' ') {
            // commit old data
            let routine_ident = curr_routine_data.1.clone();
            if routine_ident == INIT_ROUTINE {
                curr_routine_data.2 = INIT_PLACEHOLDER_GROUP; // init has no group, -1 serves as a unique _init marker
                curr_group -= 1;
            } else {
                routine_group_map.push((routine_ident, curr_group));
            }
            routine_data.push(curr_routine_data.clone());

            // no indent, check for routine identifier.
            let mut strip = line.trim().to_string();
            if strip.ends_with(':') && !strip.contains(' ') {
                curr_group += 1;
                // now we are certain that this is a routine ident
                strip.pop();
                let routine_ident = strip;
                if routine_ident == ENTRY_POINT {
                    seen_entry_point = true;
                }
                // clear out bad data
                curr_routine_data = (line_idx, routine_ident, curr_group, vec![]);
                in_routine = true;
            } else {
                // this is not a routine identifier, so it is a bad token
                errors.push(TasmParseError::BadToken((line.to_string(), line_idx)));
            }
        } else if in_routine {
            let trim = line.trim();
            curr_routine_data.3.push((line_idx, trim));
            if trim.is_empty() {
                in_routine = false;
            }
        }
    }

    // commit routine data
    let routine_ident = curr_routine_data.1.clone();
    if routine_ident == INIT_ROUTINE {
        curr_routine_data.2 = 0i16; // init has no group
    } else {
        routine_group_map.push((routine_ident, curr_group));
    }
    routine_data.push(curr_routine_data.clone());

    // first routine was garbage data, so remove it
    routine_data.remove(0);

    if !seen_entry_point {
        errors.push(TasmParseError::NoEntryPoint);
    }

    let mut routines = vec![];

    for (_, rtn_ident, rtn_group, rtn_lines) in routine_data {
        let mut curr_routine = Routine::default()
            .group(match rtn_group {
                // if this routine is the init routine, don't give it a group
                INIT_PLACEHOLDER_GROUP => 0,
                g => g,
            })
            .ident(&rtn_ident);

        for (curr_line, line) in rtn_lines {
            let trimmed_line = line.trim();
            if trimmed_line == "" {
                continue; // skip blank line
            }

            // parse instruction and args
            let instr;
            let args: Vec<TasmValue>;
            if let Some(pos) = trimmed_line.trim().find(" ") {
                instr = trimmed_line[..pos].to_uppercase();

                let mut erroneous_instr = false;
                args = trimmed_line[pos + 1..]
                    .split(',')
                    .filter_map(|v| match TasmValue::to_value(v.trim()) {
                        Ok(t) => parse_tasm_value(
                            t,
                            &routine_group_map,
                            &mut errors,
                            mem_end_counter,
                            curr_line,
                        ),
                        Err(e) => {
                            // error if unable to parse argument value
                            errors.push(e);
                            erroneous_instr = true;
                            None
                        }
                    })
                    .collect();
                if erroneous_instr {
                    errors.push(TasmParseError::InvalidInstruction((
                        "Failed to parse instruction: invalid argset".into(),
                        curr_line,
                    )));
                }
            } else {
                // no args or extras (everything after | )
                instr = trimmed_line.to_uppercase();
                args = vec![];
            }

            if !INSTRUCTIONS.contains(&instr.as_str()) {
                // error due to unrecognized instruction
                errors.push(TasmParseError::InvalidInstruction((
                    instr.into(),
                    curr_line,
                )));
                continue;
            }

            // find the instruction spec which contains arg handlers
            let (_, init_exclusive, handlers) =
                match INSTR_SPEC.iter().find(|(ident, _, _)| ident == &instr) {
                    Some(spec) => spec,
                    None => {
                        errors.push(TasmParseError::InvalidInstruction((
                            format!("Unrecognized instruction {instr}: "),
                            curr_line,
                        )));
                        continue;
                    }
                };

            // check if this isntruction is allowed in the routine
            if *init_exclusive && rtn_ident != INIT_ROUTINE {
                errors.push(TasmParseError::InvalidInstruction((
                    format!(
                        "Instruction {instr} is not allowed in routine {} because it is exclusive to the initialiser routine, {INIT_ROUTINE}.",
                        curr_routine.ident,
                    ),
                    curr_line,
                )));
                continue;
            }

            // find the handler function
            match handlers
                .iter()
                .find(|&(sig, _)| fits_arg_sig(&args, sig))
                .and_then(|v| Some(v.1))
            {
                Some(handler) => {
                    // finally, add instruction to routine
                    curr_routine.add_instruction(Instruction {
                        ident: instr.clone(),
                        _type: get_instr_type(&instr).unwrap(),
                        line_number: curr_line,
                        args,
                        handler_fn: handler,
                    });
                }
                None => {
                    // otherwise, error
                    errors.push(TasmParseError::InvalidInstruction((
                        format!(
                            "Instruction {instr} has no argument handler for the given arguments."
                        ),
                        curr_line,
                    )));
                }
            }
        }
        routines.push(curr_routine);
    }

    if errors.len() > 0 {
        Err(errors)
    } else {
        Ok(Tasm::from_routines(routines))
    }
}
