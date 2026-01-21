use std::{fs, time::Instant};

use anyhow::{Error, anyhow};
use clap::Parser;

pub mod core;
pub mod instr;
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
    if args.infile == "test" {
        let file = fs::read_to_string("../programs/nuclear_reactor.tasm").unwrap();
        let parse_start = Instant::now();
        let tasm = match lexer::parse_file(file) {
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

        println!(
            "Parse time: {:.3}ms",
            parse_start.elapsed().as_micros() as f64 / 1000.0
        );
        return Ok(());
    }

    let file = fs::read_to_string(args.infile).unwrap();

    let tasm = match lexer::parse_file(file) {
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
