use std::fs;

use anyhow::{Error, anyhow};
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
    /// Input file.
    infile: String,
    /// Whether or not to use release mode.
    /// Release mode optimises routines to be as fast as possible,
    /// which will reduce readability in the editor.
    #[arg(long)]
    release: bool,

    #[arg(
        value_parser = clap::value_parser!(i16),
        default_value_t = 9999i16,
        long
    )]
    mem_end_counter: i16,
    /* args todo:
     * group offset
     */
    /// Whether to export the copmiled level as a .gmd
    #[arg(long)]
    gmd: bool,

    #[arg(long, value_name = "STRING")]
    level_name: Option<String>,
}

fn main() -> Result<(), Error> {
    let args = Args::parse();
    println!("Parsing tasm...");
    let file = fs::read_to_string(&args.infile).unwrap();

    let mut tasm = match lexer::parse_file(file, args.mem_end_counter) {
        Ok(t) => {
            println!("Parsed file with 0 errors.");
            t
        }
        Err(e) => {
            show_errors(e, "Unable to parse file");
            return Err(anyhow!("bad tasm"));
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
