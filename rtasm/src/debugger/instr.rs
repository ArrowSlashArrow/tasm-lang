use gdlib::gdobj::Item;
use paste::paste;
use rand::RngExt;

use crate::{
    core::structs::{Instruction, TasmValue},
    debugger::{Emulator, LegacyMemstate, TickingTimer},
};

// todo: handle flags

impl Emulator {
    /// This function is used where instructions will *never* be ran in the emulator.
    pub fn unreachable(&mut self, args: &Instruction) {
        // soft unreachable in case something does actually trigger it
        self.add_log(format!(
            "Unreachable function was called! {:?}:{}",
            args.ident,
            args.line_number + 1
        ));
    }

    pub fn not_implemented(&mut self, args: &Instruction) {
        let argtypes = &args.args.iter().map(|a| a.get_type()).collect::<Vec<_>>();

        self.add_log(format!(
            "[WARN] Unimplemented [line {}] {:?} {argtypes:?}",
            args.line_number + 1,
            args.ident,
        ));
    }

    pub fn skip(&mut self, args: &Instruction) {
        self.add_log(format!(
            "Skipping external instruction {:?} [line {}].",
            args.ident,
            args.line_number + 1,
        ));
    }

    pub fn silent_skip(&mut self, _args: &Instruction) {
        // ...
    }

    /* Instruction handlers */

    pub fn breakpoint(&mut self, _args: &Instruction) {
        self.paused = true;
    }

    pub fn spawn_group(&mut self, group: i16) {
        if let Some(_) = self.toggled_groups.iter().find(|&g| *g == group) {
            return; // don't spawn if this group is toggled off
        }
        match self.tasm.routines.iter().find(|&rtn| rtn.group == group) {
            Some(routine) => {
                self.add_running_routine(routine.clone());
            }
            None => {
                self.add_log(format!("Spawned external group {group}"));
            }
        }
    }

    pub fn spawn(&mut self, args: &Instruction) {
        let group = args.args[0].to_group_id().unwrap();
        self.spawn_group(group);
    }

    /* NOP, WAIT, WAITS are omitted since their whole point is waiting */

    // do NOT use this on anything but a valid number
    fn to_f64(&self, v: &TasmValue) -> f64 {
        match v {
            TasmValue::Counter(c) => self.state.get_num(Item::Counter(*c)),
            TasmValue::Timer(c) => self.state.get_num(Item::Timer(*c)),
            TasmValue::Number(f) => *f,
            _ => unreachable!(),
        }
    }

    fn arithmetic_2items<F: Fn(f64, f64) -> f64>(&mut self, args: &[TasmValue], op: F) {
        let res = op(self.to_f64(&args[0]), self.to_f64(&args[1]));
        self.state.set_item(args[0].to_item().unwrap(), res);
    }
    fn arithmetic_3items<F: Fn(f64, f64, f64) -> f64>(&mut self, args: &[TasmValue], op: F) {
        let res = op(
            self.to_f64(&args[0]),
            self.to_f64(&args[1]),
            self.to_f64(&args[2]),
        );
        self.state.set_item(args[0].to_item().unwrap(), res);
    }
    fn arithmetic_4items<F: Fn(f64, f64, f64, f64) -> f64>(&mut self, args: &[TasmValue], op: F) {
        let res = op(
            self.to_f64(&args[0]),
            self.to_f64(&args[1]),
            self.to_f64(&args[2]),
            self.to_f64(&args[3]),
        );
        self.state.set_item(args[0].to_item().unwrap(), res);
    }

    fn compare_spawn<F: Fn(f64, f64) -> bool>(&mut self, args: &[TasmValue], spawn_cond: F) {
        if spawn_cond(self.to_f64(&args[1]), self.to_f64(&args[2])) {
            self.spawn_group(args[0].to_group_id().unwrap());
        }
    }
    fn compare_fork<F: Fn(f64, f64) -> bool>(&mut self, args: &[TasmValue], spawn_cond: F) {
        if spawn_cond(self.to_f64(&args[2]), self.to_f64(&args[3])) {
            self.spawn_group(args[0].to_group_id().unwrap());
        } else {
            self.spawn_group(args[1].to_group_id().unwrap());
        }
    }

    pub fn srand(&mut self, args: &Instruction) {
        if rand::rng().random_range(0.0..100.0) < args.args[1].to_float().unwrap() {
            self.spawn_group(args.args[0].to_group_id().unwrap());
        }
    }
    pub fn frand(&mut self, args: &Instruction) {
        if rand::rng().random_range(0.0..100.0) < args.args[2].to_float().unwrap() {
            self.spawn_group(args.args[0].to_group_id().unwrap());
        } else {
            self.spawn_group(args.args[1].to_group_id().unwrap());
        }
    }

    pub fn pause(&mut self, args: &Instruction) {
        if let Some(rtn) = self
            .running_routines
            .get_mut(args.args[0].to_group_id().unwrap() as usize)
        {
            rtn.paused = true;
        }
    }
    pub fn kill(&mut self, args: &Instruction) {
        if let Some(rtn) = self
            .running_routines
            .get_mut(args.args[0].to_group_id().unwrap() as usize)
        {
            rtn.done = true;
        }
    }
    pub fn resume(&mut self, args: &Instruction) {
        if let Some(rtn) = self
            .running_routines
            .get_mut(args.args[0].to_group_id().unwrap() as usize)
        {
            rtn.paused = false;
        }
    }

    pub fn tspawn(&mut self, args: &Instruction) {
        let timer = args.args[0].to_timer_id().unwrap();
        let start_time = args.args[1].to_float().unwrap();
        let end_time = args.args[2].to_float().unwrap();
        let spawn_group = args.args[3].to_group_id().unwrap();
        if !self.started_timers.contains(&timer) {
            self.started_timers.push(timer);
        }

        // reset time
        self.state.timers[timer as usize] = start_time as f32;

        let timer_obj = TickingTimer {
            id: timer,
            group: spawn_group,
            target_time: end_time as f32,
            paused: false,
        };
        if let Some(t) = self.ticking_timers.iter_mut().find(|t| t.id == timer) {
            // override an active timer if it exists
            // i couldn't figure out whether gd allows multiple timers that have the same id
            // to be ticking at once
            // i'm not gonna allow it because i don't wanna implement that
            *t = timer_obj;
        } else {
            // starting a fresh one
            self.ticking_timers.push(timer_obj);
        }
    }

    pub fn tstart(&mut self, args: &Instruction) {
        let timer = args.args[0].to_timer_id().unwrap();
        if !self.started_timers.contains(&timer) {
            return;
        }

        if let Some(t) = self.ticking_timers.iter_mut().find(|t| t.id == timer) {
            t.paused = false;
        } else {
            // starting a fresh one
            self.ticking_timers.push(TickingTimer {
                id: timer,
                group: 0, // this one has no volition other than to tick
                target_time: f32::MAX,
                paused: false,
            });
        }
    }

    pub fn tstop(&mut self, args: &Instruction) {
        let timer = args.args[0].to_timer_id().unwrap();
        // timers are deleted when they are expired
        // i.e. not here
        if let Some(t) = self.ticking_timers.iter_mut().find(|t| t.id == timer) {
            t.paused = true;
        }
    }

    pub fn toggleon(&mut self, args: &Instruction) {
        let group = args.args[0].to_group_id().unwrap();
        match self.tasm.routines.iter().find(|&rtn| rtn.group == group) {
            Some(_) => {
                // toggleable routine
                self.toggled_groups.retain(|g| *g != group);

                // then, update all running routines
                for rtn in self.running_routines.iter_mut() {
                    if rtn.routine.group == group {
                        rtn.toggled = true;
                    }
                }
            }
            None => self.add_log(format!("Toggled on external group {group}")),
        }
    }

    pub fn toggleoff(&mut self, args: &Instruction) {
        let group = args.args[0].to_group_id().unwrap();
        // todo: optimize the group search to be a flat array (cache friendly)
        // this is too much since were only checking if the group exists
        // same for toggleon
        match self.tasm.routines.iter().find(|&rtn| rtn.group == group) {
            Some(_) => {
                self.toggled_groups.push(group);

                // then, update all running routines
                for rtn in self.running_routines.iter_mut() {
                    if rtn.routine.group == group {
                        rtn.toggled = false;
                    }
                }
            }
            None => self.add_log(format!("Toggled off external group {group}")),
        }
    }

    pub fn lmreset(&mut self, _args: &Instruction) {
        let mem = self.tasm.mem_info.as_ref().unwrap();
        let ctr_id = mem.ptrpos.to_counter_id().unwrap();
        self.state.set_item(Item::Counter(ctr_id), 0.0);
    }

    pub fn lmptr(&mut self, args: &Instruction) {
        let move_amount = args.args[0].to_int().unwrap();
        let mem = self.tasm.mem_info.as_ref().unwrap();
        let ctr_id = mem.ptrpos.to_counter_id().unwrap();
        self.state
            .set_item(Item::Counter(ctr_id), move_amount as f64);
    }

    pub fn lmread(&mut self, _args: &Instruction) {
        self.legacy_memstate = LegacyMemstate::Read;
    }
    pub fn lmwrite(&mut self, _args: &Instruction) {
        self.legacy_memstate = LegacyMemstate::Write;
    }

    /// Returns addr and if it is in valid range
    fn get_ptrpos_value(&self) -> (i32, bool) {
        let mem = self.tasm.mem_info.as_ref().unwrap();
        let addr = self.state.get_num(mem.ptrpos.to_item().unwrap()) as i32;
        if addr < 0 || addr >= mem.size as i32 {
            (addr, false)
        } else {
            (addr, true)
        }
    }

    fn get_memreg(&self) -> Item {
        self.tasm
            .mem_info
            .as_ref()
            .unwrap()
            .memreg
            .to_item()
            .unwrap()
    }

    pub fn lmfunc(&mut self, _args: &Instruction) {
        match self.legacy_memstate {
            LegacyMemstate::None => {
                // skip mem io, since the operation has not been initialised
                self.add_log(
                    "[WARN] Memory mode uninitialised! Memory operation skipped due to possible UB.".into(),
                );
            }
            LegacyMemstate::Read => {
                let (addr, valid) = self.get_ptrpos_value();

                if !valid {
                    self.add_log(format!("[WARN] Cannot read address {addr} (out of range)"));
                    return;
                }

                let variable = self.read_mem(addr as i16);
                self.state.set_item(self.get_memreg(), variable);
            }
            LegacyMemstate::Write => {
                let (addr, valid) = self.get_ptrpos_value();

                if !valid {
                    self.add_log(format!(
                        "[WARN] Cannot write to address {addr} (out of range)"
                    ));
                    return;
                }

                self.write_mem(addr as i16, self.state.get_num(self.get_memreg()));
            }
        }
    }
}

macro_rules! op_fn {
    ($new_ident:ident, $underlying:ident, $closure:expr) => {
        paste! {
            impl Emulator {
                pub fn [<$underlying _ $new_ident>](&mut self, args: &Instruction) {
                    self.$underlying(&args.args[..], $closure)
                }
            }
        }
    };
}

/* the great big macro wall */

op_fn!(mov, arithmetic_2items, |_, b| b);
op_fn!(add, arithmetic_2items, |a, b| a + b);
op_fn!(sub, arithmetic_2items, |a, b| a - b);
op_fn!(mul, arithmetic_2items, |a, b| a * b);
op_fn!(div, arithmetic_2items, |a, b| a / b);
op_fn!(fldiv, arithmetic_2items, |a, b| (a / b).floor());

op_fn!(add, arithmetic_3items, |_, a, b| a + b);
op_fn!(sub, arithmetic_3items, |_, a, b| a - b);
op_fn!(mul, arithmetic_3items, |_, a, b| a * b);
op_fn!(div, arithmetic_3items, |_, a, b| a / b);
op_fn!(fldiv, arithmetic_3items, |_, a, b| (a / b).floor());

op_fn!(addm, arithmetic_3items, |a, b, c| (a + b) * c);
op_fn!(addd, arithmetic_3items, |a, b, c| (a + b) / c);
op_fn!(subm, arithmetic_3items, |a, b, c| (a - b) * c);
op_fn!(subd, arithmetic_3items, |a, b, c| (a - b) / c);
op_fn!(addm, arithmetic_4items, |_, a, b, c| (a + b) * c);
op_fn!(addd, arithmetic_4items, |_, a, b, c| (a + b) / c);
op_fn!(subm, arithmetic_4items, |_, a, b, c| (a - b) * c);
op_fn!(subd, arithmetic_4items, |_, a, b, c| (a - b) / c);

op_fn!(eq, compare_spawn, |a, b| a == b);
op_fn!(ne, compare_spawn, |a, b| a != b);
op_fn!(gt, compare_spawn, |a, b| a > b);
op_fn!(ge, compare_spawn, |a, b| a >= b);
op_fn!(lt, compare_spawn, |a, b| a < b);
op_fn!(le, compare_spawn, |a, b| a <= b);
op_fn!(eq, compare_fork, |a, b| a == b);
op_fn!(ne, compare_fork, |a, b| a != b);
op_fn!(gt, compare_fork, |a, b| a > b);
op_fn!(ge, compare_fork, |a, b| a >= b);
op_fn!(lt, compare_fork, |a, b| a < b);
op_fn!(le, compare_fork, |a, b| a <= b);
