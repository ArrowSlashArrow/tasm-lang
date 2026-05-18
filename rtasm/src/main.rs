#![warn(clippy::std_instead_of_core, clippy::std_instead_of_alloc)]

extern crate alloc;

use std::{env, fs, path::PathBuf};

use anyhow::Error;
use clap::Parser;
use gdlib::gdlevel::{Level, Levels};
use tungstenite::{Message, connect};

use crate::{core::print_errors, debugger::emulate};

pub mod core;
pub mod debugger;
pub mod instr;
pub mod lexer;

#[cfg(test)]
mod tests;

#[derive(Parser)]
#[command(about, version, author)]
struct Args {
    /// Path to input file.
    infile: String,

    /// Whether or not to use release mode.
    /// Release mode optimises routines to be as fast as possible,
    /// but will reduce readability in the editor.
    #[arg(long, short)]
    release: bool,

    /// Ending counter ID of memory block. Does not apply to programs using new memory.
    #[arg(long, default_value_t = 9999i16, value_parser = clap::value_parser!(i16))]
    mem_end_counter: i16,

    /// Whether to export the compiled level as a .gmd
    #[arg(long, short)]
    gmd: bool,

    /// Whether to send the compiled level to WSLive (optionally specify port)
    #[arg(long, value_name = "PORT")]
    wslive: Option<u16>,

    /// Name of exported level
    #[arg(long, value_name = "STRING")]
    level_name: Option<String>,

    /// Starting group offset.
    #[arg(long, default_value_t = 0i16, value_parser = clap::value_parser!(i16))]
    group_offset: i16,

    /// Toggles verbose logging from the compiler
    #[arg(long, short)]
    verbose_logs: bool,

    /// Skips exporting the level.
    #[arg(long)]
    no_export: bool,

    /// Does not require an entry point to be present in the input file.
    /// Useful for compiling utility programs that don't necessarily contain an entry point.
    #[arg(long)]
    no_entry_point: bool,

    /// Disables logging to stdout from the compiler, including verbose logs.
    #[arg(long)]
    no_log: bool,

    /// Emulate the program in the tasm debugger
    #[arg(long, short)]
    emulate: bool,
}

fn use_wslive(mut level: Level, port: u16) -> Result<(), Error> {
    let ws_url = format!("ws://127.0.0.1:{}", port);
    let (mut socket, _response) = connect(&ws_url)?;

    let mut objects_str = String::new();
    if let Some(data) = level.get_decrypted_data_ref() {
        objects_str = data
            .objects
            .iter()
            .map(|obj| obj.serialise_to_string())
            .collect::<Vec<_>>()
            .join("");
    }

    let payload = format!(
        r#"{{
        "action": "ADD_OBJECTS",
        "objects": "{}",
        "close": true
    }}"#,
        objects_str
    );

    socket.send(Message::Text(payload.into()))?;
    let _ = socket.close(None);

    Ok(())
}

// Temporary function that will be used until the next version of gdlib when this gets fixed
// This function does not check for the linux savefile path since this version of gdlib
// does not implement saving to that location
fn get_local_levels_path() -> Option<PathBuf> {
    if let Ok(local_appdata) = env::var("LOCALAPPDATA") {
        let path = PathBuf::from(format!("{local_appdata}/GeometryDash/CCLocalLevels.dat"));
        if path.exists() {
            return Some(path);
        }
    }

    None
}

fn export_to_savefile(level: Level, logs_enabled: bool) -> Result<(), Error> {
    if let None = get_local_levels_path() {
        log!(logs_enabled, "Unable to find savefile. Please pass --gmd.");
        return Ok(());
    }

    let mut savefile = Levels::from_local()?;
    savefile.add_level(level);
    savefile.export_to_savefile()?;
    log!(logs_enabled, "Exported to savefile.");
    Ok(())
}

fn main() -> Result<(), Error> {
    let args = Args::parse();
    log!(!args.no_log, "Parsing tasm...");
    let file = fs::read_to_string(&args.infile)?;

    let id_limit = 9999;
    if args.mem_end_counter > id_limit {
        log!(
            !args.no_log,
            "You may not set the end counter beyond the ID limit of {id_limit}"
        );
        return Ok(());
    } else if args.mem_end_counter < 0 {
        log!(
            !args.no_log,
            "You may not set the end counter to a negative ID."
        );
        return Ok(());
    }

    let mut tasm = match lexer::parse_file(
        file,
        args.infile.clone(),
        args.mem_end_counter,
        args.group_offset,
        args.verbose_logs && !args.no_log,
        true,
        args.no_entry_point,
    ) {
        Ok(t) => t,
        Err(es) => {
            if !args.no_log {
                print_errors(es, &format!("Unable to compile {}", &args.infile));
            }
            return Ok(());
        }
    };

    tasm.release_mode = args.release;

    let level_name = match args.level_name {
        Some(l) => l,
        None => args.infile,
    };

    if args.emulate {
        // first, check if this program is valid
        let res = tasm.handle_routines(&level_name);
        match res {
            Err(es) => {
                if !args.no_log {
                    print_errors(es, "Unable to compile to level");
                }
            }

            Ok(_) => {
                emulate(tasm);
            }
        }
        return Ok(());
    }

    log!(
        !args.no_log,
        "Using groups {} - {}",
        args.group_offset + 1,
        tasm.curr_group
    );

    log!(!args.no_log, "Encoding level...");

    let out_level = tasm.handle_routines(&level_name);
    if let Err(e) = out_level {
        if !args.no_log {
            print_errors(e, "Unable to compile to level");
        }
        return Ok(());
    }
    let level = out_level.unwrap();

    if args.no_export {
        return Ok(());
    }

    match args.wslive {
        Some(port) => {
            if let Err(e) = use_wslive(level, port) {
                log!(!args.no_log, "Failed to send to WSLive: {}", e);
            } else {
                log!(!args.no_log, "Sent to WSLive");
            }
        }
        None => match args.gmd {
            true => level.export_to_gmd(format!("{}.gmd", level_name))?,
            false => {
                if let Err(e) = export_to_savefile(level, !args.no_log) {
                    log!(!args.no_log, "Unable to export to savefile: {e}")
                }
            }
        },
    }

    Ok(())
}
