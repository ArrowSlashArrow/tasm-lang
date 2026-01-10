use std::fs;

use anyhow::Error;
use clap::Parser;

pub mod lexer;

#[derive(Parser)]
#[command(about, version, author)]
struct Args {
    /// Input file.
    infile: String,
    /// Whether or not to use release mode.
    /// Release mode optimises routines to be as fast as possible,
    /// which may make them an unreadable mess of triggers in the process.
    #[arg(long)]
    release: bool,
}

fn main() -> Result<(), Error> {
    let args = Args::parse();
    let file = fs::read_to_string(args.infile).unwrap();

    let tasm = lexer::parse_file(file);

    match tasm {
        Ok(t) => println!("Parsed file with 0 errors."),
        Err(e) => {
            println!("Parsed file with {} errors:", e.len());
            for err in e {
                println!("{err}");
            }
        }
    }

    Ok(())
}
