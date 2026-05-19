use std::{time::Duration, vec};

use crate::core::{
    consts::INIT_ROUTINE,
    structs::{InstrIdent, InstrType, Instruction, Routine, Tasm},
};

use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use gdlib::gdobj::Item;
use ratatui::{
    self, Frame,
    layout::{Constraint, Layout},
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget},
};

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
            self.tick();
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

impl Widget for &Emulator {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        // outline
        Block::bordered()
            .border_set(border::ROUNDED)
            .render(area, buf);

        let middle_h_temp = Layout::horizontal(vec![
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .split(area);

        let workable_area = Layout::vertical(vec![
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(middle_h_temp[1])[1];

        /* setup areas */
        let h_layout = Layout::horizontal(vec![Constraint::Percentage(50), Constraint::Min(1)])
            .split(workable_area);

        let vleft_layout =
            Layout::vertical(vec![Constraint::Min(1), Constraint::Min(1)]).split(h_layout[0]);

        let htopleft_layout = Layout::horizontal(vec![Constraint::Min(1), Constraint::Length(32)])
            .split(vleft_layout[0]);

        let vbottomleft_layout = Layout::vertical(vec![Constraint::Length(5), Constraint::Min(1)])
            .split(vleft_layout[1]);

        let vright_layout =
            Layout::vertical(vec![Constraint::Min(1), Constraint::Length(10)]).split(h_layout[1]);

        let logbox_area = htopleft_layout[0];
        let display_area = htopleft_layout[1];
        let info_area = vbottomleft_layout[0];
        let routines_area = vbottomleft_layout[1];
        let memory_area = vright_layout[0];
        let keys_area = vright_layout[1];

        /* logbox */

        let logbox_height = logbox_area.height as usize;
        let logs = if self.logbox.len() > logbox_height {
            &self.logbox[(&self.logbox.len() - logbox_height)..]
        } else {
            &self.logbox[..]
        };

        Paragraph::new(Text::from(
            logs.iter()
                .map(|log| Line::from(format!(" {log} ")))
                .collect::<Vec<Line<'_>>>(),
        ))
        .block(
            Block::bordered()
                .border_set(border::DOUBLE)
                .title(" Emulator logs ".yellow().into_centered_line()),
        )
        .render(logbox_area, buf);

        /* Displays */

        let displays_height = display_area.height as usize;
        let displays = if self.displays.len() > displays_height {
            &self.displays[..displays_height]
        } else {
            &self.displays[..]
        };

        Paragraph::new(Text::from(
            displays
                .iter()
                .map(|item| {
                    Line::from(format!(
                        " {:<13} : {:>12} ",
                        format!("{item:?}"),
                        self.state.get_item_value(*item)
                    ))
                })
                .collect::<Vec<Line<'_>>>(),
        ))
        .block(
            Block::bordered()
                .border_set(border::DOUBLE)
                .title(" Displayed items ".green().into_centered_line()),
        )
        .render(display_area, buf);
    }
}
