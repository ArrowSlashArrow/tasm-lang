use std::{time::Duration, vec};

use crate::core::{
    consts::INIT_ROUTINE,
    structs::{InstrIdent, InstrType, Instruction, Routine, Tasm},
};

use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use gdlib::gdobj::Item;
use ratatui::{self, Frame};
use std::mem::take;

pub mod instr;
pub mod ui;

const TOGGLE_KEYS: &[KeyCode] = &[KeyCode::Tab];

pub fn emulate(tasm: Tasm) {
    if let Err(e) = Emulator::new(tasm).run() {
        println!("Emulator failed to run: {e}")
    };

    ratatui::restore();
}

#[derive(Debug)]
pub struct EmulatorState {
    counters: [i32; 10_000],
    timers: [f32; 10_000],
    attempts: i32,
    points: f32,
    maintime: f32,
}

impl Default for EmulatorState {
    fn default() -> Self {
        Self {
            counters: [0; 10_000],
            timers: [0.; 10_000],
            attempts: 0,
            points: 0.,
            maintime: 0.,
        }
    }
}

impl EmulatorState {
    // returns the given item value as a string
    fn get_item_value_str(&self, item: Item) -> String {
        match item {
            Item::Counter(c) => self.counters[c as usize].to_string(),
            Item::Timer(t) => self.timers[t as usize].to_string(),
            Item::Attempts => self.attempts.to_string(),
            Item::MainTime => self.maintime.to_string(),
            Item::Points => self.points.to_string(),
        }
    }

    fn set_item(&mut self, item: Item, num: f64) {
        match item {
            Item::Counter(c) => self.counters[c as usize] = num as i32,
            Item::Timer(t) => self.timers[t as usize] = num as f32,
            Item::Attempts | Item::MainTime => {}
            Item::Points => self.points = num as f32,
        }
    }

    fn get_num(&self, item: Item) -> f64 {
        match item {
            Item::Counter(c) => self.counters[c as usize] as f64,
            Item::Timer(t) => self.timers[t as usize] as f64,
            Item::Attempts => self.attempts as f64,
            Item::MainTime => self.maintime as f64,
            Item::Points => self.points as f64,
        }
    }
}

#[derive(Debug, Default)]
pub struct Emulator {
    state: EmulatorState, // counter state
    tasm: Tasm,           // original compiled tasm
    running: bool,
    paused: bool, // true if paused. happens when tripping a breakpoint
    running_routines: Vec<RunningRoutine>, // all current running routines
    ioblocks: Vec<usize>, // idxs to self.tasm.routines
    ioblock_idx: usize, // index into ioblocks
    peeking_ioblock: bool, // whether to peek the selected ioblock (see instructions of that routine)
    displays: Vec<Item>,
    init_instrs: Vec<Instruction>, // executed every reset
    ticks: u32,                    // tick counter
    logbox: Vec<String>,           // box of messages from the emulator
}

impl Emulator {
    fn reset_state(&mut self) {
        self.state = EmulatorState::default();
        self.running_routines.clear();
        self.ticks = 0;
    }

    pub fn new(tasm: Tasm) -> Self {
        let mut displays = vec![];
        let init_instrs =
            if let Some(rtn) = tasm.routines.iter().find(|rtn| rtn.ident == INIT_ROUTINE) {
                rtn.instructions
                    .iter()
                    .filter(|&instr| {
                        // all init instructions are encoded in some other way (e.g. meminfo)
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
            state: EmulatorState::default(),
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
            // fix timing to be at 240hz
        }

        Ok(())
    }

    fn setup(&mut self) {
        // this function is for setting up the state after a state reset
        // for stuff like running _init

        self.load_ioblocks();
        let instrs = take(&mut self.init_instrs);

        for instr in instrs.iter() {
            self.exec_instr(instr);
        }

        self.init_instrs = instrs;
    }

    fn load_ioblocks(&mut self) {
        // list of routines which have ioblocks
        // i.e. pointers to those routines
        let mut ioblocks = vec![];
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
                if let Some((routine_idx, _)) = self
                    .tasm
                    .routines
                    .iter()
                    .enumerate()
                    .find(|(_, rtn)| rtn.group == target_rtn)
                {
                    ioblocks.push(routine_idx);
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
            Event::Key(k) => {
                if TOGGLE_KEYS.contains(&k.code) {
                    self.handle_toggle_key(k);
                    return;
                }

                // filter release
                if k.is_release() {
                    return;
                }

                if k.modifiers.contains(KeyModifiers::CONTROL) {
                    if let KeyCode::Char('c') = k.code {
                        self.running = false;
                    }
                    return;
                }

                self.handle_keypress(k);
            }
            _ => {}
        }
    }

    fn handle_toggle_key(&mut self, k: KeyEvent) {
        match k.kind {
            // on state
            KeyEventKind::Press => match k.code {
                KeyCode::Tab => {
                    self.peeking_ioblock = true;
                }
                _ => {}
            },
            // off state
            KeyEventKind::Release => match k.code {
                KeyCode::Tab => {
                    self.peeking_ioblock = false;
                }
                _ => {}
            },
            KeyEventKind::Repeat => {}
        }
    }

    fn handle_keypress(&mut self, k: KeyEvent) {
        match k.code {
            KeyCode::Esc => self.running = false,
            KeyCode::Char(' ') => self.paused = !self.paused,
            KeyCode::Up => {
                if self.ioblock_idx > 0 {
                    self.ioblock_idx -= 1;
                }
            }
            KeyCode::Down => {
                if self.ioblock_idx < self.ioblocks.len() - 1 {
                    self.ioblock_idx += 1;
                }
            }
            KeyCode::PageUp => self.ioblock_idx = 0,
            KeyCode::PageDown => self.ioblock_idx = self.ioblocks.len() - 1,
            KeyCode::Enter => {
                let routine_idx = self.ioblocks[self.ioblock_idx];
                // happens if there are no ioblocks
                if routine_idx == usize::MAX {
                    return;
                }
                let routine = self.tasm.routines[routine_idx].clone();
                self.add_running_routine(routine);
            }
            KeyCode::Char('.') => {
                if self.paused {
                    self.tick();
                }
            }
            KeyCode::Char('c') => self.logbox.clear(),
            KeyCode::Char('r') => {
                self.reset_state();
                self.setup();
            }
            // kc @ (_) => {
            //     self.add_log(format!("Key {kc:?} is not yet supported."));
            // }
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
            if routine.instr_ptr < routine.routine.instructions.len() {
                // todo: figure out concurrent instructions
                instrs_todo.push(routine.routine.instructions[routine.instr_ptr].clone());
                routine.instr_ptr += 1;
            }
            if routine.instr_ptr == routine.routine.instructions.len() {
                // end routine
                routine.done = true;
            }
        }

        for instr in instrs_todo.iter() {
            self.exec_instr(instr);
        }

        self.running_routines.retain(|r| !r.done);
        self.ticks += 1;
    }

    fn add_log(&mut self, log: String) {
        self.logbox.push(log);
    }

    fn exec_instr(&mut self, instr: &Instruction) {
        (instr.handler_fn_emu)(self, instr);
    }

    fn add_running_routine(&mut self, routine: Routine) {
        self.add_log(format!("Spawned routine {}", routine.ident));
        self.running_routines.push(RunningRoutine::new(routine));
    }
}

#[derive(Debug, Default)]
pub struct RunningRoutine {
    routine: Routine,
    instr_ptr: usize,
    waiting: i32, // how many ticks it is waiting
    done: bool,
}

impl RunningRoutine {
    pub fn new(routine: Routine) -> Self {
        Self {
            routine,
            instr_ptr: 0,
            waiting: 0,
            done: false,
        }
    }

    pub fn get_line(&self) -> usize {
        self.routine.instructions[self.instr_ptr].line_number
    }
}
