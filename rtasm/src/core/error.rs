use std::{error::Error, fmt::Display};

#[derive(Debug, Clone)]
pub struct TasmError {
    pub _type: TasmErrorType,
    pub file: String,
    pub routine: String, // routine (helps with navigation)
    pub error: bool,     // warning: false
    pub line: usize,     // 0 if doesnt use a line (like ExceedsGroupLimit)
    pub details: String, // details msg
}

#[derive(Debug, Clone, Copy)]
pub enum TasmErrorType {
    InvalidInstruction,
    InvalidArguments,
    InvalidAssignment,
    InvalidWaitAmount,
    InvalidMemoryRange,
    BadID,
    BadToken,
    BadAlias,
    BadFlag,
    NoEntryPoint,
    InvalidNumber,
    InvalidGroup,
    ExceedsGroupLimit,
    InitRoutineSpawnError,
    MultipleMemoryInstances,
    MultipleAliasDefinitions,
    MultipleRoutineDefintions,
    NonInitAliasDefinition,
    InvalidPointerMove,
    InitRoutineMemoryAccess,
    NonexistentMemoryAccess,
    TrailingComma,
}

#[derive(Debug)]
pub enum ParseErrorType {
    BadID,
    TrailingComma,
    InvalidNumber,
}

impl Error for TasmError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

impl Display for TasmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.line != 0 {
            write!(
                f,
                "{} @ {}:{} [{:?}] {}",
                self.file,
                self.routine,
                // line + 1 to match the visual index, e.g. line 0 appears as line 1 in most editors
                self.line + 1,
                self._type,
                self.details
            )
        } else {
            write!(f, "{} [{:?}] {}", self.file, self._type, self.details)
        }
    }
}
