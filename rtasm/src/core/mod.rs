use gdlib::{
    gdlevel::Level,
    gdobj::{GDObjConfig, ItemType, misc::text},
};

use crate::{
    core::{
        consts::{ENTRY_POINT, GROUP_LIMIT, INIT_ROUTINE},
        error::TasmParseError,
        structs::{HandlerArgs, HandlerData, InstrType, Tasm, TasmValue},
    },
    instr::{fns::ioblock, get_item_spec},
};

pub mod consts {
    pub const ENTRY_POINT: &str = "_start";
    pub const INIT_ROUTINE: &str = "_init";
    pub const GROUP_LIMIT: i16 = 9_999;
}
pub mod error;
pub mod flags;
pub mod structs;

pub type HandlerReturn = Result<HandlerData, TasmParseError>;
pub type HandlerFn = fn(HandlerArgs) -> HandlerReturn;

#[macro_export]
macro_rules! verbose_log {
    ($this:expr, $($arg:tt)*) => {
        if $this.logs_enabled {
            println!($($arg)*);
        }
    };
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

            let mut previous_spacing_amount = 0.0;

            // starting position of objects: (15, 75 + curr_group * 15)
            for instr in routine.instructions.iter() {
                let mut instr_args = instr.args.clone();
                instr_args.iter_mut().for_each(|v| {
                    if let TasmValue::Alias(alias) = v {
                        // builtin alias
                        *v = self.aliases.get_value(*alias)
                    }
                });

                // check that we are not accessing memory in init routine
                if instr._type == InstrType::Memory {
                    if routine.ident == INIT_ROUTINE {
                        self.errors
                            .push(TasmParseError::InitRoutineMemoryAccess(instr.line_number));
                        continue;
                    }
                    if self.mem_info.is_none() {
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

                // do not increment x-position if this instruction is concurrent.
                // in a concurrent chain, all instructions before the last one
                // will be ignored for extra spacing. therefore, it is the responsibility
                // of the programmer to manage timing with concurrent instructions.
                if instr.is_concurrent {
                    // move back
                    obj_pos -= previous_spacing_amount;
                }

                let cfg = if routine.ident == INIT_ROUTINE {
                    if let InstrType::Init = instr._type {
                        // in the case of a custom init structure,
                        // leave default obj config since it likely wont be used anyways
                        GDObjConfig::default()
                    } else {
                        // in the case of a normal position-dependent instruction
                        // negate usual position to place normal triggers in init routine
                        // before the x=0 line to make the instantly execute at the level start
                        GDObjConfig::default().pos(-15.0 - obj_pos, rtn_ypos)
                    }
                } else {
                    // normal trigger placement for everything else
                    GDObjConfig::default()
                        .pos(105.0 + obj_pos, rtn_ypos)
                        .groups([routine.group])
                }
                .multitrigger(true);

                let handler = instr.handler_fn;
                let args = HandlerArgs {
                    args: instr_args,
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
                    memreg: self.aliases.memreg.clone(),
                    ptrpos_id: self.aliases.ptrpos_id,
                    displayed_items: self.displayed_items,
                    routine_count,
                    mem_end_counter: self.mem_end_counter,
                    flags: instr.flags.clone(),
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

                let skip_spaces = data.skip_spaces as f64 * spacing;
                self.curr_group += data.used_extra_groups;
                obj_pos += skip_spaces;
                previous_spacing_amount = skip_spaces;

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
                ..Default::default()
            })
            .unwrap();

            // add starting block
            for obj in ioblock_result.objects.into_iter() {
                level.add_object(obj);
            }
        }

        if !self.errors.is_empty() {
            Err(self.errors.clone())
        } else {
            Ok(level)
        }
    }
}

pub fn show_errors(es: Vec<TasmParseError>, err_msg: &str) {
    println!("{err_msg} with {} errors:", es.len());
    for e in es {
        println!("{e}");
    }
}
