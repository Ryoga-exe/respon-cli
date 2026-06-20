use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(name = "respon", version, about = "Submit respon attendance")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    Attend(AttendArgs),
}

#[derive(Args)]
pub struct AttendArgs {
    #[arg(value_parser = validate_code)]
    pub code: String,
}

fn validate_code(code: &str) -> Result<String, String> {
    if code.len() == 9 && code.bytes().all(|byte| byte.is_ascii_digit()) {
        Ok(String::from(code))
    } else {
        Err(String::from("code must be exactly 9 ASCII digits"))
    }
}
