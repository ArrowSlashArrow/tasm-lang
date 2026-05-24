use crate::{debugger::Emulator, instr::EmulatorArgs};

impl Emulator {
    pub fn breakpoint(&mut self, _args: EmulatorArgs) {
        self.paused = true;
    }

    pub fn unreachable(&mut self, _args: EmulatorArgs) {
        // ...
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
