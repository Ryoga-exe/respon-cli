use std::process::ExitCode;

use clap::Parser;
use respon_cli::{
    cli::{AttendArgs, CheckArgs, Cli, Command, QueryArgs},
    diagnostics::Diagnostics,
    error::Result,
    protocol::ResponClient,
};

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(error.exit_code())
        }
    }
}
fn run() -> Result<u8> {
    let cli = Cli::parse();
    match cli.command {
        Command::Check(args) => check(args),
        Command::Attend(args) => attend(args),
        Command::Exists(args) => exists(args),
        Command::Status(args) => status(args),
    }
}
fn check(args: CheckArgs) -> Result<u8> {
    let diagnostics = Diagnostics::new(args.verbose);
    let client = ResponClient::new(diagnostics, args.user_agent.as_deref())?;
    client.check(&args.code);
    Ok(0)
}

fn attend(args: AttendArgs) -> Result<u8> {
    println!("{}", args.code);
    return Ok(0);
}

fn exists(args: QueryArgs) -> Result<u8> {
    let diagnostics = Diagnostics::new(args.verbose);
    let client = ResponClient::new(diagnostics, args.user_agent.as_deref())?;
    todo!("implement")
}

fn status(args: QueryArgs) -> Result<u8> {
    todo!("implement")
}
