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
    pub code: String,
}
