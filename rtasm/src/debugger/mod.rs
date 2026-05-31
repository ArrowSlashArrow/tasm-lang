use alloc::vec;
use core::time::Duration;
use std::{
    borrow::Cow,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::Receiver,
    },
    thread::JoinHandle,
    time::Instant,
};

use crate::{
    core::{
        consts::INIT_ROUTINE,
        flags::Flag,
        resolve_aliases,
        structs::{Aliases, InstrIdent, InstrType, Instruction, MemInfo, Routine, Tasm, TasmValue},
    },
    debugger::{RoutineCommand::Spawn, layout::PrecomputedLayout},
};

use anyhow::Result;
use core::mem::take;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, poll};
use gdlib::gdobj::Item;
use ratatui::{self, Frame};

pub mod instr;
pub mod layout;
pub mod ui;

const TOGGLE_KEYS: &[KeyCode] = &[KeyCode::Tab];
// to find how many keybinds it should take to double the speed,
// use f(x) = 2^(1/x)
// using f(5) here
const HZ_SCALE: f64 = 1.148698355;
// seconds in a tick
const TICK_LENGTH: f32 = 0.004_166_667;
const TICKS_PER_SECOND: f64 = 240.0;

pub fn emulate(tasm: Tasm) {
    if let Err(e) = Emulator::new(tasm).run() {
        println!("Emulator failed to run: {e}")
    };

    ratatui::restore();
}

#[derive(Debug)]
pub struct CounterState {
    counters: [i32; 10_000],
    timers: [f32; 10_000],
    attempts: i32,
    points: f32,
    maintime: f32,
}

impl Default for CounterState {
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

impl CounterState {
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
    tasm: Tasm, // original compiled tasm. should **NEVER** be mutated.
    running: bool,
    paused: bool, // happens when tripping a breakpoint
    /// The active, changing state
    state: EmulatorState, // counter state
    ioblocks: Vec<usize>, // idxs to self.tasm.routines
    ui_state: UIState,
    ticks: u64, // tick counter
    hz: f64,    // ticks per second
    eventh: Option<EventHandler>,
    // not in self.state for optimization reasons
    running_routines: Vec<RunningRoutine>, // all current running routines
}

#[derive(Debug, Default)]
pub struct UIState {
    displays: Vec<Item>,
    peeking_ioblock: bool, // whether to peek the selected ioblock (see instructions of that routine)
    ioblock_idx: usize,    // index into ioblocks
    lagging: bool,         // whether the emulator is lagging behind
    last_tick_time: Duration, // how long the previous tick took to run
    logbox: Vec<String>,   // box of messages from the emulator
    selected_routine: usize, // Emulator.running_routines[idx + 1]
    true_tick_count: u64, // keeps track of ALL ticks that have passed since the start of the program
    passed_ticks: f64,    // amount of ticks that have passed since the last ui update.
    layout: PrecomputedLayout,
    tpf: f64,          // cached value for ticks per frame
    tick_ns: Duration, // cached value for nanoseconds per tick
    unlimited_speed: bool,
    last_ui_update: Option<Instant>,
    struct_field_0xe: Duration,
    last_seen_ticks: u64,
    elapsed_ticks: u64,
    poll_interval: u64,
}

#[derive(Debug, Default)]
pub struct EmulatorState {
    ticking_timers: Vec<TickingTimer>, // list of timers that are currently ticking
    started_timers: Vec<i16>,          // list of timer ids that have been initailized
    init_instrs: Vec<Instruction>,     // executed every reset
    // list of groups that are toggled off
    // before spawning a group, check that it isn't in here
    // if there *is* an active process with this group, toggle it off
    toggled_groups: Vec<i16>,
    timer_spawns_todo: Vec<(i16, i16)>,
    counter_state: CounterState,
    mem_info: Option<MemInfo>,
    /// Tracks what mode the memory is in right now
    legacy_memstate: LegacyMemstate,
    /// This field exists to allow for `self::add_running_routine`
    routine_groups: Vec<i16>,
    /// Temporary container for logs created from instructions that will be forwarded to the master struct
    temp_logbox: Vec<String>,
    /// Temporary container for `Emulator.paused`
    temp_paused: bool,
    /// Stores all routine commands that were committed this tick
    temp_routine_commands: Vec<RoutineCommand>,
}

#[derive(Debug)]
pub struct EventHandler {
    rx: Receiver<Event>,
    thread_handle: JoinHandle<()>,
    stop_flag: Arc<AtomicBool>,
}

#[derive(Debug, Default)]
pub enum LegacyMemstate {
    #[default]
    None,
    Read,
    Write,
}

#[derive(Debug)]
pub enum RoutineCommand {
    Spawn(usize),
    Pause(i16),
    Resume(i16),
    Kill(i16),
    ToggleOff(i16),
    ToggleOn(i16),
}

impl Emulator {
    fn reset_state(&mut self) {
        self.state.counter_state = CounterState::default();
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
            running: true,
            state: EmulatorState {
                init_instrs,
                mem_info: tasm.mem_info.clone(),
                routine_groups: tasm.routines.iter().map(|r| r.group).collect::<Vec<_>>(),
                ..Default::default()
            },
            tasm,
            ui_state: UIState {
                displays,
                layout: PrecomputedLayout::new(),
                last_ui_update: Some(Instant::now()),
                poll_interval: 1000000,
                ..Default::default()
            },
            paused: true,
            hz: TICKS_PER_SECOND,
            ..Default::default()
        }
    }

    fn update_ticks_per_frame(&mut self) {
        self.ui_state.tpf = self.hz / 60.0;
        self.ui_state.tick_ns = Duration::from_nanos((1_000_000_000.0 / self.hz) as u64);
    }

    fn next_tick(&self) -> bool {
        let ui = &self.ui_state;
        if ui.unlimited_speed {
            ui.true_tick_count % ui.poll_interval == 0
        } else {
            ui.passed_ticks > ui.tpf
        }
    }

    pub fn run(&mut self) -> Result<()> {
        self.reset_state();
        self.setup();
        self.start_event_polling();
        self.update_ticks_per_frame();

        let mut terminal = ratatui::init();

        let mut next_tick_time = Instant::now();
        while self.running {
            if !self.ui_state.unlimited_speed {
                next_tick_time = Instant::now();
            }

            if self.next_tick() {
                terminal.draw(|frame| {
                    if self.ui_state.layout.is_dirty {
                        self.ui_state.layout.compute(frame.area());
                    }
                    let logbox_height = (self.ui_state.layout.logbox_area.height - 2) as usize;

                    if self.ui_state.logbox.len() > logbox_height {
                        self.ui_state
                            .logbox
                            .drain(..self.ui_state.logbox.len() - logbox_height);
                    }

                    self.draw(frame)
                })?;
                self.ui_state.passed_ticks -= self.ui_state.tpf;
                let elapsed = self.ui_state.last_ui_update.unwrap().elapsed();
                if elapsed > Duration::from_millis(500) {
                    self.ui_state.struct_field_0xe = elapsed;
                    self.ui_state.elapsed_ticks =
                        self.ui_state.true_tick_count - self.ui_state.last_seen_ticks;
                    self.ui_state.last_ui_update = Some(Instant::now());
                    self.ui_state.last_seen_ticks = self.ui_state.true_tick_count;
                }
            }

            if !self.paused {
                self.tick();
            }

            if let Ok(event) = self.eventh.as_ref().unwrap().rx.try_recv() {
                self.handle_event(event);
            }

            self.ui_state.passed_ticks += 1.0;
            self.ui_state.true_tick_count += 1;

            if self.ui_state.unlimited_speed {
                // at 2MHz, timing the next tick becomes counter-productive because
                // std::time::Instant::now() takes ~50ns itself
                // therefore, skip it
                continue;
            }

            // hold specified timing
            let now = Instant::now();
            self.ui_state.last_tick_time = next_tick_time.elapsed();
            next_tick_time += self.ui_state.tick_ns;

            if now > next_tick_time {
                self.ui_state.lagging = true;
            } else {
                self.ui_state.lagging = false;
                while Instant::now() < next_tick_time {
                    core::hint::spin_loop();
                }
            }
        }

        let handle = self.eventh.take().unwrap();
        handle.stop_flag.store(true, Ordering::Relaxed);
        let _ = handle.thread_handle.join();

        Ok(())
    }

    fn start_event_polling(&mut self) {
        let (tx, rx) = std::sync::mpsc::channel::<Event>();

        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop_clone = stop_flag.clone();

        let thread = std::thread::spawn(move || {
            loop {
                if stop_clone.load(Ordering::Relaxed) {
                    break;
                }
                if let Ok(c) = poll(Duration::from_millis(1000))
                    && c
                {
                    if let Err(_) = tx.send(crossterm::event::read().unwrap()) {
                        break;
                    }
                }
            }
        });

        self.eventh = Some(EventHandler {
            rx,
            thread_handle: thread,
            stop_flag,
        })
    }

    fn setup(&mut self) {
        // this function is for setting up the state after a state reset
        // for stuff like running _init

        self.load_ioblocks();

        let instrs = take(&mut self.state.init_instrs);

        for instr in instrs.iter() {
            self.state
                .exec_instr(&instr, &self.tasm.aliases, &self.tasm.routines);
        }

        self.state.init_instrs = instrs;
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
        if ioblocks.is_empty() {
            ioblocks.push(usize::MAX)
        }

        self.ioblocks = ioblocks;
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area())
    }

    pub fn handle_event(&mut self, event: Event) {
        if let Event::Key(k) = event {
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

        if let Event::Resize(_, _) = event {
            // mark layout as dirty; redraw
            self.ui_state.layout.is_dirty = true;
        }
    }

    fn handle_toggle_key(&mut self, k: KeyEvent) {
        match k.kind {
            // on state
            KeyEventKind::Press => {
                if k.code == KeyCode::Tab {
                    self.ui_state.peeking_ioblock = true;
                }
            }
            // off state
            KeyEventKind::Release => {
                if k.code == KeyCode::Tab {
                    self.ui_state.peeking_ioblock = false;
                }
            }
            KeyEventKind::Repeat => {}
        }
    }

    fn handle_keypress(&mut self, k: KeyEvent) {
        match k.code {
            KeyCode::Esc => self.running = false,
            KeyCode::Char(' ') => self.paused = !self.paused,
            KeyCode::Up if self.ui_state.ioblock_idx > 0 => {
                self.ui_state.ioblock_idx -= 1;
            }
            KeyCode::Down if self.ui_state.ioblock_idx < self.ioblocks.len() - 1 => {
                self.ui_state.ioblock_idx += 1;
            }
            KeyCode::PageUp => self.ui_state.ioblock_idx = 0,
            KeyCode::PageDown => self.ui_state.ioblock_idx = self.ioblocks.len() - 1,
            KeyCode::Enter => {
                let routine_idx = self.ioblocks[self.ui_state.ioblock_idx];
                // happens if there are no ioblocks
                if routine_idx == usize::MAX {
                    return;
                }
                self.add_running_routine(routine_idx);
            }
            KeyCode::Char('.') if self.paused => {
                self.tick();
            }
            KeyCode::Char('c') => self.ui_state.logbox.clear(),
            KeyCode::Char('r') => {
                self.reset_state();
                self.setup();
            }
            KeyCode::Char('-') => {
                self.hz /= HZ_SCALE;
                self.update_ticks_per_frame();
            }
            KeyCode::Char('=') => {
                self.hz *= HZ_SCALE;
                self.update_ticks_per_frame();
            }
            KeyCode::Char('0') => {
                self.hz = TICKS_PER_SECOND;
                self.update_ticks_per_frame();
            }
            KeyCode::Char('u') => self.ui_state.unlimited_speed = !self.ui_state.unlimited_speed,
            // kc @ (_) => {
            //     self.add_log(format!("Key {kc:?} is not yet supported."));
            // }
            _ => {}
        }
    }

    fn handle_routine(emu_state: &mut EmulatorState, rtn: &mut RunningRoutine, tasm: &Tasm) {
        if rtn.paused {
            return;
        }

        if rtn.waiting > 0 {
            rtn.waiting -= 1;
            return;
        }

        let instrs = &tasm.routines[rtn.index].instructions;
        let wait_time = get_time(&instrs[rtn.instr_ptr]) - 1;

        if rtn.toggled {
            emu_state.exec_instr(
                // this is safe *ONLY IF* the emulator handler never mutates self.tasm
                // it doesn't, so we're good
                unsafe { &*(&instrs[rtn.instr_ptr] as *const Instruction) },
                &tasm.aliases,
                &tasm.routines,
            );
        }

        rtn.waiting = wait_time;

        rtn.instr_ptr += 1;
        if rtn.instr_ptr >= instrs.len() {
            rtn.done = true;
        }
    }

    pub fn tick(&mut self) {
        for routine in self.running_routines.iter_mut() {
            // emulator state was separated to struct due to borrowing issues
            Self::handle_routine(&mut self.state, routine, &self.tasm);
        }

        for cmd in self.state.temp_routine_commands.iter() {
            match cmd {
                RoutineCommand::ToggleOn(g) => self.running_routines.iter_mut().for_each(|r| {
                    if r.group == *g {
                        r.toggled = true
                    }
                }),
                RoutineCommand::ToggleOff(g) => self.running_routines.iter_mut().for_each(|r| {
                    if r.group == *g {
                        r.toggled = false
                    }
                }),
                RoutineCommand::Kill(g) => self.running_routines.iter_mut().for_each(|r| {
                    if r.group == *g {
                        r.done = true
                    }
                }),
                RoutineCommand::Pause(g) => self.running_routines.iter_mut().for_each(|r| {
                    if r.group == *g {
                        r.paused = true
                    }
                }),
                RoutineCommand::Resume(g) => self.running_routines.iter_mut().for_each(|r| {
                    if r.group == *g {
                        r.paused = false
                    }
                }),
                RoutineCommand::Spawn(idx) => self
                    .running_routines
                    .push(RunningRoutine::new(*idx, self.state.routine_groups[*idx])),
            }
        }
        self.state.temp_routine_commands.clear();

        self.running_routines.retain(|r| !r.done);
        // update self fields from self.state
        if self.state.temp_logbox.len() > 0 {
            // take gets the logs and clears them out at the same time
            self.ui_state
                .logbox
                .extend(take(&mut self.state.temp_logbox));
        }
        self.paused = self.state.temp_paused;
        self.state.temp_paused = false;

        self.state.timer_spawns_todo.clear();
        for timer in self.state.ticking_timers.iter() {
            if timer.paused {
                continue;
            }
            let time = self
                .state
                .counter_state
                .timers
                .get_mut(timer.id as usize)
                .unwrap();
            *time += TICK_LENGTH;
            if *time >= timer.target_time {
                self.state.timer_spawns_todo.push((timer.group, timer.id));
            }
        }

        for i in 0..self.state.timer_spawns_todo.len() {
            let (sp, id) = self.state.timer_spawns_todo[i];
            self.state.spawn_group(sp, &self.tasm.routines);
            self.state.ticking_timers.retain(|t| t.id != id);
        }

        if self.ui_state.selected_routine > self.running_routines.len() {
            self.ui_state.selected_routine = self.running_routines.len()
        }

        self.ticks += 1;
    }

    fn add_running_routine(&mut self, routine: usize) {
        self.running_routines.push(RunningRoutine::new(
            routine,
            self.tasm.routines[routine].group,
        ));
    }

    fn get_routine_instr_ref(&self, routine: &RunningRoutine) -> &Instruction {
        &self.tasm.routines[routine.index].instructions[routine.instr_ptr]
    }

    fn get_routine_ref(&self, routine: &RunningRoutine) -> &Routine {
        &self.tasm.routines[routine.index]
    }
}

impl EmulatorState {
    pub fn exec_instr(
        &mut self,
        real_instr: &Instruction,
        aliases: &Aliases,
        routines: &Vec<Routine>,
    ) {
        let (handler_fn, resolved) = {
            let resolved_args = resolve_aliases(real_instr, aliases);
            let resolved_instr = ResolvedInstruction {
                ident: real_instr.ident,
                line_number: real_instr.line_number,
                args: resolved_args,
                _flags: &real_instr.flags[..],
            };

            (real_instr.handler_fn_emu, resolved_instr)
        };

        (handler_fn)(self, resolved, routines);
    }

    pub fn add_running_routine(&mut self, routine: usize) {
        self.temp_routine_commands.push(Spawn(routine));
    }

    pub fn add_log(&mut self, log: String) {
        self.temp_logbox.push(log)
    }
}

pub struct ResolvedInstruction<'a> {
    ident: InstrIdent,
    line_number: usize,
    args: Cow<'a, [TasmValue]>,
    _flags: &'a [Flag], // unused for now
}

#[derive(Debug, Default)]
pub struct RunningRoutine {
    /// Index to Emulator.tasm.routines
    index: usize,
    group: i16,
    instr_ptr: usize,
    waiting: i32, // how many ticks it is waiting
    done: bool,
    paused: bool,
    toggled: bool, // true: on; vv.
}

impl RunningRoutine {
    pub fn new(index: usize, group: i16) -> Self {
        Self {
            index,
            group,
            instr_ptr: 0,
            waiting: 0,
            done: false,
            paused: false,
            toggled: true,
        }
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
    if instr.time == -1 {
        match instr.ident {
            InstrIdent::WAIT => instr.args[0].to_int().unwrap(),
            InstrIdent::WAITS => (instr.args[0].to_float().unwrap() * TICKS_PER_SECOND) as i32,
            _ => unreachable!(),
        }
    } else {
        instr.time
    }
}

impl EmulatorState {
    pub fn write_mem(&mut self, addr: i16, value: f64) {
        match self.mem_info {
            None => (),
            Some(ref mem) => {
                // get true counter
                let true_addr = match mem.is_int() {
                    true => Item::Counter(mem.start_counter_id + addr),
                    false => Item::Timer(mem.start_counter_id + addr),
                };

                self.counter_state.set_item(true_addr, value);
            }
        }
    }

    pub fn read_mem(&self, addr: i16) -> f64 {
        match self.mem_info {
            None => 0.0,
            Some(ref mem) => {
                // get true counter
                let true_addr = match mem.is_int() {
                    true => Item::Counter(mem.start_counter_id + addr),
                    false => Item::Timer(mem.start_counter_id + addr),
                };

                self.counter_state.get_num(true_addr)
            }
        }
    }
}
