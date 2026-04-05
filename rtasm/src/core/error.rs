use std::{error::Error, fmt::Display, num::ParseIntError};

use gdlib::gdobj::ItemType;

use crate::core::{
    consts::{ENTRY_POINT, GROUP_LIMIT},
    structs::TasmValue,
};

#[derive(Debug, Clone)]
pub enum TasmParseError {
    InvalidInstruction((String, usize)),
    InvalidArguments((String, usize)),
    InvalidAssignment((usize, ItemType)),
    InvalidWaitAmount((usize, i32)),
    BadID((String, usize)),
    BadToken((String, usize)),
    BadAlias((String, usize)),
    BadFlag((String, usize)),
    NoEntryPoint,
    InvalidNumber((String, String, usize)),
    InvalidGroup(ParseIntError),
    ExceedsGroupLimit,
    InitRoutineSpawnError(usize),
    MultipleMemoryInstances(usize),
    MultipleAliasDefinitions((usize, String, TasmValue)),
    MultipleRoutineDefintions(String, usize, usize),
    NonInitAliasDefinition(usize),
    InvalidPointerMove(String, usize),
    InitRoutineMemoryAccess(usize),
    NonexistentMemoryAccess(usize),
    TrailingComma(usize),
}

#[derive(Debug)]
pub enum ParseErrorType {
    BadID,
    TrailingComma,
    InvalidNumber,
}

impl Error for TasmParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

impl Display for TasmParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInstruction((cmd, line)) => {
                write!(f, "Bad command: {cmd} on line {}", line + 1)
            }
            Self::BadFlag((cmd, line)) => {
                write!(f, "Bad flag: {cmd} on line {}", line + 1)
            }
            Self::BadAlias((cmd, line)) => {
                write!(f, "Bad alias: {cmd} on line {}", line + 1)
            }
            Self::NoEntryPoint => write!(f, "No entry point found. ({ENTRY_POINT} routine)"),
            Self::InvalidArguments((reason, line)) => {
                write!(f, "Invalid arguments on line {}: {reason}", line + 1)
            }
            Self::InvalidAssignment((line, _type)) => {
                write!(f, "Cannot assign to {_type:?} on line {}", line + 1)
            }
            Self::InvalidWaitAmount((line, wait)) => {
                write!(
                    f,
                    "Cannot wait a negative amount of ticks ({wait}) on line {line}."
                )
            }
            Self::BadID((msg, line)) => {
                write!(f, "Bad ID on line {}: {msg}.", line + 1)
            }
            Self::BadToken((tok, line)) => {
                write!(
                    f,
                    "Bad token on line {}: {tok}. If this is an instruction, it must be indented.",
                    line + 1
                )
            }
            Self::InitRoutineSpawnError(line) => {
                write!(
                    f,
                    "Spawning the initialiser routine is not allowed (line {}).",
                    line + 1
                )
            }
            Self::InvalidNumber((why, num, line)) => {
                write!(f, "Invalid number {num} on line {}. {why}", line + 1)
            }
            Self::InvalidGroup(why) => {
                write!(f, "Invalid group. {why}")
            }
            Self::ExceedsGroupLimit => {
                write!(f, "Input file exceeds group limit of {GROUP_LIMIT} groups.")
            }
            Self::MultipleMemoryInstances(line) => {
                write!(f, "Multiple memory instances are not allowed: line {line}")
            }
            Self::MultipleAliasDefinitions((line, alias, value)) => {
                write!(
                    f,
                    "Line {line}: Alias {alias} cannot be reassigned a value, since it already corresponds to {value:?}"
                )
            }
            Self::NonInitAliasDefinition(line) => {
                write!(
                    f,
                    "Cannot define an alias on line {line} since it is not in the _init routine."
                )
            }
            Self::InvalidPointerMove(reason, line) => {
                write!(f, "{reason} at line {line}.")
            }
            Self::MultipleRoutineDefintions(rtn, line, prev_line) => {
                write!(
                    f,
                    "Routine {rtn}, on line {line}, has already been declared on line {prev_line}."
                )
            }
            Self::InitRoutineMemoryAccess(line) => {
                write!(
                    f,
                    "Memory access attempt on line {line} is forbidden, due to being in the initializer routine."
                )
            }
            Self::NonexistentMemoryAccess(line) => {
                write!(
                    f,
                    "Attempted to access memory while none exists on line {line}."
                )
            }
            Self::TrailingComma(line) => {
                write!(f, "Trailing comma found at line {}.", line + 1)
            }
        }
    }
}
