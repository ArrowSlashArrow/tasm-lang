use std::fs;

use anyhow::Error;
use clap::Parser;
use gdlib::gdlevel::Levels;

use crate::core::show_errors;

pub mod core;
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
    /// which will reduce readability in the editor.
    #[arg(long)]
    release: bool,

    #[arg(long, default_value_t = 9999i16, value_parser = clap::value_parser!(i16))]
    mem_end_counter: i16,
    /// Whether to export the copmiled level as a .gmd
    #[arg(long)]
    gmd: bool,

    /// Name of exported level
    #[arg(long, value_name = "STRING")]
    level_name: Option<String>,

    /// Starting group offset. Default value is 0
    #[arg(long, default_value_t = 0i16, value_parser = clap::value_parser!(i16))]
    group_offset: i16,

    /// Toggles verbose logging from the compiler
    #[arg(long)]
    verbose_logs: bool,

    /// Toggles printing of all errors if the input file is parsed with errors
    #[arg(long)]
    log_errors: bool,

    /// Does not require an entry point to be present in the input file.
    /// Useful for compiling utility programs that don't necessary contain an entry point.
    #[arg(long)]
    no_entry_point: bool,
}

fn main() -> Result<(), Error> {
    let args = Args::parse();
    println!("Parsing tasm...");
    let file = fs::read_to_string(&args.infile).unwrap();

    let mut tasm = match lexer::parse_file(
        file,
        args.mem_end_counter,
        args.group_offset,
        args.verbose_logs,
        args.log_errors,
        args.no_entry_point,
    ) {
        Ok(t) => t,
        Err(es) => {
            show_errors(es, &format!("Unable to compile {}", &args.infile));
            return Ok(());
        }
    };

    let level_name = match args.level_name {
        Some(l) => l,
        None => args.infile,
    };

    println!("Encoding level...");
    match tasm.handle_routines(&level_name) {
        Ok(level) => match args.gmd {
            true => level.export_to_gmd(&format!("{}.gmd", level_name))?,
            false => {
                let mut savefile = Levels::from_local()?;
                savefile.add_level(level);
                savefile.export_to_savefile()?;
                println!("exported to savefile.")
            }
        },
        Err(e) => {
            show_errors(e, "Unable to compile to level");
        }
    }

    Ok(())
}
