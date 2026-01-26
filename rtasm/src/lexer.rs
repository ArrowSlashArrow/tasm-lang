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
    "WAIT",
    "TSPAWN", // timer
    "TSTART",
    "TSTOP",
    "SRAND", // process
    "FRAND",
    "RET",
    "STOP",
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

pub fn parse_file<T: AsRef<str>>(f_str: T) -> Result<Tasm, Vec<TasmParseError>> {
    let file = f_str.as_ref();
    let mut errors = vec![];
    let lines = file
        .lines()
        .map(|l| l.split(';').next().unwrap().trim_end())
        .collect::<Vec<&str>>();

    // [(line, ident, group, instruction_lines)]
    let mut routine_data = vec![];
    // [(group, ident)]: mapping between what routine has which group
    let mut routine_group_map = vec![];

    let mut seen_entry_point = false;

    let mut curr_group = 0i16;
    let mut curr_routine_data = (0usize, String::new(), 0i16, vec![]);

    // index all routines
    let mut line_idx = 0usize;
    let mut in_routine = false;
    for line in lines.iter() {
        if line.trim().is_empty() {
            line_idx += 1;
            continue;
        }

        if !line.starts_with(' ') {
            // commit old data
            let routine_ident = curr_routine_data.1.clone();
            if routine_ident == INIT_ROUTINE {
                curr_routine_data.2 = 0i16; // init has no group
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
            if !trim.is_empty() {
                curr_routine_data.3.push(trim);
            } else {
                in_routine = false;
            }
        }
        line_idx += 1;
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

    println!("{routine_data:#?}");

    if !seen_entry_point {
        errors.push(TasmParseError::NoEntryPoint);
    }

    let mut routines = vec![];

    for (start_line, ident, rtn_group, rtn_lines) in routine_data {
        let mut curr_routine = Routine::default().group(rtn_group).ident(&ident);

        let mut curr_line = start_line + 1;
        for line in rtn_lines {
            curr_line += 1;

            let trimmed_line = line.trim();
            if trimmed_line == "" {
                continue; // skip blank line
            }

            // parse instruction and args
            let instr;
            let args: Vec<TasmValue>;
            if let Some(pos) = trimmed_line.trim().find(" ") {
                instr = &trimmed_line[..pos];

                let mut erroneous_instr = false;
                args = trimmed_line[pos + 1..]
                    .split(',')
                    .filter_map(|v| match TasmValue::to_value(v.trim()) {
                        Ok(t) => {
                            // if this is a routine ident, add corresponding group
                            if let TasmValue::String(s) = t.clone() {
                                if let Some(group) = routine_group_map
                                    .iter()
                                    .find(|(ident, _)| *ident == s)
                                    .and_then(|data| Some(data.1))
                                {
                                    Some(TasmValue::Group(group))
                                } else {
                                    errors.push(TasmParseError::InitRoutineSpawnError(curr_line));
                                    None
                                }
                            } else {
                                Some(t)
                            }
                        }
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
                instr = trimmed_line;
                args = vec![];
            }

            if !INSTRUCTIONS.contains(&instr) {
                // error due to unrecognized instruction
                errors.push(TasmParseError::InvalidInstruction((
                    instr.into(),
                    curr_line,
                )));
            }

            let args_signature = args
                .iter()
                .map(|a| a.get_type())
                .collect::<Vec<TasmPrimitive>>();

            // the args are now valid tasm values,
            // so we can find the matching argset in instr.rs to get handler

            let (_, init_exclusive, handlers) =
                match INSTR_SPEC.iter().find(|(ident, _, _)| ident == &instr) {
                    Some(spec) => spec,
                    None => {
                        errors.push(TasmParseError::InvalidInstruction((
                            format!("Instruction {instr} has no argument handler."),
                            curr_line,
                        )));
                        continue;
                    }
                };

            // check if this isntruction is allowed in the routine
            if *init_exclusive && instr != INIT_ROUTINE {
                errors.push(TasmParseError::InvalidInstruction((
                    format!(
                        "Instruction {instr} is not allowed in routine {} because it is excluse to the initialiser routine, {INIT_ROUTINE}.",
                        curr_routine.ident,
                    ),
                    curr_line,
                )));
                continue;
            }

            // find the handler function
            match handlers
                .iter()
                .find(|&(sig, _)| {
                    if sig.len() > 0
                        && let TasmValueType::List(list_type) = &sig[0]
                    // an argset with a list signature should NEVER have any other argument types.
                    {
                        args_signature.iter().all(|arg_type| arg_type == list_type)
                    } else {
                        // assume that the sig is made entirely of primitives
                        sig.iter()
                            .filter_map(|v| match v {
                                TasmValueType::Primitive(p) => Some(p),
                                _ => None,
                            })
                            .eq(&args_signature[..])
                    }
                })
                // return fn pointer
                .and_then(|v| Some(v.1))
            {
                Some(handler) => {
                    // finally, add instruction to routine
                    curr_routine.add_instruction(Instruction {
                        ident: instr.into(),
                        _type: get_instr_type(instr).unwrap(),
                        line_number: curr_line,
                        args,
                        handler_fn: handler,
                    });
                }
                None => {
                    // otherwise, error
                    errors.push(TasmParseError::InvalidInstruction((
                        format!("Instruction {instr} has no argument handler."),
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
