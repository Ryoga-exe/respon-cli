use std::process::ExitCode;

use clap::Parser;
use respon_cli::{
    cli::{self, AttendArgs, Command},
    error::Result,
};

fn run() -> Result<u8> {
    let cli = cli::Cli::parse();
    match cli.command {
        Command::Attend(args) => attend(args),
    }
}
fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(error.exit_code())
        }
    }
}

fn attend(args: AttendArgs) -> Result<u8> {
    println!("{}", args.code);
    return Ok(0);
}
