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
use std::mem::take;

use crate::{
    core::{
        consts::{ENTRY_POINT, INIT_ROUTINE},
        error::{ParseErrorType, TasmError, TasmErrorType},
        flags::{Flag, FlagValueType, get_flag_type},
        push_error, push_error_lineless,
        structs::{
            Instruction, Routine, Tasm, TasmValue, fits_arg_signature, get_instr_type,
            is_builtin_alias,
        },
    },
    instr::INSTR_SPEC,
    verbose_log,
};

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

        verbose_log!(self, "Pushing _init to front of routine data");
        // push _init routine to the start to process it before anyting else
        // important to do for alias resolution, since memtype is determined in init.
        if let Some(init_pos) = self.routine_data.iter().position(|r| r.1 == INIT_ROUTINE) {
            let rtn = self.routine_data[init_pos].clone();
            self.routine_data.remove(init_pos);
            self.routine_data.insert(0, rtn);
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
        let init_instructions = &mut self.routine_data[0].3;

        let mut aliases: Vec<(String, String)> = vec![];

        // need to take to iteration with mutable references to self in self.push_error
        let instrs = take(init_instructions);
        for (line, raw_instr) in instrs.iter() {
            if !raw_instr.starts_with("ALIAS ") {
                continue;
            }
            let args = raw_instr.split('|').next().unwrap();
            let trimmed = args
                .strip_prefix("ALIAS ")
                .unwrap()
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
                if aliases.iter().any(|(a, _)| *a == s) {
                    push_error(
                        &mut self.errors,
                        &self.fname,
                        TasmErrorType::BadAlias,
                        *line,
                        INIT_ROUTINE.into(),
                        format!("Cannot override existing alias {s}."),
                    );
                } else if is_builtin_alias(&s) {
                    push_error(
                        &mut self.errors,
                        &self.fname,
                        TasmErrorType::BadAlias,
                        *line,
                        INIT_ROUTINE.into(),
                        format!("Cannot override default alias {s}."),
                    );
                } else {
                    aliases.push((s, trimmed[1].into()));
                    continue;
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
        self.routine_data[0].3 = instrs;

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
            .map(|(_, ident, group, lines)| {
                (
                    lines.to_owned(), // routine lines
                    Routine::default()
                        .group(match *group {
                            // if this routine is the init routine, don't give it a group
                            INIT_PLACEHOLDER_GROUP => 0,
                            g => g,
                        })
                        .ident(ident), // routine object
                )
            })
            .collect();

        for (lines, routine) in routines.iter_mut() {
            let prev_err_count = self.errors.len();
            for (curr_line, line) in lines {
                let trimmed_line = line.trim();
                if trimmed_line.is_empty() {
                    continue; // skip blank line
                }

                // parse instruction and args
                self.parse_instr_line(routine, *curr_line, trimmed_line);
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

    fn parse_raw_value(&mut self, v: &str, curr_line: usize, routine: String) -> Option<TasmValue> {
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
                    },
                    curr_line,
                    routine.clone(),
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
    ) {
        // determine the arguments and the flags
        // line is structured like this:
        // <whitespace> INSTR [...ARGS] [| ...FLAGS]

        let (args_string, flags) = match split_at_char_once(
            trimmed_line,
            '|',
            TasmError {
                _type: TasmErrorType::InvalidInstruction,
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
                    let flags_parsed =
                        match parse_flags_str(right, curr_line, &self.fname, &curr_routine.ident) {
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
                    format!("Trailing commas are not allowed."),
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
                        format!("Cannot define an alias outside of the init routine."),
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
                if let Some((_, raw_val)) = self.defined_aliases.iter().find(|a| a.0 == *raw) {
                    *raw = raw_val.clone();
                }
            }

            for raw in raw_args {
                match self.parse_raw_value(&raw, curr_line, curr_routine.ident.clone()) {
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
        let (_, init_exclusive, handlers) =
            match INSTR_SPEC.iter().find(|(ident, _, _)| ident == &instr) {
                Some(spec) => spec,
                None => {
                    push_error(
                        &mut self.errors,
                        &self.fname,
                        TasmErrorType::InvalidInstruction,
                        curr_line,
                        curr_routine.ident.clone(),
                        format!("Unrecognized instruction {instr}: "),
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
                    _type: get_instr_type(&instr).unwrap(),
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
        routine: String,
    ) -> Option<TasmValue> {
        parse_tasm_value(
            t,
            &self.routine_group_map,
            &mut self.errors,
            self.fname.clone(),
            routine,
            curr_line,
        )
    }

    pub fn index_routines(&mut self) {
        let mut curr_routine_data = (0usize, String::new(), 0i16, vec![]);

        // index all routines
        let mut in_routine = false;
        for (line_idx, line) in self.lines.iter().enumerate() {
            if line.trim().is_empty() {
                continue;
            }

            if !line.starts_with(' ') {
                // commit old data
                // due to this being the first piece of code being ran once parsing starts,
                // the first routine data will therefore be garbage data.
                let routine_ident = curr_routine_data.1.clone();
                if routine_ident == INIT_ROUTINE {
                    curr_routine_data.2 = INIT_PLACEHOLDER_GROUP; // init has no group, -1 serves as a unique _init marker
                    self.curr_group -= 1;
                } else {
                    self.routine_group_map
                        .push((routine_ident.clone(), self.curr_group));
                }

                // ignore the garbage data
                if self.routine_group_map.len() > 1 {
                    verbose_log!(self, "Got routine: {} on line {}", routine_ident, line_idx);
                }
                self.routine_data.push(curr_routine_data.clone());

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

                    // check that this routine was not already declared
                    // take to iterate with mutable reference available for error pushing

                    for rtn in self.routine_data.iter() {
                        if routine_ident == rtn.1 {
                            verbose_log!(self, "Routine was already declared.");
                            push_error(
                                &mut self.errors,
                                &self.fname,
                                TasmErrorType::MultipleRoutineDefintions,
                                line_idx,
                                routine_ident.clone(),
                                format!(
                                    "
                                Routine {} was already declared on line {}",
                                    rtn.1.clone(),
                                    rtn.0
                                ),
                            );
                        }
                    }

                    // clear out bad data
                    curr_routine_data = (line_idx, routine_ident, self.curr_group, vec![]);
                    in_routine = true;
                } else {
                    // this is not a routine identifier, so it is a bad token
                    verbose_log!(self, "Found bad token on line {line_idx}");
                    push_error(
                        &mut self.errors,
                        &self.fname,
                        TasmErrorType::BadToken,
                        line_idx,
                        format!("<No routine>"),
                        format!("Bad token."),
                    );
                }
            } else if in_routine {
                let trim = line.trim();
                curr_routine_data.3.push((line_idx, trim.to_owned()));
                if trim.is_empty() {
                    in_routine = false;
                }
            }
        }

        verbose_log!(self, "Pushing routine data.");
        // commit last routine data
        let routine_ident = curr_routine_data.1.clone();
        if routine_ident == INIT_ROUTINE {
            curr_routine_data.2 = 0i16; // init has no group
        } else {
            self.routine_group_map
                .push((routine_ident, self.curr_group));
        }
        self.routine_data.push(curr_routine_data.clone());

        verbose_log!(self, "Removing garbage routine data.");
        // first routine was garbage data, so remove it
        self.routine_data.remove(0);
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
) -> Result<Vec<Flag>, TasmError> {
    let raw_flags = flags_str.trim().split(' ');

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
                _type: TasmErrorType::BadFlag,
                file: file.clone(),
                routine: routine.clone(),
                error: true,
                line: curr_line,
                details: format!("Bad flag: {flag_segment}"),
            },
        ) {
            Ok((ident, value)) => {
                match get_flag_type(ident) {
                    Some(t) => match t {
                        FlagValueType::Dict => {
                            // if a dict flag is identified, it is the beginning of the dict
                            if value.ends_with('}') {
                                // dict is contained in one segment
                                preprocessed.push((
                                    ident.into(),
                                    value.into(),
                                    FlagValueType::Dict,
                                ));
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
                            _type: TasmErrorType::BadFlag,
                            file: file.clone(),
                            routine: routine.clone(),
                            error: true,
                            line: curr_line,
                            details: format!("Unrecognized flag {flag_segment}"),
                        });
                    }
                }
            }
            Err(e) => return Err(e),
        }
    }

    let mut parsed_flags = vec![];

    for (ident, raw_value, t) in preprocessed {
        match Flag::from(ident.clone(), &raw_value, t.clone()) {
            Some(flag) => parsed_flags.push(flag),
            None => {
                return Err(TasmError {
                    _type: TasmErrorType::BadFlag,
                    file: file.clone(),
                    routine: routine.clone(),
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

pub fn parse_tasm_value(
    t: TasmValue,
    routine_group_map: &[(String, i16)],
    errors: &mut Vec<TasmError>,
    fname: String,
    routine: String,
    curr_line: usize,
) -> Option<TasmValue> {
    // if this is a routine ident, add corresponding group
    if let TasmValue::String(s) = t.clone() {
        match routine_group_map
            .iter()
            .find(|(ident, _)| *ident == s)
            .map(|data| data.1)
        {
            Some(group) => {
                if group != INIT_PLACEHOLDER_GROUP {
                    Some(TasmValue::Group(group))
                } else {
                    // only throw err if the group is the _init group
                    errors.push(TasmError {
                        _type: TasmErrorType::InitRoutineSpawnError,
                        file: fname,
                        routine,
                        error: true,
                        line: curr_line,
                        details: format!("Cannot spawn init routine."),
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
