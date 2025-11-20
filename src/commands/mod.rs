use std::path::PathBuf;
use clap::{Parser, Subcommand};
use crate::utils;
use ztl_base::{config::Config, error::{Error, Result}};

pub mod result;
mod watch;
#[cfg(feature = "anki")]
pub mod anki;
#[cfg(feature = "schedule")]
mod schedule;
#[cfg(feature = "mastodon")]
mod mastodon;

pub(crate) use watch::watch;
#[cfg(feature = "schedule")]
pub(crate) use schedule::schedule;
#[cfg(feature = "anki")]
pub(crate) use anki::ankify;
#[cfg(feature = "mastodon")]
pub(crate) use mastodon::publish;

#[derive(Parser, Debug)]
#[command(version, about, long_about = "Blaa")]
pub(crate) struct Cli {
    /// Enable debugging 
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub debug: u8,

    /// Supported output formats
    #[arg(short, long)]
    #[clap(value_enum, default_value_t = OutputFormat::Human)]
    pub format: OutputFormat,

    /// Root folder of ZTL repository
    #[arg(short, long, default_value = "", value_parser = utils::find_root)]
    pub root: Option<PathBuf>,

    /// Optional subcommand
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub(crate) struct Build {
    #[arg(short, long)]
    pub dry_run: bool,
}

#[derive(Debug, Clone, clap::ValueEnum, PartialEq)]
#[clap(rename_all = "kebab_case")]
pub(crate) enum OutputFormat {
    Human,
    JSON
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub(crate) struct Publish {
    #[arg(short, long)]
    pub delete_all: bool,
}

#[derive(Parser, Debug)]
pub(crate) struct Ankify {
    #[arg(short, long, default_value = "output.apkg")]
    pub out: String,
}

#[derive(Parser, Debug)]
pub(crate) struct Watch {
    #[arg(short, long, default_value = "stats")]
    pub show: String,
    #[arg(short, long, default_value_t = false)]
    pub http: bool,
}

#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
    /// Initialize a new ZTL repository
    Init,
    /// List all notes 
    List,
    /// Build all notes from scratch
    Build(Build),
    /// Watch files and rebuild
    Watch(Watch),
    /// Generate anki card deck (apkg) from notes
    #[cfg(feature = "anki")]
    Ankify(Ankify),
    /// Create a schedule from current active notes
    #[cfg(feature = "schedule")]
    Schedule,
    /// Publish notes to Mastodon instance
    #[cfg(feature = "mastodon")]
    Publish(Publish),
}

impl Cli {
    pub fn config(&self) -> Result<Config> {
        self.root.as_ref()
            .ok_or_else(|| Error::RootNotFound(self.root.clone().unwrap_or(std::path::PathBuf::new())))
            .and_then(|x| Config::from_root(&x))
    }
}
