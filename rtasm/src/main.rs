use std::fs;

use anyhow::{Error, anyhow};
use clap::Parser;

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
     * export as gmd
     * export level name
     *
     */
}

fn main() -> Result<(), Error> {
    let args = Args::parse();
    println!("{}", args.mem_end_counter);
    let file = fs::read_to_string(args.infile).unwrap();

    let _tasm = match lexer::parse_file(file, args.mem_end_counter) {
        Ok(t) => {
            println!("Parsed file with 0 errors.");
            t
        }
        Err(e) => {
            for err in e.iter() {
                println!("{err}");
            }
            println!("Parsed file with {} errors.", e.len());
            return Err(anyhow!("bad tasm"));
        }
    };

    Ok(())
}
