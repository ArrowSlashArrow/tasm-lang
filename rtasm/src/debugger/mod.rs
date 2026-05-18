use std::time::Duration;

use crate::core::{
    consts::INIT_ROUTINE,
    structs::{InstrType, Instruction, Routine, Tasm},
};

use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    self, Frame,
    text::Text,
    widgets::{Paragraph, Widget},
};

pub fn emulate(tasm: Tasm) {
    println!("{tasm:#?}");
    // setup state
    if let Err(e) = Emulator::new(tasm).run() {
        println!("Emulator failed to run: {e}")
    };

    ratatui::restore();
}

struct EmulatorState {
    counters: [i32; 9999],
    timers: [f32; 9999],
    attempts: i32,
    points: f32,
    maintime: f32,
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
}

struct Emulator {
    state: EmulatorState,
    tasm: Tasm,
    running: bool,
    running_routines: Vec<RunningRoutine>,
    ioblocks: Vec<usize>, // idxs to self.tasm.routines
    init_instrs: Vec<Instruction>,
    ticks: u32,
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
        let init_instrs =
            if let Some(rtn) = tasm.routines.iter().find(|rtn| rtn.ident == INIT_ROUTINE) {
                rtn.instructions
                    .iter()
                    .filter(|&instr| {
                        instr.itype != InstrType::Init
                            || instr.ident == "INITMEM"
                            || instr.ident == "DISPLAY"
                    })
                    .map(|i| i.clone())
                    .collect()
            } else {
                vec![]
            };

        Self {
            state: EmulatorState::new(),
            tasm,
            running: true,
            running_routines: vec![],
            ioblocks: vec![],
            init_instrs,
            ticks: 0,
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
        // for stuff like indexing ioblocks, getting memsize, running _init

        let init_idx;

        if let Some((idx, _)) = self
            .tasm
            .routines
            .iter()
            .enumerate()
            .find(|(_, rtn)| rtn.ident == INIT_ROUTINE)
        {
            init_idx = idx;
        } else {
            return;
        }

        self.load_ioblocks();

        // all init instructions are encoded in some other way (e.g. MEMINFO)
        // other than INITMEM, DISPLAY

        // todo: figure out how to run instructions
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
                if instr.ident.as_str() != "IOBLOCK" {
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

    fn exec_instr(&mut self, instr: Instruction) {
        todo!()
    }
}

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
        let temp_pg = Paragraph::new(Text::from("press esc to leave"));

        temp_pg.render(area, buf);
    }
}
