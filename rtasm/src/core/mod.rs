use gdlib::{
    gdlevel::Level,
    gdobj::{GDObjConfig, ItemType, misc::text},
};

use crate::{
    core::{
        consts::{ENTRY_POINT, GROUP_LIMIT, INIT_ROUTINE},
        error::{TasmError, TasmErrorType},
        structs::{
            Aliases, HandlerArgs, HandlerData, InstrType, Instruction, MemInfo, Routine, Tasm,
            TasmValue,
        },
    },
    instr::get_item_spec,
};

use alloc::borrow::Cow;
use std::collections::HashMap;

pub mod consts {
    pub const ENTRY_POINT: &str = "_start";
    pub const INIT_ROUTINE: &str = "_init";
    pub const GROUP_LIMIT: i16 = 9_999;
}
pub mod error;
pub mod flags;
pub mod structs;

pub type HandlerReturn = Result<HandlerData, TasmError>;
pub type HandlerFn = for<'a> fn(HandlerArgs<'a>) -> HandlerReturn;

#[macro_export]
macro_rules! verbose_log {
    ($this:expr, $($arg:tt)*) => {
        if $this.logs_enabled {
            println!($($arg)*);
        }
    };
}

#[macro_export]
macro_rules! log {
    ($on:expr, $($t:tt)*) => {
        if $on {
            println!($($t)*);
        }
    };
}

impl Tasm {
    pub fn handle_routines(&mut self, level_name: &str) -> Result<Level, Vec<TasmError>> {
        // clear errors
        self.errors.clear();

        let spacing = match self.release_mode {
            true => 1.0,
            false => 30.0,
        };

        // setup state
        self.aliases.ptrpos_id = self.mem_end_counter;
        let mut level = Level::new(level_name, "tasm", None, None);

        let routine_count = self.routines.len();
        self.curr_group = routine_count as i16 + self.group_offset + 1;

        // need to take to iteration with mutable references to self in self.push_error and self.handle_instruction
        let routines = core::mem::take(&mut self.routines);
        for routine in &routines {
            // setup position variables
            let mut obj_pos = 0.0;
            // subtracting from group offset ensures that high group IDs are still placed close to y=0
            let rtn_ypos = 75.0 + ((routine.group - self.group_offset) as f64) * 30.0;
            if self.curr_group > GROUP_LIMIT {
                push_error_lineless(
                    &mut self.errors,
                    &self.fname,
                    TasmErrorType::ExceedsGroupLimit,
                    format!("Program uses more than {GROUP_LIMIT} groups."),
                );
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

            let mut previous_spacing_amount = 0.0;

            // starting position of objects: (15, 75 + curr_group * 15)
            for instr in routine.instructions.iter() {
                self.handle_instruction(
                    instr,
                    routine,
                    &mut previous_spacing_amount,
                    &mut obj_pos,
                    rtn_ypos,
                    spacing,
                    routine_count,
                    &mut level,
                );
            }
        }
        self.routines = routines;

        if !self.errors.is_empty() {
            // Given that we won't be using this TASM-object again (since we faild to compile),
            // taking the errors will be ultimately more efficient.
            Err(core::mem::take(&mut self.errors))
        } else {
            Ok(level)
        }
    }

    pub fn handle_instruction(
        &mut self,
        instr: &Instruction,
        routine: &Routine,
        previous_spacing_amount: &mut f64,
        obj_pos: &mut f64,
        rtn_ypos: f64,
        spacing: f64,
        routine_count: usize,
        level: &mut Level,
    ) {
        let resolved_args = resolve_aliases(instr, &self.aliases);

        // check that we are not accessing memory in init routine
        if let Err((etype, msg)) =
            verify_memory_access(instr, routine.ident.as_str(), &self.mem_info)
        {
            push_error(
                &mut self.errors,
                &self.fname,
                etype,
                instr.line_number,
                routine.ident.clone(),
                msg,
            );
            return;
        }

        // check that any bad assignments aren't happening
        if let Err(err) = verify_assign(instr, &resolved_args[..]) {
            push_error(
                &mut self.errors,
                &self.fname,
                TasmErrorType::InvalidAssignment,
                instr.line_number,
                routine.ident.clone(),
                err,
            );
            return;
        }

        // do not increment x-position if this instruction is concurrent.
        // in a concurrent chain, all instructions before the last one
        // will be ignored for extra spacing. therefore, it is the responsibility
        // of the programmer to manage timing with concurrent instructions.
        if instr.is_concurrent {
            // move back
            *obj_pos -= *previous_spacing_amount;
        }

        let cfg = if routine.ident == INIT_ROUTINE {
            if let InstrType::Init = instr.itype {
                // in the case of a custom init structure,
                // leave default obj config since it likely wont be used anyways
                GDObjConfig::default()
            } else {
                // in the case of a normal position-dependent instruction
                // negate usual position to place normal triggers in init routine
                // before the x=0 line to make the instantly execute at the level start
                GDObjConfig::default().pos(-15.0 - *obj_pos, rtn_ypos)
            }
        } else {
            // normal trigger placement for everything else
            GDObjConfig::default()
                .pos(105.0 + *obj_pos, rtn_ypos)
                .groups([routine.group])
        }
        .multitrigger(true);

        let flag_assoc = instr
            .flags
            .iter()
            .map(|f| (f.ident.clone(), f))
            .collect::<HashMap<_, _>>();

        let handler = instr.handler_fn;
        let args = HandlerArgs {
            args: resolved_args,
            // assuming that all init instructions are before x=0,
            // which only doesnt happen if the triggers were manually moved,
            // then they all execute immediately at the start of the level,
            // hence they are "initializers".
            // there is nothing to spawn them, since they are on group 0
            // therefore, the "spawn triggered" option is omitted
            cfg: if routine.ident != INIT_ROUTINE {
                cfg.spawnable(true)
            } else {
                cfg
            },
            curr_group: self.curr_group, // used as auxiliary group
            ptr_group: self.ptr_group,
            ptr_reset_group: self.ptr_reset_group,
            line: instr.line_number,
            // these two are set only once a MALLOC instruction is processed
            // if there is no malloc, there is no memory access allowed
            // therefore it does not matter if there is junk data in there
            // since it will either be overwritten or never used
            memreg: &self.aliases.memreg,
            ptrpos_id: self.aliases.ptrpos_id,
            displayed_items: self.displayed_items,
            routine_count,
            mem_end_counter: self.mem_end_counter,

            flags: instr.flags.as_slice(),
            flag_by_ident: flag_assoc,

            mem_info: self.mem_info.as_ref(),
        };

        let data = match handler(args) {
            Ok(data) => data,
            Err(mut e) => {
                e.file = self.fname.clone();
                e.routine = routine.ident.clone();
                self.errors.push(e);
                return;
            }
        };
        for obj in data.objects.into_iter() {
            level.add_object(obj);
        }

        let skip_spaces = data.skip_spaces as f64 * spacing;
        self.curr_group += data.used_extra_groups;
        *obj_pos += skip_spaces;
        *previous_spacing_amount = skip_spaces;

        if data.added_item_display {
            self.displayed_items += 1;
        }

        // this if statement handles the logic of keeping track of the ptr group
        // it is necessary for instructions such as MRESET and MPTR which move the pointer
        // this information is only updated if it is set. this information is set
        // only in the malloc methods, which would be parsed before any mem ops

        if let Some(m) = data.new_mem {
            // check that memory does not already exist
            if self.mem_info.is_some() {
                push_error(
                    &mut self.errors,
                    &self.fname,
                    TasmErrorType::MultipleMemoryInstances,
                    instr.line_number,
                    routine.ident.clone(),
                    format!("Memory was already created on line {}.", m.line + 1),
                );
                return;
            }

            // assigning new mem info, also assign the aliases

            self.mem_info = Some(m);
            let mref = self.mem_info.as_ref().unwrap();
            // assign to alias map
            self.aliases.memreg = mref.memreg.clone();
            self.aliases.ptrpos_id = mref.ptrpos.to_counter_id().unwrap();
            self.aliases.memsize = mref.size;
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

pub fn push_error(
    errors: &mut Vec<TasmError>,
    file: &str,
    etype: TasmErrorType,
    line: usize,
    rtn: String,
    details: String,
) {
    errors.push(TasmError {
        etype,
        file: file.to_string(),
        routine: rtn,
        error: true,
        line,
        details,
    })
}

pub fn push_error_lineless(
    errors: &mut Vec<TasmError>,
    file: &str,
    etype: TasmErrorType,
    details: String,
) {
    errors.push(TasmError {
        etype,
        file: file.to_string(),
        routine: String::new(),
        error: true,
        line: 0,
        details,
    })
}

pub fn print_errors(es: Vec<TasmError>, err_msg: &str) {
    println!("{err_msg} with {} errors:", es.len());
    for e in es {
        println!("{e}");
    }
}

pub fn verify_assign(instr: &Instruction, args: &[TasmValue]) -> Result<(), String> {
    if instr.itype == InstrType::Arithmetic {
        // first argument is always the result
        let counter_type = get_item_spec(&args[0]).unwrap().get_type();
        if counter_type == ItemType::Attempts || counter_type == ItemType::MainTime {
            return Err(format!("Cannot overwrite value of {counter_type:?}."));
        }
    }
    Ok(())
}

pub fn resolve_aliases<'a>(instr: &'a Instruction, aliases: &Aliases) -> Cow<'a, [TasmValue]> {
    if instr.has_alias {
        let mut resolved = instr.args.clone();
        for v in &mut resolved {
            if let TasmValue::Alias(alias) = v {
                // builtin alias
                *v = aliases.get_value(*alias)
            };
        }
        Cow::Owned(resolved)
    } else {
        Cow::Borrowed(instr.args.as_slice())
    }
}

pub fn verify_memory_access(
    instr: &Instruction,
    routine_ident: &str,
    mem_info: &Option<MemInfo>,
) -> Result<(), (TasmErrorType, String)> {
    if instr.itype == InstrType::Memory {
        if routine_ident == INIT_ROUTINE {
            return Err((
                TasmErrorType::InitRoutineMemoryAccess,
                "Cannot access memory in the init routine.".to_string(),
            ));
        }
        if mem_info.is_none() {
            return Err((
                TasmErrorType::NonexistentMemoryAccess,
                "Cannot access memory when none exists.".to_string(),
            ));
        }
    }
    Ok(())
}
