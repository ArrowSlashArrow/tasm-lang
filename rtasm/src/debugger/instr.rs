use crate::{debugger::Emulator, instr::EmulatorArgs};

impl Emulator {
    pub fn breakpoint(&mut self, _args: EmulatorArgs) {
        self.paused = true;
    }

    /// This function is used where instructions will *never* be ran in the emulator.
    pub fn unreachable(&mut self, args: EmulatorArgs) {
        // soft unreachable in case something does actually trigger it
        self.add_log(format!(
            "Unreachable function was called! {:?}:{}",
            args.ident,
            args.line_number + 1
        ));
    }

    pub fn not_implemented(&mut self, args: EmulatorArgs) {
        let argtypes = &args.args.iter().map(|a| a.get_type()).collect::<Vec<_>>();

        self.add_log(format!(
            "[WARN] Unimplemented [line {}]: {:?} {argtypes:?}",
            args.line_number + 1,
            args.ident,
        ));
    }
}
