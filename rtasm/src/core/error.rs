use std::{error::Error, fmt::Display};

/// Representative of TASM high-level lexer, parser, and logic errors.
/// 
/// - `type`: the type of error. Refer to `TasmErrorType` for more info.
/// - `file`: the file in which the error occurred. This is typically the source file being compiled.
///     - In the future, this could also include modules and imported files.
/// - `routine`: the routine in which the error occurred. This is typically the current routine being compiled.
/// - `line`: the line number in which the error occurred. This is typically the line number in the source file being compiled. 0 if the error does not use a line (like `ExceedsGroupLimit`).
/// - `details`: a detailed message about the error. This is typically a human-readable message that provides more information about the error.
#[derive(Debug, Clone)]
pub struct TasmError {
    pub r#type: TasmErrorType,
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
                self.r#type,
                self.details
            )
        } else {
            write!(f, "{} [{:?}] {}", self.file, self.r#type, self.details)
        }
    }
}

/// Low-level temporary error type used for internal handling.
pub(crate) enum ParseErrorType {
    BadID,
    TrailingComma,
    InvalidNumber,
}