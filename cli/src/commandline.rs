use clap::{Parser, Subcommand};

#[derive(clap::Parser)]
#[command(name = "repmgr")]
pub struct CommandLine {
    #[arg(long)]
    pub config_file: String,

    #[command(subcommand)]
    command: RepmgrCommands,
}

impl CommandLine {
    // This is a "trick" to avoid including clap::Parser inside
    // the function that calls module's parse()
    pub fn parse_cli() -> Self {
        Self::parse()
    }
}

#[derive(Subcommand)]
enum RepmgrCommands {
    Primary {
        #[command(subcommand)]
        action: PrimaryCommands,
    },
}

#[derive(Subcommand)]
enum PrimaryCommands {
    Register,
}

#[derive(Subcommand)]
enum ClusterCommands {
    Show,
    Event,
}
