use std::fs;

use clap::Parser;

mod commands;
mod utils;

use ztl_base::{config::Config, notes::Notes, error::ParseReport};
use commands::{Cli, OutputFormat, Build};
use commands::result::{Result, Output};

fn main() -> anyhow::Result<()> {
    let cli = commands::Cli::parse();
    let cfg = cli.config();
    let format = cli.format.clone();

    let res = match cli.command {
        None => analyze(cfg?),
        Some(commands::Commands::Init) => init(cli),
        Some(commands::Commands::Build(ref cmd)) => build(cfg?, cmd),
        Some(commands::Commands::List) => list(cfg?),
        Some(commands::Commands::Watch(ref cmd)) => commands::watch(cfg?, cmd),
        #[cfg(feature = "anki")]
        Some(commands::Commands::Ankify(ankify)) => commands::ankify(cfg?, &ankify.out),
        #[cfg(feature = "schedule")]
        Some(commands::Commands::Schedule) => commands::schedule(cfg?),
        #[cfg(feature = "mastodon")]
        Some(commands::Commands::Publish(res)) => commands::publish(cfg?, res),
    };

    match format {
        OutputFormat::JSON => {
            let tmp = serde_json::to_string(&res.map_err(|x| x.to_serialize()))?;

            println!("{}", tmp);
            Ok(())
        },
        OutputFormat::Human => {
            res.map(|x| print!("{}", x)).map_err(anyhow::Error::from)
        },
    }
}

fn init(cli: Cli) -> Result {
    // construct ZTL repository in current directory
    let cwd = cli.root
        .unwrap_or(std::env::current_dir().unwrap());

    if cwd.join(".ztl").exists() {
        return Ok(Output::Init { root: cwd.join(".ztl"), existed: true });
    }

    // create all relevant folders
    fs::create_dir(cwd.join(".ztl"))?;
    fs::create_dir(cwd.join(".ztl").join("notes"))?;
    fs::create_dir(cwd.join(".ztl").join("files"))?;
    fs::create_dir(cwd.join(".ztl").join("cache"))?;

    // create empty configuration
    Config::empty(&cwd.join(".ztl").join("config"))?;

    return Ok(Output::Init { root: cwd.join(".ztl"), existed: false })
}

fn build(config: Config, cmd: &Build) -> Result {
    let mut report = ParseReport::empty();

    // update notes from files in repository
    let mut notes = Notes::from_cache(&config.ztl_root())?
        .update_files("**/*.bib", &config, &mut report)?
        .update_files("**/*.md", &config, &mut report)?
        .update_files("**/*.tex", &config, &mut report)?;

    notes.update_incoming_links();

    if !cmd.dry_run {
        notes.write_to_cache(&config.ztl_root())?;
    }

    report.as_err()
        .map(|_| Output::Build { changes: notes.collect_changes()})
}

fn analyze(config: Config) -> Result {
    let (nnotes, nlinks) = Notes::from_cache(&config.ztl_root())?.notes.values()
        .fold((0, 0), |a,b| (a.0 + 1, a.1 + b.outgoing.len()));

    Ok(Output::Analyze { nnotes, nlinks })
}

fn list(config: Config) -> Result {
    let notes = Notes::from_cache(&config.ztl_root())?;

    let notes = notes.notes.values()
        .map(|note| crate::commands::result::Note {
            key: note.id.clone(),
            header: note.header.replace("\\", "\\\\"), 
            kind: note.kind.as_ref().map(|x| x.as_str()).unwrap_or("note").to_string(),
            target: format!("{}:{}", note.span.source.as_ref().map(|x| x.display().to_string()).unwrap_or(String::new()), note.span.start.line)
        })
        .collect::<Vec<_>>();

    Ok(Output::List { notes })
}



