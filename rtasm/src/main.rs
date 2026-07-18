#![warn(clippy::std_instead_of_core, clippy::std_instead_of_alloc)]

extern crate alloc;

use std::{collections::HashMap, path::PathBuf};

use anyhow::{Error, Result, anyhow};
use clap::Parser;
use cli_clipboard::{ClipboardContext, ClipboardProvider};
use gdlib::{
    core::get_local_levels_path,
    gdlevel::{Level, Levels},
    gdobj::GDObject,
};
use tungstenite::{Message, connect};

use crate::core::{error::ERROR_DOCS, print_errors, structs::Tasm};

use crate::linker::{parse_module, post_link_processing};

mod core;
mod instr;
mod lexer;
mod linker;
#[cfg(test)]
mod tests;

#[derive(Parser, Default)]
#[command(about, version, author)]
struct Args {
    /// Path to input file.
    #[arg(required_unless_present = "error_help")]
    infile: Option<String>,

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

    /// Sends the level to the clipboard instead of a file. This flag only works for users on Windows.
    /// The compiled objects can be pasted in via BetterEdit.
    #[arg(long, short)]
    clipboard: bool,

    /// Prints help for a specific error code.
    #[arg(long, short, default_value_t = 0)]
    error_help: usize,

    /// Displays all dependencies of the program being compiled
    #[arg(long, short)]
    dependencies: bool,

    /// Show intermediate linker output. Used primarily for debugging.
    #[arg(long, short)]
    linker_output: bool,

    /// Disables the compiler from inserting the _init routine's triggers into the level. Does not skip parsing it. Useful when the _init routine is already in the level
    #[arg(long)]
    skip_init: bool,
}

fn get_obj_str(obj: &Vec<GDObject>) -> String {
    obj.iter()
        .map(|obj| obj.serialise_to_string())
        .collect::<Vec<_>>()
        .join("")
}

fn use_wslive(mut level: Level, port: u16) -> Result<(), Error> {
    let ws_url = format!("ws://127.0.0.1:{}", port);
    let (mut socket, _response) = connect(&ws_url)?;

    let objects_str = match level.get_decrypted_data_ref() {
        Some(data) => get_obj_str(&data.objects),
        None => return Ok(()),
    };

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

fn parse_main(args: &Args) -> Result<(Tasm, i16)> {
    let main_path = PathBuf::from(&args.infile.clone().unwrap()).canonicalize()?;
    // note: the path for each module must be relative to the module! not doing so may cause overwrites in the cache.
    // relative path qualifier: (module, is done)
    let mut module_cache: HashMap<PathBuf, (Tasm, bool, Vec<usize>)> = HashMap::new();
    let mut dependency_map: HashMap<String, PathBuf> = HashMap::new(); // module ident => module path
    let mut start_using_this_group = args.group_offset;
    let (mut main_module, curr_group, _) = match parse_module(
        main_path.clone(),
        &args,
        &mut module_cache,
        &mut dependency_map,
        &mut start_using_this_group,
        !args.no_entry_point,
        !args.no_log,
    ) {
        Ok(m) => m,
        Err(e) => {
            if !args.no_log {
                println!("Unable to parse main module: {e}");
            }
            return Err(anyhow!(""));
        }
    };

    if args.dependencies && !args.no_log {
        println!("Using dependencies:");
        for k in module_cache.keys() {
            if k != &main_path {
                println!("* {}", k.to_str().unwrap_or("Not a UTF-8 path!"));
            }
        }
    }

    if let Err(e) = post_link_processing(&mut main_module, module_cache, dependency_map) {
        if !args.no_log {
            println!("Unable to compile to level");
            for err in e {
                println!("{err}");
            }
        }
        return Err(anyhow!(""));
    }

    return Ok((main_module, curr_group));
}

fn main() {
    let args = Args::parse();
    if args.error_help != 0 {
        match ERROR_DOCS.get(args.error_help) {
            Some(s) => println!("{s}"),
            None => println!("No documentation for E{:0>4}.", args.error_help),
        };
        return;
    }

    log!(!args.no_log, "Parsing tasm...");

    let id_limit = 9999;
    if args.mem_end_counter > id_limit {
        log!(
            !args.no_log,
            "You may not set the end counter beyond the ID limit of {id_limit}"
        );
        return;
    } else if args.mem_end_counter < 0 {
        log!(
            !args.no_log,
            "You may not set the end counter to a negative ID."
        );
        return;
    }

    let (mut main_module, curr_group) = match parse_main(&args) {
        Ok(m) => m,
        Err(_) => return,
    };

    if args.linker_output {
        println!("-------  linker output -------");
        for routine in main_module.routines.iter() {
            println!("{}: ({})", routine.ident, routine.group);
            for instr in routine.instructions.iter() {
                println!(
                    "    {} {} | {}",
                    instr.ident,
                    instr
                        .args
                        .iter()
                        .map(|a| format!("{a:?}"))
                        .collect::<Vec<_>>()
                        .join(", "),
                    instr
                        .flags
                        .iter()
                        .map(|a| format!("{a:?}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        }
        println!("----- end linker output -----");
    }

    main_module.release_mode = args.release;

    let level_name = match args.level_name {
        Some(l) => l,
        None => args.infile.unwrap(),
    };

    log!(
        !args.no_log,
        "Using groups {} - {}",
        args.group_offset + 1,
        curr_group
    );

    log!(!args.no_log, "Encoding level...");

    let level = match main_module.handle_routines_inner(&level_name, curr_group, args.skip_init) {
        Err(e) => {
            if !args.no_log {
                print_errors(e, "Unable to compile to level");
            }
            return;
        }
        Ok(l) => l,
    };

    if args.no_export {
        return;
    }

    if args.clipboard {
        let mut ctx = ClipboardContext::new().unwrap();
        let obj_str = get_obj_str(&level.get_decrypted_data().unwrap().objects);
        match ctx.set_contents(obj_str) {
            Ok(_) => log!(!args.no_log, "Sent to clipboard"),
            Err(e) => log!(!args.no_log, "Failed to send to clipboard: {e}"),
        };

        return;
    }

    match args.wslive {
        Some(port) => match use_wslive(level, port) {
            Ok(()) => log!(!args.no_log, "Sent to WSLive"),
            Err(e) => log!(!args.no_log, "Failed to send to WSLive: {}", e),
        },
        None => match args.gmd {
            true => {
                if let Err(e) = level.export_to_gmd(format!("{}.gmd", level_name)) {
                    println!("Failed to export to level file: {e}");
                }
            }
            false => {
                if let Err(e) = export_to_savefile(level, !args.no_log) {
                    log!(!args.no_log, "Unable to export to savefile: {e}")
                }
            }
        },
    }
}
