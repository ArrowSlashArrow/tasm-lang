use gdlib::gdobj::Item;
use paste::paste;
use rand::RngExt;

use crate::{
    core::structs::{Instruction, TasmValue},
    debugger::Emulator,
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

    /* Instruction handlers */

    pub fn breakpoint(&mut self, _args: &Instruction) {
        self.paused = true;
    }

    fn spawn_group(&mut self, group: i16) {
        match self.tasm.routines.iter().find(|&rtn| rtn.group == group) {
            Some(routine) => {
                self.add_running_routine(routine.clone());
            }
            None => {
                self.add_log(format!("Spawned external group {group:?}"));
            }
        }
    }

    fn wait_ticks(&mut self, rtn_idx: usize, ticks: i32) {
        if let Some(rtn) = self.running_routines.get_mut(rtn_idx) {
            rtn.waiting = ticks;
        }
    }

    pub fn spawn(&mut self, args: &Instruction) {
        let group = args.args[0].to_group_id().unwrap();
        self.spawn_group(group);
    }
    pub fn nop(&mut self, args: &Instruction) {
        self.wait_ticks(args.parent_running_routine_idx, 1);
    }

    pub fn wait(&mut self, args: &Instruction) {
        self.wait_ticks(
            args.parent_running_routine_idx,
            args.args[0].to_int().unwrap(),
        );
    }

    pub fn waits(&mut self, args: &Instruction) {
        self.wait_ticks(
            args.parent_running_routine_idx,
            (args.args[0].to_float().unwrap() * 240.0) as i32,
        );
    }

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

    fn compare_spawn<F: Fn(f64, f64) -> bool>(
        &mut self,
        args: &[TasmValue],
        spawn_cond: F,
        parent: usize,
    ) {
        if spawn_cond(self.to_f64(&args[1]), self.to_f64(&args[2])) {
            self.spawn_group(args[0].to_group_id().unwrap());
        }
        self.wait_ticks(parent, 2);
    }
    fn compare_fork<F: Fn(f64, f64) -> bool>(
        &mut self,
        args: &[TasmValue],
        spawn_cond: F,
        parent: usize,
    ) {
        if spawn_cond(self.to_f64(&args[2]), self.to_f64(&args[3])) {
            self.spawn_group(args[0].to_group_id().unwrap());
        } else {
            self.spawn_group(args[1].to_group_id().unwrap());
        }
        self.wait_ticks(parent, 2);
    }

    pub fn srand(&mut self, args: &Instruction) {
        if rand::rng().random_range(0.0..100.0) < args.args[1].to_float().unwrap() {
            self.spawn_group(args.args[0].to_group_id().unwrap());
        }
        self.wait_ticks(args.parent_running_routine_idx, 2);
    }
    pub fn frand(&mut self, args: &Instruction) {
        if rand::rng().random_range(0.0..100.0) < args.args[2].to_float().unwrap() {
            self.spawn_group(args.args[0].to_group_id().unwrap());
        } else {
            self.spawn_group(args.args[1].to_group_id().unwrap());
        }
        self.wait_ticks(args.parent_running_routine_idx, 2);
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

    ($new_ident:ident => $underlying:ident, $closure:expr) => {
        paste! {
            impl Emulator {
                pub fn [<$underlying _ $new_ident>](&mut self, args: &Instruction) {
                    self.$underlying(&args.args[..], $closure, args.parent_running_routine_idx as usize)
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

op_fn!(eq => compare_spawn, |a, b| a == b);
op_fn!(ne => compare_spawn, |a, b| a != b);
op_fn!(gt => compare_spawn, |a, b| a > b);
op_fn!(ge => compare_spawn, |a, b| a >= b);
op_fn!(lt => compare_spawn, |a, b| a < b);
op_fn!(le => compare_spawn, |a, b| a <= b);
op_fn!(eq => compare_fork, |a, b| a == b);
op_fn!(ne => compare_fork, |a, b| a != b);
op_fn!(gt => compare_fork, |a, b| a > b);
op_fn!(ge => compare_fork, |a, b| a >= b);
op_fn!(lt => compare_fork, |a, b| a < b);
op_fn!(le => compare_fork, |a, b| a <= b);
