use std::{io, process::ExitCode};

use clap::{Parser, builder::Str};
use dialoguer::{Confirm, Input, Password};
use respon_cli::{
    cli::{AttendArgs, Cli, Command, QueryArgs},
    diagnostics::Diagnostics,
    error::{Error, Result},
    protocol::{AttendanceAccess, Credentials, PreparationStatus, ProbeStatus, ResponClient},
};
use zeroize::Zeroizing;

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
        Command::Status(args) => status(args),
    }
}
fn attend(args: AttendArgs) -> Result<u8> {
    let AttendArgs {
        code,
        username,
        password,
        password_stdin,
        user_agent,
        yes,
        verbose,
    } = args;
    let diagnostics = Diagnostics::new(verbose);
    let client = ResponClient::new(diagnostics, user_agent.as_deref())?;
    let access = match client.probe_code(&code)? {
        ProbeStatus::Available(access) => {
            println!("code accepted; card={}", access.card_id());
            access
        }
        ProbeStatus::Unavailable(rejection) => return Err(Error::Rejected(rejection.reason())),
    };

    let preparetion = match access {
        AttendanceAccess::AuthenticationRequired { login_url, .. } => {
            let credentials = read_credentials(username, password, password_stdin)?;
            client.prepare_after_authentication(&login_url, &credentials)?
        }
        AttendanceAccess::ConfirmationAvailable { page_url, .. } => {
            todo!("wip");
        }
    };

    match preparetion {
        PreparationStatus::AlreadySubmitted { url, completion } => {
            if let Some(order) = completion.and_then(|value| value.answer_order) {
                println!("already submitted: {url} (answer order {order})");
            } else {
                println!("already submitted: {url}");
            }
            Ok(0)
        }
        PreparationStatus::Confirmation(confirmation) => {
            println!("confirmation page reached: {}", confirmation.action);
            if !yes
                && !Confirm::new()
                    .with_prompt("Submit attendance?")
                    .default(false)
                    .interact()?
            {
                println!("aborted");
                return Ok(1);
            }

            let submitted = client.submit(&confirmation)?;
            if let Some(order) = submitted.completion.answer_order {
                println!("submitted: {} (answer order {order})", submitted.url);
            } else {
                println!("submitted: {}", submitted.url);
            }
            Ok(0)
        }
    }
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

fn read_credentials(
    username: Option<String>,
    password: Option<String>,
    password_stdin: bool,
) -> Result<Credentials> {
    let username = match username {
        Some(username) => username,
        None => Input::<String>::new().with_prompt("Username").interact()?,
    };
    if username.is_empty() {
        return Err(Error::Authentication("username is required".to_owned()));
    }

    let password = Zeroizing::new(match password {
        Some(password) => password,
        None if password_stdin => read_password_stdin()?,
        None => Password::new().with_prompt("Password").interact()?,
    });
    if password.is_empty() {
        return Err(Error::Authentication("password is required".to_owned()));
    }

    Ok(Credentials { username, password })
}

fn read_password_stdin() -> Result<String> {
    let mut password = String::new();
    io::stdin().read_line(&mut password)?;
    let trimmed_len = password.trim_end_matches(['\r', '\n']).len();
    password.truncate(trimmed_len);
    Ok(password)
}
