use crate::{
    error::{TasmError, TasmErrorType},
    structs::{HandlerArgs, HandlerData},
};

extern crate alloc;

pub mod consts {
    pub const ENTRY_POINT: &str = "_start";
    pub const INIT_ROUTINE: &str = "_init";
    pub const GROUP_LIMIT: i16 = 9_999;
}
pub mod error;
pub mod flags;
pub mod structs;

pub type HandlerReturn = Result<HandlerData, TasmError>;
pub type HandlerFn = for<'a> fn(HandlerArgs<'a>) -> HandlerReturn;

#[macro_export]
macro_rules! verbose_log {
    ($this:expr, $($arg:tt)*) => {
        if $this.logs_enabled {
            println!($($arg)*);
        }
    };
}

#[macro_export]
macro_rules! log {
    ($on:expr, $($t:tt)*) => {
        if $on {
            println!($($t)*);
        }
    };
}

pub fn push_error(
    errors: &mut Vec<TasmError>,
    file: &str,
    etype: TasmErrorType,
    line: usize,
    rtn: String,
    details: String,
    errcode: i32,
) {
    errors.push(TasmError {
        etype,
        file: file.to_string(),
        routine: rtn,
        errcode,
        line,
        details,
    })
}

pub fn push_error_lineless(
    errors: &mut Vec<TasmError>,
    file: &str,
    etype: TasmErrorType,
    details: String,
    errcode: i32,
) {
    errors.push(TasmError {
        etype,
        file: file.to_string(),
        routine: String::new(),
        errcode,
        line: 0,
        details,
    })
}

pub fn print_errors(es: Vec<TasmError>, err_msg: &str) {
    println!("{err_msg} with {} errors:", es.len());
    for e in es {
        println!("{e}");
    }
}
