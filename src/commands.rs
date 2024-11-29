use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub(crate) struct Cli {
    /// Enable debugging 
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub debug: u8,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub(crate) struct Publish {
    #[arg(short, long)]
    pub delete_all: bool,
}

#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
    /// Watch files and rebuild
    Watch,
    /// Publish notes to Mastodon instance
    Publish(Publish),
    /// Build all notes from scratch
    Build,
}
