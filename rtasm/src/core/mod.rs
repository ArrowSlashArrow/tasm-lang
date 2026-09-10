use gdlib::{
    gdlevel::Level,
    gdobj::{GDObjConfig, ItemType, misc::text},
};

use crate::{
    core::{
        consts::{ENTRY_POINT, GROUP_LIMIT, INIT_ROUTINE},
        error::{TasmError, TasmErrorType},
        structs::{HandlerArgs, HandlerData, InstrType, Instruction, Routine, Tasm, TasmValue},
    },
    instr::{fns::ioblock, get_item_spec},
};

extern crate alloc;
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
    // pub fn handle_routines(
    //     &mut self,
    //     level_name: &str,
    //     skip_init: bool,
    // ) -> Result<Level, Vec<TasmError>> {
    //     self.handle_routines_inner(
    //         level_name,
    //         self.routines.len() as i16 + self.group_offset + 1,
    //         skip_init,
    //     )
    // }

    pub fn handle_routines_inner(
        &mut self,
        level_name: &str,
        start_at_group: i16,
        skip_init: bool,
    ) -> Result<Level, Vec<TasmError>> {
        let spacing = match self.release_mode {
            true => 1.0,
            false => 30.0,
        };

        // setup state
        let mut level = Level::new(level_name, "tasm", None, None);
        self.curr_group = start_at_group;

        // need to take to iteration with mutable references to self in self.push_error and self.handle_instruction
        let routines = core::mem::take(&mut self.routines);
        for routine in &routines {
            if routine.ident == INIT_ROUTINE && skip_init {
                continue;
            }

            // setup position variables
            let mut obj_pos = 0.0;
            // subtracting from group offset ensures that high group IDs are still placed close to y=0
            let rtn_ypos = if routine.group != 0 {
                75.0 + ((routine.group - self.group_offset) as f64) * 30.0
            } else {
                75.0
            };
            if self.curr_group > GROUP_LIMIT {
                push_error_lineless(
                    &mut self.errors,
                    &self.fname,
                    TasmErrorType::ExceedsGroupLimit,
                    format!("Program uses more than {GROUP_LIMIT} groups."),
                    1,
                );
                break;
            }

            // keep track of entry group
            if routine.ident == ENTRY_POINT {
                self.start_rtn_group = routine.group;
            }

            if routine.ident != INIT_ROUTINE {
                // routine marker
                level.add_object(text(
                    &GDObjConfig::new().pos(0.0, rtn_ypos).scale(0.6, 0.6),
                    format!("{}: {}", routine.group, routine.ident),
                    0,
                ));
            }

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
                    &mut level,
                );
            }
        }
        self.routines = routines;

        if self.start_rtn_group != 0 {
            let ioblock_result = ioblock(HandlerArgs {
                args: &[
                    TasmValue::Group(self.start_rtn_group),
                    TasmValue::Number(0.0),
                    TasmValue::String("start".into()),
                ],
                cfg: GDObjConfig::new(),
                displayed_items: self.displayed_items,
                curr_group: self.curr_group,
                ..Default::default()
            })
            .unwrap();

            // add starting block
            for obj in ioblock_result.objects.into_iter() {
                level.add_object(obj);
            }
        }

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
        level: &mut Level,
    ) {
        // check that any bad assignments aren't happening
        if instr.itype == InstrType::Arithmetic {
            // arithmetic instructions always write back to first argument
            let counter_type = get_item_spec(&instr.args[0]).unwrap().get_type();
            if counter_type == ItemType::Attempts || counter_type == ItemType::MainTime {
                push_error(
                    &mut self.errors,
                    &self.fname,
                    TasmErrorType::InvalidAssignment,
                    instr.line_number,
                    routine.ident.clone(),
                    format!("Cannot overwrite value of {counter_type:?}."),
                    4,
                );
                return;
            }
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
            args: &instr.args[..],
            cfg: if routine.ident != INIT_ROUTINE {
                cfg.spawnable(true)
            } else {
                // init stuff doesn't get spawned normally
                // though it is really up to the programmer what they wanna do with it
                cfg
            },
            curr_group: self.curr_group, // used as auxiliary group
            line: instr.line_number,
            displayed_items: self.displayed_items,
            flag_by_ident: flag_assoc,
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
    }
}

pub fn push_error(
    errors: &mut Vec<TasmError>,
    file: &str,
    etype: TasmErrorType,
    line: usize,
    rtn: String,
    details: String,
    errcode: i32,
) {
    errors.push(TasmError {
        etype,
        file: file.to_string(),
        routine: rtn,
        errcode,
        line,
        details,
    })
}

pub fn push_error_lineless(
    errors: &mut Vec<TasmError>,
    file: &str,
    etype: TasmErrorType,
    details: String,
    errcode: i32,
) {
    errors.push(TasmError {
        etype,
        file: file.to_string(),
        routine: String::new(),
        errcode,
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
