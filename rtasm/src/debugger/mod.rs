use std::{time::Duration, vec};

use crate::core::{
    consts::INIT_ROUTINE,
    structs::{InstrIdent, InstrType, Instruction, Routine, Tasm},
};

use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use gdlib::gdobj::Item;
use ratatui::{
    self,
    layout::{Constraint, Layout},
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget},
    Frame,
};

pub mod ui;

pub fn emulate(tasm: Tasm) {
    if let Err(e) = Emulator::new(tasm).run() {
        println!("Emulator failed to run: {e}")
    };

    ratatui::restore();
}

#[derive(Debug)]
struct EmulatorState {
    counters: [i32; 9999],
    timers: [f32; 9999],
    attempts: i32,
    points: f32,
    maintime: f32,
}

impl Default for EmulatorState {
    fn default() -> Self {
        Self {
            counters: [0; 9999],
            timers: [0.; 9999],
            attempts: 0,
            points: 0.,
            maintime: 0.,
        }
    }
}

impl EmulatorState {
    fn new() -> Self {
        Self {
            counters: [0; 9999],
            timers: [0.0; 9999],
            attempts: 1,
            points: 0.0,
            maintime: 0.0,
        }
    }

    // returns the given item value as a string
    fn get_item_value(&self, item: Item) -> String {
        match item {
            Item::Counter(c) => self.counters[c as usize].to_string(),
            Item::Timer(t) => self.timers[t as usize].to_string(),
            Item::Attempts => self.attempts.to_string(),
            Item::MainTime => self.maintime.to_string(),
            Item::Points => self.points.to_string(),
        }
    }
}

#[derive(Debug, Default)]
struct Emulator {
    state: EmulatorState, // counter state
    tasm: Tasm,           // original compiled tasm
    running: bool,
    paused: bool, // true if paused. happens when tripping a breakpoint
    running_routines: Vec<RunningRoutine>, // all current running routines
    ioblocks: Vec<usize>, // idxs to self.tasm.routines
    ioblock_idx: usize,      // index into ioblocks
    displays: Vec<Item>,
    init_instrs: Vec<Instruction>, // executed every reset
    ticks: u32,                    // tick counter
    logbox: Vec<String>,           // box of messages from the emulator
}

impl Emulator {
    fn reset_state(&mut self) {
        self.state = EmulatorState {
            counters: [0i32; 9999],
            timers: [0.0f32; 9999],
            attempts: 1,
            points: 0.0f32,
            maintime: 0.0f32,
        }
    }

    pub fn new(tasm: Tasm) -> Self {
        let mut displays = vec![];
        let init_instrs =
            if let Some(rtn) = tasm.routines.iter().find(|rtn| rtn.ident == INIT_ROUTINE) {
                rtn.instructions
                    .iter()
                    .filter(|&instr| {
                        // all init instructions are encoded in some other way (e.g. MEMINFO)
                        // other than INITMEM, DISPLAY
                        instr.itype != InstrType::Init
                            || instr.ident == InstrIdent::INITMEM
                            || instr.ident == InstrIdent::DISPLAY
                    })
                    .filter_map(|i| {
                        if i.ident == InstrIdent::DISPLAY {
                            let item = i.args[0].to_item().unwrap();
                            displays.push(item);
                            None
                        } else {
                            Some(i.clone())
                        }
                    })
                    .collect()
            } else {
                vec![]
            };

        Self {
            state: EmulatorState::new(),
            tasm,
            running: true,
            init_instrs,
            displays,
            ..Default::default()
        }
    }

    pub fn run(&mut self) -> Result<()> {
        self.reset_state();
        self.setup();

        let mut terminal = ratatui::init();
        while self.running {
            terminal.draw(|frame| self.draw(frame))?;

            if !self.paused {
                self.tick();
            }
            
            if let Ok(c) = crossterm::event::poll(Duration::from_millis(0))
                && c
            {
                self.handle_key();
            }
        }

        Ok(())
    }

    fn setup(&mut self) {
        // this function is for setting up the state after a state reset
        // for stuff like running _init

        self.load_ioblocks();
        for instr in self.init_instrs.clone() {
            self.exec_instr(instr);
        }
    }

    fn load_ioblocks(&mut self) {
        // list of routines which have ioblocks
        // i.e. pointers to those routines
        let mut ioblocks = vec![];    // sentinel value for no ioblocks
        if let Some(init) = self
            .tasm
            .routines
            .iter()
            .find(|rtn| rtn.ident == INIT_ROUTINE)
        {
            for instr in &init.instructions[..] {
                if instr.ident != InstrIdent::IOBLOCK {
                    continue;
                }
                // get group of routine
                let target_rtn = instr.args[0].to_group_id().unwrap();
                if let Some((idx, _)) = self
                    .tasm
                    .routines
                    .iter()
                    .enumerate()
                    .find(|(_, rtn)| rtn.group == target_rtn)
                {
                    // push idx
                    ioblocks.push(idx);
                }
            }
        }
        if ioblocks.len() == 0 {
            ioblocks.push(usize::MAX)
        }
        
        self.ioblocks = ioblocks;
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area())
    }

    pub fn handle_key(&mut self) {
        let event = match crossterm::event::read() {
            Ok(e) => e,
            Err(_) => return,
        };

        match event {
            Event::Key(k) if !k.is_release() => {
                if k.modifiers.contains(KeyModifiers::CONTROL) {
                    self.handle_ctrl_key(k);
                    return;
                }

                self.handle_regular_key(k);
            }
            _ => {}
        }
    }

    fn handle_ctrl_key(&mut self, k: KeyEvent) {
        match k.code {
            KeyCode::Char('c') => {
                self.running = false;
                return;
            }
            _ => {}
        }
    }

    fn handle_regular_key(&mut self, k: KeyEvent) {
        match k.code {
            KeyCode::Esc => {
                self.running = false;
            },
            KeyCode::Char(' ') => {
                self.paused = !self.paused;
            },
            KeyCode::Up => {
                if self.ioblock_idx > 0 {
                    self.ioblock_idx -= 1;
                }
            },
            KeyCode::Down => {
                if self.ioblock_idx < self.ioblocks.len() - 1 {
                    self.ioblock_idx += 1;
                }
            }
            _ => {}
        }
    }

    pub fn tick(&mut self) {
        let mut instrs_todo = vec![];
        for routine in self.running_routines.iter_mut() {
            if routine.waiting > 0 {
                routine.waiting -= 1;
                continue;
            }

            // otherwise, increment instruction ptr
            if routine.instr_ptr < routine.routine.instructions.len() - 1 {
                // todo: figure out concurrent instructions
                routine.instr_ptr += 1;
                instrs_todo.push(routine.routine.instructions[routine.instr_ptr].clone());
            } else {
                // end routine
                routine.done = true;
            }
        }

        for instr in instrs_todo {
            self.exec_instr(instr);
        }

        self.running_routines.retain(|r| !r.done);
        self.ticks += 1;
    }

    fn add_log(&mut self, log: String) {
        self.logbox.push(log);
    }

    fn exec_instr(&mut self, instr: Instruction) {
        match instr.ident {
            _ => self.add_log(format!(
                "Instruction {:?} is not implemented in the emulator yet.",
                instr.ident
            )),
        }
    }
}

#[derive(Debug, Default)]
struct RunningRoutine {
    routine: Routine,
    instr_ptr: usize,
    waiting: i32, // how many ticks it is waiting
    done: bool,
}
