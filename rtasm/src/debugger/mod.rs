use std::{
    time::{Duration, Instant},
    vec,
};

use crate::core::{
    consts::INIT_ROUTINE,
    resolve_aliases,
    structs::{InstrIdent, InstrType, Instruction, MemType, Routine, Tasm},
};

use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use gdlib::gdobj::Item;
use ratatui::{self, Frame};
use std::mem::take;

pub mod instr;
pub mod ui;

const TOGGLE_KEYS: &[KeyCode] = &[KeyCode::Tab];
// to find how many keybinds it should take to double the speed,
// use f(x) = 2^(1/x)
// using f(5) here
const HZ_SCALE: f64 = 1.148698355;
// seconds in a tick
const TICK_LENGTH: f32 = 0.00416666667;
const TICKS_PER_SECOND: f64 = 240.0;

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
            Item::Timer(t) => format!("{:.6}", self.timers[t as usize]),
            Item::Attempts => self.attempts.to_string(),
            Item::MainTime => self.maintime.to_string(),
            Item::Points => self.points.to_string(),
        }
    }

    fn set_item(&mut self, item: Item, num: f64) {
        match item {
            Item::Counter(c) => self.counters[c as usize] = num as i32,
            Item::Timer(t) => self.timers[t as usize] = (num as f32),
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
    running: bool,
    state: EmulatorState,                  // counter state
    tasm: Tasm,                            // original compiled tasm
    paused: bool,                          // happens when tripping a breakpoint
    running_routines: Vec<RunningRoutine>, // all current running routines
    ioblocks: Vec<usize>,                  // idxs to self.tasm.routines
    ioblock_idx: usize,                    // index into ioblocks
    peeking_ioblock: bool, // whether to peek the selected ioblock (see instructions of that routine)
    displays: Vec<Item>,
    init_instrs: Vec<Instruction>,     // executed every reset
    ticks: u32,                        // tick counter
    hz: f64,                           // ticks per second
    lagging: bool,                     // whether the emulator is lagging behind
    last_tick_time: Duration,          // how long the previous tick took to run
    logbox: Vec<String>,               // box of messages from the emulator
    ticking_timers: Vec<TickingTimer>, // list of timers that are currently ticking
    started_timers: Vec<i16>,          // list of timer ids that have been initailized
    // list of groups that are toggled off
    // before spawning a group, check that it isn't in here
    // if there *is* an active process with this group, toggle it off
    toggled_groups: Vec<i16>,
    /// Tracks what mode the memory is in right now
    legacy_memstate: LegacyMemstate,
}

#[derive(Debug, Default)]
pub enum LegacyMemstate {
    #[default]
    None,
    Read,
    Write,
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
            paused: true,
            hz: TICKS_PER_SECOND,
            ..Default::default()
        }
    }

    pub fn run(&mut self) -> Result<()> {
        self.reset_state();
        self.setup();

        let mut terminal = ratatui::init();

        let mut next_tick_time;
        while self.running {
            next_tick_time = Instant::now();
            terminal.draw(|frame| self.draw(frame))?;
            if !self.paused {
                self.tick();
            }

            if let Ok(c) = crossterm::event::poll(Duration::from_millis(0))
                && c
            {
                self.handle_key();
            }

            // hold specified timing
            self.last_tick_time = next_tick_time.elapsed();
            next_tick_time += Duration::from_nanos((1_000_000_000.0 / self.hz) as u64);
            if Instant::now() > next_tick_time {
                self.lagging = true;
                continue;
            } else {
                self.lagging = false;
            }
            while Instant::now() < next_tick_time {
                std::hint::spin_loop();
            }
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
            KeyCode::Char('-') => self.hz /= HZ_SCALE,
            KeyCode::Char('=') => self.hz *= HZ_SCALE,
            KeyCode::Char('0') => self.hz = TICKS_PER_SECOND,
            // kc @ (_) => {
            //     self.add_log(format!("Key {kc:?} is not yet supported."));
            // }
            _ => {}
        }
    }

    pub fn tick(&mut self) {
        self.running_routines.retain(|r| !r.done);
        let mut instrs_todo = vec![];
        let mut waits_todo = vec![];
        for (rtn_idx, routine) in self.running_routines.iter_mut().enumerate() {
            if routine.paused {
                continue;
            }
            if routine.waiting > 0 {
                routine.waiting -= 1;
                if routine.waiting > 0 {
                    continue;
                }
            }

            // otherwise, increment instruction ptr
            if routine.instr_ptr < routine.routine.instructions.len() {
                // todo: figure out concurrent instructions
                if routine.toggled {
                    instrs_todo.push((
                        rtn_idx,
                        routine.routine.instructions[routine.instr_ptr].clone(),
                    ));
                }
                // progression still happens even if routine is not toggled
                let instr = &routine.routine.instructions[routine.instr_ptr];
                let wait_time = get_time(instr);
                waits_todo.push((instr.parent_running_routine_idx, wait_time));
                routine.instr_ptr += 1;
            }
            if routine.instr_ptr == routine.routine.instructions.len() {
                // end routine
                routine.done = true;
            }
        }

        for (parent, instr) in instrs_todo.iter_mut() {
            let resolved_args = resolve_aliases(&self.tasm.aliases, &instr);
            instr.args = resolved_args.to_vec();
            instr.parent_running_routine_idx = *parent;
            self.exec_instr(instr);
        }

        for (parent, wait) in waits_todo {
            self.wait_ticks(parent, wait);
        }

        let mut spawns_todo = vec![];
        for timer in self.ticking_timers.iter() {
            if timer.paused {
                continue;
            }
            let time = self.state.timers.get_mut(timer.id as usize).unwrap();
            *time += TICK_LENGTH;
            if *time >= timer.target_time {
                spawns_todo.push((timer.group, timer.id));
            }
        }

        for (sp, id) in spawns_todo {
            self.spawn_group(sp);
            self.ticking_timers.retain(|t| t.id != id);
        }

        self.ticks += 1;
    }

    fn wait_ticks(&mut self, rtn_idx: usize, ticks: i32) {
        if let Some(rtn) = self.running_routines.get_mut(rtn_idx) {
            rtn.waiting = ticks;
        }
    }

    fn add_log(&mut self, log: String) {
        self.logbox.push(format!("[{}] {log}", self.ticks));
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
    paused: bool,
    toggled: bool, // true: on; vv.
}

impl RunningRoutine {
    pub fn new(routine: Routine) -> Self {
        Self {
            routine,
            instr_ptr: 0,
            waiting: 0,
            done: false,
            paused: false,
            toggled: true,
        }
    }

    pub fn get_line(&self) -> usize {
        self.routine.instructions[self.instr_ptr].line_number
    }
}

#[derive(Debug, Default)]
pub struct TickingTimer {
    pub id: i16,
    pub group: i16,
    pub target_time: f32,
    pub paused: bool,
}

pub fn get_time(instr: &Instruction) -> i32 {
    match instr.ident {
        /* never executed */
        InstrIdent::MALLOC => 0,
        InstrIdent::FMALLOC => 0,
        InstrIdent::INITMEM => 0,
        InstrIdent::PERS => 0,
        InstrIdent::DISPLAY => 0,
        InstrIdent::IOBLOCK => 0,
        InstrIdent::LMALLOC => 0,
        InstrIdent::LFMALLOC => 0,
        /* execution time in ticks */
        InstrIdent::LMFUNC => 2,
        InstrIdent::LMREAD => 1,
        InstrIdent::LMWRITE => 1,
        InstrIdent::LMPTR => 1,
        InstrIdent::LMRESET => 1,
        InstrIdent::MOV => 1,
        InstrIdent::MSET => 4,
        InstrIdent::MGET => 4,
        InstrIdent::BREAKPOINT => 1,
        InstrIdent::SPAWN => 1,
        InstrIdent::NOP => 1,
        InstrIdent::WAIT => instr.args[0].to_int().unwrap(),
        InstrIdent::WAITS => (instr.args[0].to_float().unwrap() * TICKS_PER_SECOND) as i32,
        InstrIdent::ADD => 1,
        InstrIdent::SUB => 1,
        InstrIdent::ADDM => 1,
        InstrIdent::SUBM => 1,
        InstrIdent::ADDD => 1,
        InstrIdent::SUBD => 1,
        InstrIdent::MUL => 1,
        InstrIdent::DIV => 1,
        InstrIdent::FLDIV => 1,
        InstrIdent::SE => 2,
        InstrIdent::SNE => 2,
        InstrIdent::SL => 2,
        InstrIdent::SLE => 2,
        InstrIdent::SG => 2,
        InstrIdent::SGE => 2,
        InstrIdent::FE => 2,
        InstrIdent::FNE => 2,
        InstrIdent::FL => 2,
        InstrIdent::FLE => 2,
        InstrIdent::FG => 2,
        InstrIdent::FGE => 2,
        InstrIdent::SRAND => 2,
        InstrIdent::FRAND => 2,
        InstrIdent::TSPAWN => 1,
        InstrIdent::TSTART => 1,
        InstrIdent::TSTOP => 1,
        InstrIdent::PAUSE => 1,
        InstrIdent::RESUME => 1,
        InstrIdent::KILL => 1,
        InstrIdent::TOGGLEON => 1,
        InstrIdent::TOGGLEOFF => 1,
        InstrIdent::RAW => 0,
    }
}

impl Emulator {
    pub fn write_mem(&mut self, addr: i16, value: f64) {
        match self.tasm.mem_info {
            None => return,
            Some(ref mem) => {
                // get true counter
                let true_addr = match mem._type {
                    MemType::LegacyInt | MemType::Int => Item::Counter(mem.start_counter_id + addr),
                    MemType::LegacyFloat | MemType::Float => {
                        Item::Timer(mem.start_counter_id + addr)
                    }
                };

                self.state.set_item(true_addr, value);
            }
        }
    }

    pub fn read_mem(&self, addr: i16) -> f64 {
        match self.tasm.mem_info {
            None => 0.0,
            Some(ref mem) => {
                // get true counter
                let true_addr = match mem._type {
                    MemType::LegacyInt | MemType::Int => Item::Counter(mem.start_counter_id + addr),
                    MemType::LegacyFloat | MemType::Float => {
                        Item::Timer(mem.start_counter_id + addr)
                    }
                };

                self.state.get_num(true_addr)
            }
        }
    }
}
