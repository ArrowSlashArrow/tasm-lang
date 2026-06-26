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
        consts::{ENTRY_POINT, INIT_ROUTINE},
        error::{ParseErrorType, TasmError, TasmErrorType},
        flags::{Flag, FlagValueType, get_flag_type},
        push_error, push_error_lineless,
        structs::{
            Instruction, Routine, RoutineData, Tasm, TasmValue, fits_arg_signature,
            is_builtin_alias,
        },
    },
    instr::INSTR_SPEC,
    verbose_log,
};
use std::collections::{HashMap, hash_map};

const INIT_PLACEHOLDER_GROUP: i16 = -1i16;

impl Tasm {
    pub fn parse(&mut self, group_offset: i16, disable_entry_point_check: bool) {
        // index routines before anything else

        verbose_log!(self, "Indexing routines.");
        self.curr_group = group_offset;
        self.index_routines();

        verbose_log!(self, "Finished indexing routines.");

        if self.routine_data.is_empty() {
            // there is nothing to parse
            return;
        }

        // push _init routine to the start to process it before anyting else
        // important to do for alias resolution, since memtype is determined in init.
        if let Some(init_pos) = self
            .routine_data
            .iter()
            .position(|r| r.routine_ident == INIT_ROUTINE)
        {
            verbose_log!(self, "Pushing _init to front of routine data");
            let rtn = self.routine_data[init_pos].clone();
            self.routine_data.remove(init_pos);
            self.routine_data.insert(0, rtn);
            verbose_log!(self, "Parsing aliases");
            self.get_aliases_from_init();
        }

        // error if no entry point
        if !self.has_entry_point && !disable_entry_point_check {
            push_error_lineless(
                &mut self.errors,
                &self.fname,
                TasmErrorType::NoEntryPoint,
                "No entry point found in file.".into(),
            );
        }

        verbose_log!(self, "Parsing instructions.");
        self.handle_instructions();

        if !self.errors.is_empty() {
            verbose_log!(self, "Parsed file with {} errors.", self.errors.len());
        } else {
            verbose_log!(self, "Parsed file successfully with 0 errors.")
        }
    }

    pub fn get_aliases_from_init(&mut self) {
        // _init is at idx 0
        let init_instructions = &mut self.routine_data[0].lines;

        let mut aliases: HashMap<String, String> = HashMap::new();

        // need to take to iteration with mutable references to self in self.push_error
        let instrs = core::mem::take(init_instructions);
        for (line, raw_instr) in instrs.iter() {
            if !raw_instr.to_uppercase().starts_with("ALIAS ") {
                continue;
            }
            let args = raw_instr.split('|').next().unwrap();
            let trimmed = &args[6..] // condition above ensures that this never fails
                .split(',')
                .map(|v| v.trim())
                .collect::<Vec<_>>();
            if trimmed.len() != 2 {
                // otherwise, error
                push_error(
                    &mut self.errors,
                    &self.fname,
                    TasmErrorType::InvalidInstruction,
                    *line,
                    INIT_ROUTINE.into(),
                    "Instruction ALIAS must only have two arguments: [String, Any]".to_string(),
                );
            }

            if let Ok(v) = TasmValue::to_value(trimmed[0])
                && let Some(s) = v.to_string()
            {
                match aliases.entry(s) {
                    hash_map::Entry::Occupied(entry) => {
                        push_error(
                            &mut self.errors,
                            &self.fname,
                            TasmErrorType::BadAlias,
                            *line,
                            INIT_ROUTINE.into(),
                            format!("Cannot override existing alias {}.", entry.key()),
                        );
                    }
                    hash_map::Entry::Vacant(entry) => {
                        if is_builtin_alias(entry.key()) {
                            push_error(
                                &mut self.errors,
                                &self.fname,
                                TasmErrorType::BadAlias,
                                *line,
                                INIT_ROUTINE.into(),
                                format!("Cannot override default alias {}.", entry.key()),
                            );
                        } else {
                            entry.insert(trimmed[1].into());
                            continue;
                        }
                    }
                };
            } else {
                push_error(
                    &mut self.errors,
                    &self.fname,
                    TasmErrorType::InvalidInstruction,
                    *line,
                    INIT_ROUTINE.into(),
                    format!("Bad alias identifier: {}", trimmed[0]),
                );
            };
        }

        // put these back after taking
        self.routine_data[0].lines = instrs;

        self.defined_aliases = aliases;
    }
    pub fn mem_end_counter(mut self, ctr: i16) -> Self {
        self.mem_end_counter = ctr;
        self
    }

    pub fn handle_instructions(&mut self) {
        verbose_log!(self, "Restructuring routine data.");
        let mut routines: Vec<(Vec<(usize, String)>, Routine)> = self
            .routine_data
            .iter()
            .map(|r| {
                (
                    r.lines.to_owned(), // routine lines
                    Routine::default()
                        .group(match r.group_id {
                            // if this routine is the init routine, don't give it a group
                            INIT_PLACEHOLDER_GROUP => 0,
                            g => g,
                        })
                        .ident(&r.routine_ident), // routine object
                )
            })
            .collect();

        // can't have an immutable reference to this
        let gm = self.routine_group_map.clone();

        for (lines, routine) in routines.iter_mut() {
            let prev_err_count = self.errors.len();
            for (curr_line, line) in lines {
                let trimmed_line = line.trim();
                if trimmed_line.is_empty() {
                    continue; // skip blank line
                }

                self.parse_instr_line(routine, *curr_line, trimmed_line, &gm);
            }
            let new_err_count = self.errors.len();
            verbose_log!(
                self,
                "Parsed {} routine instructions with {} errors",
                routine.ident,
                new_err_count - prev_err_count
            );
            self.routines.push(routine.clone());
        }
    }

    fn parse_raw_value(&mut self, v: &str, curr_line: usize, routine: &str) -> Option<TasmValue> {
        match TasmValue::to_value(v.trim()) {
            // whitespace is stripped when parsing
            Ok(t) => self.parse_tasm_value(t, curr_line, routine),
            Err((etype, msg)) => {
                // error if unable to parse argument value
                push_error(
                    &mut self.errors,
                    &self.fname,
                    match etype {
                        ParseErrorType::BadID => TasmErrorType::BadID,
                        ParseErrorType::InvalidNumber => TasmErrorType::InvalidNumber,
                        ParseErrorType::TrailingComma => TasmErrorType::TrailingComma,
                        ParseErrorType::BadHexLiteral => TasmErrorType::BadHexLiteral,
                    },
                    curr_line,
                    routine.to_string(),
                    msg,
                );

                None
            }
        }
    }

    pub fn parse_instr_line(
        &mut self,
        curr_routine: &mut Routine,
        curr_line: usize,
        trimmed_line: &str,
        gm: &HashMap<String, i16>,
    ) {
        // determine the arguments and the flags
        // line is structured like this:
        // <whitespace> INSTR [...ARGS] [| ...FLAGS]

        let (args_string, flags) = match split_at_char_once(
            trimmed_line,
            '|',
            TasmError {
                etype: TasmErrorType::InvalidInstruction,
                file: self.fname.clone(),
                routine: curr_routine.ident.clone(),
                error: true,
                line: curr_line,
                details: "Bad flag arguments".into(),
            },
        ) {
            Ok((left, right)) => {
                if right.is_empty() {
                    (left, vec![])
                } else {
                    let flags_parsed = match parse_flags_str(
                        right,
                        curr_line,
                        &self.fname,
                        &curr_routine.ident,
                        gm,
                        &self.defined_aliases,
                    ) {
                        Ok(flags) => flags,
                        Err(e) => {
                            self.errors.push(e);
                            return;
                        }
                    };

                    (left, flags_parsed)
                }
            }
            Err(e) => {
                self.errors.push(e);
                return;
            }
        };

        let instr: String;
        let mut args: Vec<TasmValue> = vec![];

        let is_concurrent: bool;

        if let Some(pos) = args_string.trim().find(" ") {
            if args_string.ends_with(',') {
                push_error(
                    &mut self.errors,
                    &self.fname,
                    TasmErrorType::TrailingComma,
                    curr_line,
                    curr_routine.ident.clone(),
                    "Trailing commas are not allowed.".to_string(),
                );
                return;
            }

            let instr_raw = args_string[..pos].to_uppercase();
            if let Some(stripped) = instr_raw.strip_prefix('~') {
                is_concurrent = true;
                instr = stripped.to_string();
            } else {
                is_concurrent = false;
                instr = instr_raw;
            }

            // exclude alias instructions (parsed first thing after lexing)
            if instr.as_str() == "ALIAS" {
                if curr_routine.ident.as_str() != "_init" {
                    push_error(
                        &mut self.errors,
                        &self.fname,
                        TasmErrorType::NonInitAliasDefinition,
                        curr_line,
                        curr_routine.ident.clone(),
                        "Cannot define an alias outside of the init routine.".to_string(),
                    );
                }
                return;
            }

            let mut erroneous_instr = false;
            // get all chars after the first space, which separates the instruction and args
            let mut raw_args = args_string[pos + 1..]
                .split(',')
                .map(|v| v.trim().to_string())
                .collect::<Vec<_>>();

            for raw in raw_args.iter_mut() {
                // replace if an alias is referenced
                if let Some(raw_val) = self.defined_aliases.get(&raw.to_string()) {
                    *raw = raw_val.clone();
                }
            }

            for raw in raw_args {
                match self.parse_raw_value(&raw, curr_line, &curr_routine.ident) {
                    Some(v) => args.push(v),
                    None => erroneous_instr = true,
                }
            }
            if erroneous_instr {
                verbose_log!(self, "Got bad args.");
                push_error(
                    &mut self.errors,
                    &self.fname,
                    TasmErrorType::InvalidInstruction,
                    curr_line,
                    curr_routine.ident.clone(),
                    "Failed to parse instruction: invalid argset".into(),
                );
            }
        } else {
            // no args or extras (everything after | )
            let instr_raw = args_string.to_uppercase();
            if let Some(stripped) = instr_raw.strip_prefix('~') {
                is_concurrent = true;
                instr = stripped.to_string();
            } else {
                is_concurrent = false;
                instr = instr_raw;
            }
        }

        // find the instruction spec which contains arg handlers
        let (init_exclusive, handlers, itype) = match INSTR_SPEC.get(&instr) {
            Some(spec) => spec,
            None => {
                push_error(
                    &mut self.errors,
                    &self.fname,
                    TasmErrorType::InvalidInstruction,
                    curr_line,
                    curr_routine.ident.clone(),
                    format!("Unrecognized instruction {instr}"),
                );
                return;
            }
        };

        // check if this isntruction is allowed in the routine
        if *init_exclusive && curr_routine.ident != INIT_ROUTINE {
            push_error(
                &mut self.errors,
                &self.fname,
                TasmErrorType::InvalidInstruction,
                curr_line,
                curr_routine.ident.clone(),
                format!(
                    "Instruction {instr} is not allowed in routine {} because it is exclusive to the initialiser routine, {INIT_ROUTINE}.",
                    curr_routine.ident
                ),
            );
            return;
        }

        // find the handler function
        match handlers
            .iter()
            .find(|&(sig, _)| fits_arg_signature(&args, sig))
            .map(|v| v.1)
        {
            Some(handler) => {
                // finally, add instruction to routine
                curr_routine.add_instruction(Instruction {
                    ident: instr.clone(),
                    itype: *itype,
                    line_number: curr_line,
                    args,
                    flags,
                    handler_fn: handler,
                    is_concurrent,
                });
            }
            None => {
                let argtypes = &args.iter().map(|a| a.get_type()).collect::<Vec<_>>();
                // otherwise, error
                push_error(
                    &mut self.errors,
                    &self.fname,
                    TasmErrorType::InvalidInstruction,
                    curr_line,
                    curr_routine.ident.clone(),
                    format!(
                        "Instruction {instr} has no argument handler for the argset {argtypes:?}"
                    ),
                );
            }
        }
    }

    fn parse_tasm_value(
        &mut self,
        t: TasmValue,
        curr_line: usize,
        routine: &str,
    ) -> Option<TasmValue> {
        validate_tasm_value(
            t,
            &self.routine_group_map,
            &mut self.errors,
            &self.fname,
            routine,
            curr_line,
        )
    }

    pub fn index_routines(&mut self) {
        let mut seen_routines: HashMap<String, usize> = HashMap::new(); // routine => line number
        let mut curr_routine_data = RoutineData::default();
        let mut in_routine = false;

        // index all routines
        for (line_idx, line) in self.lines.iter().enumerate() {
            if line.trim().is_empty() {
                continue;
            }

            if !line.starts_with(' ') {
                // commit old data
                let routine_ident = curr_routine_data.routine_ident.clone();
                if routine_ident == INIT_ROUTINE {
                    curr_routine_data.group_id = INIT_PLACEHOLDER_GROUP;
                    self.curr_group -= 1;
                } else {
                    self.routine_group_map
                        .insert(routine_ident.clone(), self.curr_group);
                }

                // omit handling empty routines
                if !curr_routine_data.lines.is_empty() {
                    verbose_log!(self, "Got routine: {} on line {}", routine_ident, line_idx);
                    self.routine_data.push(curr_routine_data.clone());
                } else {
                    // didn't commit empty routine, re-use its group
                    self.curr_group -= 1;
                }

                // no indent, check for routine identifier.
                let mut strip = line.trim().to_string();
                if strip.ends_with(':') && !strip.contains(' ') {
                    self.curr_group += 1;
                    // now we are certain that this is a routine ident
                    strip.pop();
                    let routine_ident = strip;
                    if routine_ident == ENTRY_POINT {
                        self.has_entry_point = true;
                    }

                    // HashMap<K, V>.insert() returns None if the value was not already defined.
                    // If we don't get a none, the routine was already declared.
                    if seen_routines
                        .insert(routine_ident.clone(), line_idx)
                        .is_some()
                    {
                        verbose_log!(self, "Routine was already declared.");
                        push_error(
                            &mut self.errors,
                            &self.fname,
                            TasmErrorType::MultipleRoutineDefintions,
                            line_idx,
                            routine_ident.clone(),
                            format!(
                                "Routine {} was already declared on line {}",
                                routine_ident.clone(),
                                seen_routines.get(&routine_ident).unwrap_or(&0)
                            ),
                        );
                    }

                    // clear out bad data
                    curr_routine_data = RoutineData {
                        line_idx,
                        routine_ident,
                        group_id: self.curr_group,
                        lines: vec![],
                    };
                    in_routine = true;
                } else {
                    // this is not a routine identifier, so it is a bad token
                    verbose_log!(self, "Found bad token on line {line_idx}");
                    push_error(
                        &mut self.errors,
                        &self.fname,
                        TasmErrorType::BadToken,
                        line_idx,
                        "<No routine>".to_string(),
                        "Bad token.".to_string(),
                    );
                }
            } else if in_routine {
                let trim = line.trim();
                curr_routine_data.lines.push((line_idx, trim.to_owned()));
                if trim.is_empty() {
                    in_routine = false;
                }
            }
        }

        verbose_log!(self, "Pushing routine data.");
        // commit last routine data
        let routine_ident = curr_routine_data.routine_ident.clone();
        if routine_ident == INIT_ROUTINE {
            curr_routine_data.group_id = INIT_PLACEHOLDER_GROUP; // init has no group
        } else {
            self.routine_group_map
                .insert(routine_ident, self.curr_group);
        }
        self.routine_data.push(curr_routine_data);
    }
}

fn split_at_char_once(instr: &str, ch: char, err: TasmError) -> Result<(&str, &str), TasmError> {
    let mut line_split = instr.split(ch);

    // the first part is always present, which is guaranteed to be
    // the string with the instruction and its arguments
    let left = line_split.next().unwrap();

    let right = line_split.next().unwrap_or_default();

    if line_split.next().is_some() {
        return Err(err);
    }

    Ok((left, right))
}

fn parse_flags_str(
    flags_str: &str,
    curr_line: usize,
    file: &String,
    routine: &String,
    gm: &HashMap<String, i16>,
    aliases: &HashMap<String, String>,
) -> Result<Vec<Flag>, TasmError> {
    let raw_flags = flags_str.split_whitespace();

    let mut preprocessed = vec![];
    let mut in_dict = false;
    let mut dict_ident = String::new();
    let mut current_dict = String::new();

    // preprocessing, for joining of dicts

    for flag_segment in raw_flags {
        if in_dict {
            // disallow spaces between colons, i.e. no 123: 234
            current_dict.push_str(flag_segment);
            if flag_segment.ends_with('}') {
                in_dict = false;
                preprocessed.push((
                    dict_ident.clone(),
                    current_dict.clone(),
                    FlagValueType::Dict,
                ));
            }
            continue;
        }

        match split_at_char_once(
            flag_segment,
            ':',
            TasmError {
                etype: TasmErrorType::BadFlag,
                file: file.to_owned(),
                routine: routine.to_owned(),
                error: true,
                line: curr_line,
                details: format!("Bad flag: {flag_segment}"),
            },
        ) {
            Ok((ident, value)) => match get_flag_type(ident) {
                Some(t) => match t {
                    FlagValueType::Dict => {
                        // if a dict flag is identified, it is the beginning of the dict
                        if value.ends_with('}') {
                            // dict is contained in one segment
                            preprocessed.push((ident.into(), value.into(), FlagValueType::Dict));
                            continue;
                        }

                        // if a dict does not end with a } , it must be in multiple segments
                        // therefore, the first char must be a { since the rest of the dict
                        // is in other segments of the iterator.
                        // we can set the in_dict flag to find the other segments and concatenate them
                        in_dict = true;
                        dict_ident = ident.into();
                        current_dict = value.into();
                    }
                    t => preprocessed.push((ident.into(), value.into(), t)),
                },
                None => {
                    return Err(TasmError {
                        etype: TasmErrorType::BadFlag,
                        file: file.to_owned(),
                        routine: routine.to_owned(),
                        error: true,
                        line: curr_line,
                        details: format!("Unrecognized flag {flag_segment}"),
                    });
                }
            },
            Err(e) => return Err(e),
        }
    }

    let mut parsed_flags = vec![];

    for (ident, raw_value, t) in preprocessed {
        match Flag::from(ident.clone(), &raw_value, t.clone(), gm, aliases) {
            Some(flag) => parsed_flags.push(flag),
            None => {
                return Err(TasmError {
                    etype: TasmErrorType::BadFlag,
                    file: file.to_owned(),
                    routine: routine.to_owned(),
                    error: true,
                    line: curr_line,
                    details: format!(
                        "Unable to parse {ident} with value of {raw_value} and type {t:?}"
                    ),
                });
            }
        }
    }

    Ok(parsed_flags)
}

pub fn validate_tasm_value(
    t: TasmValue,
    routine_group_map: &HashMap<String, i16>,
    errors: &mut Vec<TasmError>,
    fname: &str,
    routine: &str,
    curr_line: usize,
) -> Option<TasmValue> {
    // if this is a routine ident, add corresponding group
    if let TasmValue::String(s) = t {
        match routine_group_map.get(&s) {
            Some(&group) => {
                if group != INIT_PLACEHOLDER_GROUP {
                    Some(TasmValue::Group(group))
                } else {
                    // only throw err if the group is the _init group
                    errors.push(TasmError {
                        etype: TasmErrorType::InitRoutineSpawnError,
                        file: fname.to_string(),
                        routine: routine.to_string(),
                        error: true,
                        line: curr_line,
                        details: "Cannot spawn init routine.".to_string(),
                    });
                    None
                }
            }
            None => Some(TasmValue::String(s)),
        }
    } else {
        Some(t)
    }
}

pub fn parse_file<T: AsRef<str>>(
    in_str: T,
    fname: String,
    mem_end_counter: i16,
    group_offset: i16,
    verbose_logs: bool,
    log_errs: bool,
    disable_entry_point_check: bool,
) -> Result<Tasm, Vec<TasmError>> {
    let mut tasm = Tasm::default().mem_end_counter(mem_end_counter);
    let lines = in_str
        .as_ref()
        .replace('\t', " ") // tabs converted to spaces, works for parsing purposes.
        .lines() // remove comments
        .map(|l| l.split(';').next().unwrap().trim_end().to_string())
        .collect::<Vec<String>>();

    tasm.lines = lines;
    tasm.logs_enabled = verbose_logs;
    tasm.group_offset = group_offset;
    tasm.fname = fname;
    tasm.parse(group_offset, disable_entry_point_check);

    if tasm.errors.is_empty() {
        Ok(tasm)
    } else {
        if log_errs && verbose_logs {
            for err in &tasm.errors {
                println!("{err}");
            }
        }
        Err(tasm.errors)
    }
}
