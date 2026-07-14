use core::{error::Error, fmt::Display};

pub const ERROR_DOCS: &[&str] = &[
    "", // there is no error "0"
    "E0001: Program uses more groups than are given. 

    GD gives us a certain amount of groups in total to work with for the entire level. 
If the program exceeds this limit due to too many routines or instructions with overhead, 
this error will be tripped.

As of GD 2.208, this limit is 9,999 groups.",
    "E0002: Cannot access memory in the _init routine.
Note: This error is emitted only when working with legacy memory instructions.

Memory initialization happens in the _init routine, which means it is not safe to use it 
after the _init routine completes. The memory instructions themselves - LMALLOC and 
LFMALLOC - signal to the compiler to insert a specific structure into the level, and they 
are NOT repeatable actions with a definitive end like an arithmetic or comparison operation.",
    "E0003: Cannot access memory when none exists.
Note: This error is emitted only when working with legacy memory instructions.

This error is emitted when there are memory instructions used when there is also not an 
allocator instruction (LMALLOC or LFMALLOC). To use memory, some must first be allocated 
with either of those instructions. ",
"E0004: Cannot overwrite value of counter.

This error is emitted when there is an instruction in the program that tries to write to 
an unwritable counter. There are two items in GD which you cannot write to: the MainTime 
timer and the Attempts counter. It is not possible to write to these because that option 
is not given in the ItemEdit trigger. These two items are used by the game to keep track 
of consistent in-level variables.",
    "E0005: Memory was already created on a previous line.
Note: This error is emitted only when working with legacy memory instructions.

Memory can only be allocated once per program. This error is emitted if two or more memory 
instructions are used in the _init routine of a program.",
"E0006: Cannot wait a negative number of ticks.

This error is emitted if the argument is negative for any of the wait instructions, excluding NOP.",
    "E0007: Cannot initialise memory when none exists.
Note: This error is emitted only when working with legacy memory instructions.

This error is emitted if the INITMEM instruction is used in a program where no memory 
is allocated, since it is not possible to initialise memory with some values if there is 
no memory to begin with.",
    "E0008: Pointer moved more spaces than memory size.
Note: This error is emitted only when working with legacy memory instructions.

This error is emitted to prevent pointer address overflows when using LMPTR. Because it 
is not possible to set the address of the pointer dynamically in legacy memory, this 
instruction is needed. To try to prevent bugs with incorrect addresses, this fail-safe 
was introduced.",
    "E0009: Pointer moved while no memory exists.
Note: This error is emitted only when working with legacy memory instructions.
    
This error is emitted when the MPTR instruction is used without any memory present. 
If there is no memory, there is no pointer to move.",
    "E0010: Cannot allocate memory from A to B.
Note: This error is emitted only when working with legacy memory instructions.

This error is emitted when the given memory size is negative.",
    "E0011: No entry point found in file.

This error is emitted when the compiler can't find the _start routine. If this routine 
was intentionally omitted, this error can be silenced with --no-entry-point.",
    "E0012: Instruction <instruction> must only have X arguments: <arguments>.
    
This error is emitted if the number of arguments for an instruction are incorrect. 
This specific error only exists for IMPORT and ALIAS due to the way they are handled.",
    "E0013: Cannot override existing alias.
    
This error is emitted if there are two or more ALIAS instructions that try to define a value
for a single alias. Since aliases are immutable, they cannot have more than one value.",
    "E0014: Cannot override default alias.
    
This error is emitted if there is an ALIAS instruction that tries to define a value 
for a built-in alias - MEMREG, PTRPOS, MEMSIZE, POINTS, ATTEMPTS, or MAINTIME.",
    "E0015: Bad alias identifier.
    
This error is emitted if an alias is given an identifier that was parsed as anything 
other than a string. This prevents aliases from being named as groups or routine identifiers.",
    "E0016: Got a 0-length string. Perhaps there is a trailing comma.
    
This error is emitted when the compiler parses a value to an empty string: \"\". 
Usually this is unintentional and happens when there is a trailing comma at the end 
of the line, causing the compiler to parse an extra string argument, which may trip E0027.",
    "E0017: Item/group must be within the range [1, 9999].
    
This error is emitted if a counter/timer has an invalid ID.",
    "E0018: Infinity is not allowed.
    
This error is emitted if a number is parsed as either positive or negative infinity.",
    "E0019: NaN is not allowed.
    
This error is emitted if a number is parsed as NaN.",
    "E0020: Could not parse hexadecimal number.
    
This error is emitted if the compiler tries to parse an invalid hexadecimal value.",
    "E0021: Bad flag arguments.

This error is emitted if there is more than one pipe | in an instruction line. This character
is used to denote that flag arguments follow it on this line. In TASM, only one pipe is used
to denote this.",
    "E0022: Trailing commas are not allowed.
    
Usually this is unintentional and happens when there is a trailing comma at the end 
of the line, causing the compiler to parse an extra string argument, which may trip E0027.
This is caught by this error to prevent such mistakes.",
    "E0023: Cannot define an alias outside of the init routine.

The ALIAS instruction is an instruction for the compiler and less so for the program.
It is used to name constant magic values once, hence it is init-exclusive.",
    "E0024: Failed to parse instruction: invalid argset.
    
This error is emitted when an instruction argument has an invalid value. See E0016-E0020.",
    "E0025: Unrecognized instruction.
    
This error is emitted if the compiler encounters and unknown instruction identifier. This may
happen when using an old compiler for newer TASM.",
    "E0026: Instruction is not allowed in routine because it is exclusive to the initialiser routine.
    
This error is emitted when an init-exclusive instruction is used in any routine other than the _init routine.",
    "E0027: Instruction has no argument handler for the argset.
    
This error is emitted when a set of arguments is parsed successfully but no handler for this specific 
set of arguments can be found for the instruction.",
    "E0028: Routine was already declared on a previous line.
    
This error is emitted if two or more routines with the same name are defined in a program.",
    "E0029: Bad token.
    
This error is emitted when the compiler finds an unindented string that is also not a routine 
identifier or comment. A valid routine identifier must contain valid ASCII but no spaces.",
    "E0030: Bad flag.
    
This error is emitted if a flag does not have a value - specifically, a colon to split the 
flag's identifier and its value.",
    "E0031: Unrecognized flag.
    
This error is emitted if a flag is used that is not one of the supported TASM flags.
E0025 for flags.",
    "E0032: Unable to parse flag with value and type.
    
This error is emitted if a flag has a bad value",
    "E0033: Cannot spawn _init routine.
    
This error is emitted if an instruction may spawn the _init routine.
Due to the way the _init routine works - that being that it is for setup instructions which 
are not necessarily replicatable (for example, defining an alias) - it is not meant to be
processed more than once at the beginning, and is therefore prevented from being spawned.",
    "E0034: Cannot import a module outside of the init routine.
    
Module imports are a setup action only allowed in the _init routine.",
    "E0035: Unable to find module::symbol
    
This errors is emitted when an external symbol is referenced with mod::symbol syntax but
the linker can't find the routine in the imported module. This indicates that either
the symbol does not exist or its name was mistyped.",
    "E0036: Unable to find module::routine
    
This error is emitted when a routine is parsed and the linker tries to find all the other 
routines it references, except it can't one of those routines. This error happens only
when the linker tries to resolve an imported symbol. Similar to E0035.",
    "E0037: Unable to find module::routine
    
This error is emitted when a routine is parsed and the linker tries to find all the other 
routines it references, except it can't one of those routines. This error happens only
when the linker tries to resolve a symbol in the same file as the routine it is probing. 
Similar to E0035.",
    "E0038: Unable to find module.
    
This error is emitted when a module is imported via the IMPORT command but the linker
can't find the module. Arguments to the IMPORT command must be paths relative to the
module that is importing the dependency.",
    "E0039: Unable to find dependency.
    
This error is emitted during post-link processing when the linker tries to resolve an
unparsed dictionary flag entry that is an external symbol, however it cannot find the
module to which the symbol points.",
    "E0040: Could not find external symbol.
    
This error is emitted during post-link processing when the linker tries to resolve an
unparsed dictionary flag entry that is an external symbol, however it cannot find the
routine/alias to which the symbol points.",
    "E0041: Invalid external alias dict value.
    
This error is emitted during post-link processing when the linker tries to resolve an
unparsed dictionary flag entry as an integer, however it cannot due to the value not 
being a value integer.",
    "E0042: Instruction was not properly linked!
    
During the linking phase, external symbols which reference groups are attempted to be
assigned a group (due to being resolved). If this fails, the assigned group will remain
as the sentinel value given in the lexing stage, which does not correspond to a real group.

This error should never occur normally. It will only be emitted if there is an instruction
whose arguments were not properly resolved."
];

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
    pub etype: TasmErrorType,
    pub file: String,
    pub routine: String, // routine (helps with navigation)
    pub line: usize,     // 0 if doesnt use a line (like ExceedsGroupLimit)
    pub details: String, // details msg
    // this is necessary to differentiate between different kinds of the same error type
    pub errcode: i32, // unique code for a specific error
}

#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
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
    BadHexLiteral,
    NoEntryPoint,
    InvalidNumber,
    InvalidGroup,
    ExceedsGroupLimit,
    InitRoutineSpawnError,
    MultipleMemoryInstances,
    MultipleAliasDefinitions,
    MultipleRoutineDefintions,
    NonInitAliasDefinition,
    NonInitImport,
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
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.line != 0 {
            write!(
                f,
                "{} @ {}:{} [E{:0>4}] {:?}: {}",
                self.file,
                self.routine,
                // line + 1 to match the visual index, e.g. line 0 appears as line 1 in most editors
                self.line + 1,
                self.errcode,
                self.etype,
                self.details
            )
        } else {
            write!(
                f,
                "{} [E{:0>4}] {:?}: {}",
                self.file, self.errcode, self.etype, self.details
            )
        }
    }
}

/// Low-level temporary error type used for internal handling.
#[derive(Debug)]
pub(crate) enum ParseErrorType {
    BadID,
    TrailingComma,
    InvalidNumber,
    BadHexLiteral,
}
