#![warn(clippy::std_instead_of_core, clippy::std_instead_of_alloc)]

extern crate alloc;

use std::{collections::HashMap, path::PathBuf};

use anyhow::{Error, Result, anyhow};
use clap::Parser;
use cli_clipboard::{ClipboardContext, ClipboardProvider};

use gdlib::cclocallevels::gdlevel::leveldata::{GDLevelHeaderKey, GDLevelHeaderValue};
use gdlib::cclocallevels::gdlevel::{CCLocalLevels, GDLevel};
use gdlib::cclocallevels::gdobj::ids::level_header;
use gdlib::cclocallevels::gdobj::serialise_objects;
use gdlib::core::get_cclocallevels_path;
use tasm_core::log;
use tungstenite::{Message, connect};

use crate::structs::Tasm;
use tasm_core::{error::ERROR_DOCS, print_errors};

use crate::decomp::decompile_gmd;
use crate::linker::{parse_module, post_link_processing};

mod decomp;
mod instr;
mod lexer;
mod linker;
mod structs;
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
    #[arg(long, short = 'D')]
    dependencies: bool,

    /// Show intermediate linker output. Used primarily for debugging.
    #[arg(long, short)]
    linker_output: bool,

    /// Disables the compiler from inserting the _init routine's triggers into the level. Does not skip parsing it. Useful when the _init routine is already in the level
    #[arg(long)]
    skip_init: bool,

    /// Makes the spawn trigger of the ioblock for the _start routine have `spawn ordered` disabled.
    #[arg(long, short)]
    unordered_start: bool,

    /// Decompiles the given .gmd file and dumps it into stdout.
    #[arg(long, short)]
    decompile: bool,
}

fn use_wslive(level: GDLevel, port: u16) -> Result<(), Error> {
    let ws_url = format!("ws://127.0.0.1:{}", port);
    let (mut socket, _response) = connect(&ws_url)?;

    let objects_str = match level.get_decrypted_data() {
        Some(data) => serialise_objects(data.objects),
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

fn export_to_savefile(level: GDLevel, logs_enabled: bool) -> Result<(), Error> {
    if let None = get_cclocallevels_path() {
        log!(logs_enabled, "Unable to find savefile. Please pass --gmd.");
        return Ok(());
    }

    let mut savefile = CCLocalLevels::from_local()?;
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
        !args.no_log && args.verbose_logs,
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

    if let Err(e) = post_link_processing(&mut main_module, main_path, module_cache, dependency_map)
    {
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

    if args.decompile {
        decompile_gmd(&args);
        return;
    }

    log!(!args.no_log, "Parsing tasm...");

    let (mut main_module, curr_group) = match parse_main(&args) {
        Ok(m) => m,
        Err(_) => return,
    };

    if args.linker_output && !args.no_log {
        println!("------- linker output -------");
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

    log!(!args.no_log, "Encoding level...");

    let mut level = match main_module.handle_routines_inner(
        &level_name,
        curr_group,
        args.skip_init,
        args.unordered_start,
    ) {
        Err(e) => {
            if !args.no_log {
                print_errors(e, "Unable to compile to level");
            }
            return;
        }
        Ok(l) => l,
    };

    log!(
        !args.no_log,
        "Using groups {} - {}",
        args.group_offset + 1,
        curr_group
    );

    if args.no_export {
        return;
    }

    if let Some(data) = level.get_decrypted_data_ref() {
        data.headers.insert(
            GDLevelHeaderKey::from_id(level_header::PLAFORMER_MODE),
            GDLevelHeaderValue::Bool(true),
        );
    }

    if args.clipboard {
        let mut ctx = ClipboardContext::new().unwrap();
        let obj_str = serialise_objects(level.get_decrypted_data().unwrap().objects);
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
