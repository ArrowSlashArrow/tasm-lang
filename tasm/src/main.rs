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

    let in_file = fs::read(args.infile)?;

    Ok(())
}
