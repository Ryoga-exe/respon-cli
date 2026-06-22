use std::process::ExitCode;

use clap::Parser;
use respon_cli::{
    cli::{AttendArgs, Cli, Command, QueryArgs},
    diagnostics::Diagnostics,
    error::{Error, Result},
    protocol::{ProbeStatus, ResponClient},
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
        Command::Attend(args) => attend(args),
        Command::Exists(args) => exists(args),
        Command::Available(args) => available(args),
        Command::Status(args) => status(args),
    }
}
fn attend(args: AttendArgs) -> Result<u8> {
    println!("{}", args.code);
    return Ok(0);
}

fn exists(args: QueryArgs) -> Result<u8> {
    let diagnostics = Diagnostics::new(args.verbose);
    let client = ResponClient::new(diagnostics, args.user_agent.as_deref())?;

    let exists = match client.probe_code(&args.code)? {
        ProbeStatus::Available(_) => true,
        ProbeStatus::Unavailable(rejection) => match rejection.exists() {
            Some(exists) => exists,
            None => return Err(Error::Rejected(rejection.reason())),
        },
    };
    println!("{exists}");
    Ok(if exists { 0 } else { 1 })
}

fn available(args: QueryArgs) -> Result<u8> {
    let diagnostics = Diagnostics::new(args.verbose);
    let client = ResponClient::new(diagnostics, args.user_agent.as_deref())?;

    let available = match client.probe_code(&args.code)? {
        ProbeStatus::Available(_) => true,
        ProbeStatus::Unavailable(rejection) if rejection.is_recognized() => false,
        ProbeStatus::Unavailable(rejection) => return Err(Error::Rejected(rejection.reason())),
    };
    println!("{available}");
    Ok(if available { 0 } else { 1 })
}

fn status(args: QueryArgs) -> Result<u8> {
    let diagnostics = Diagnostics::new(args.verbose);
    let client = ResponClient::new(diagnostics, args.user_agent.as_deref())?;

    match client.probe_code(&args.code)? {
        ProbeStatus::Available(access) => {
            println!("available; card={}", access.card_id());
        }
        ProbeStatus::Unavailable(rejection) => {
            println!("{}: {}", rejection.status(), rejection.reason());
        }
    }
    Ok(0)
}
