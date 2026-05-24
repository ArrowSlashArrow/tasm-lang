use crate::{core::structs::Instruction, debugger::Emulator};

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

    pub fn spawn(&mut self, args: &Instruction) {
        let group = args.args[0].to_group_id().unwrap();
        match self.tasm.routines.iter().find(|&rtn| rtn.group == group) {
            Some(routine) => {
                self.add_running_routine(routine.clone());
            }
            None => {
                self.add_log(format!("Spawned external group {group:?}"));
            }
        }
    }

    pub fn mov_item_num(&mut self, args: &Instruction) {
        let dest = args.args[0].to_item().unwrap();
        let num = args.args[1].to_float().unwrap();
        self.state.set_item(dest, num);
    }

    pub fn mov_item_item(&mut self, args: &Instruction) {
        let dest = args.args[0].to_item().unwrap();
        let src = args.args[1].to_item().unwrap();
        self.state.set_item(dest, self.state.get_num(src));
    }
}
