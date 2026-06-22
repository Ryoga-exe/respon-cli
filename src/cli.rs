use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(name = "respon", version, about = "Submit respon attendance")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Authenticate and submit am attendance code
    Attend(AttendArgs),
    /// Show the current status of an attendance code
    Status(QueryArgs),
}

#[derive(Args)]
pub struct AttendArgs {
    /// 9-digit respon attendance code
    #[arg(value_parser = validate_code)]
    pub code: String,

    /// Username; prompts when omitted
    #[arg(short = 'u', long, value_name = "USERNAME")]
    pub username: Option<String>,

    /// Password
    #[arg(long, value_name = "PASSWORD", conflicts_with = "password_stdin")]
    pub password: Option<String>,

    /// Read the password from stdin; requires --username
    #[arg(long, requires = "username", conflicts_with = "password")]
    pub password_stdin: bool,

    /// Override the HTTP User-Agent header
    #[arg(long, value_name = "USER_AGENT")]
    pub user_agent: Option<String>,

    /// Skip the final submissiopn confirmation
    #[arg(short = 'y', long)]
    pub yes: bool,

    #[arg(short = 'v', long)]
    pub verbose: bool,
}

#[derive(Args)]
pub struct QueryArgs {
    /// 9-digit respon attendance code
    #[arg(value_parser = validate_code)]
    pub code: String,

    /// Override the HTTP User-Agent header
    #[arg(long, value_name = "USER_AGENT")]
    pub user_agent: Option<String>,

    #[arg(short = 'v', long)]
    pub verbose: bool,
}

fn validate_code(code: &str) -> Result<String, String> {
    if code.len() == 9 && code.bytes().all(|byte| byte.is_ascii_digit()) {
        Ok(String::from(code))
    } else {
        Err(String::from("code must be exactly 9 ASCII digits"))
    }
}
