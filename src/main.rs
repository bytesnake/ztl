use std::{io::Write, fs};
use std::collections::HashMap;
use std::path::Path;

use clap::Parser;
use sha2::Digest;
use anyhow::{Result, Context};
use notify::{Watcher, RecursiveMode,};

mod commands;
mod config;
mod notes;
mod markdown;
mod bibtex;
mod utils;

fn main() -> Result<()> {
    let cli = commands::Cli::parse();

    // parse configuration
    let config = config::get_config_path();
    let config = fs::read_to_string(&config)
        .context("Failed to read configuration file")?;

    let config: config::Config = toml::from_str(&config)
        .context("Failed to parse configuration")?;

    match cli.command {
        Some(commands::Commands::Build) => build(),
        Some(commands::Commands::Publish) => publish(&config),
        Some(commands::Commands::Watch) => watch(&config),
        None => analyze(),
    }
}

fn build() -> Result<()> {
    println!("Rebuilding notes from scratch ..");

    let mut notes = notes::Notes::from_files("**/*.md")?
        .extend(notes::Notes::from_files("**/*.bib")?);

    notes.update_incoming_links();

    //let texs = glob("**/*.tex").unwrap().collect::<Vec<_>>();

    write_notes(&notes)
}

fn write_notes(notes: &notes::Notes) -> Result<()> {
    // write results to cache and toml files
    let cache_path = config::get_config_path()
        .parent().unwrap().join("cache");

    let _ = fs::create_dir(&cache_path);

    for (file, spans) in notes.spans()? {
        if spans.len() == 0 {
            continue;
        }

        // hash file name
        let mut sha256 = sha2::Sha256::new();
        sha256.update(&file);
        let file = format!("{:X}", sha256.finalize());

        let res = toml::to_string(&spans).unwrap();
        let mut f = fs::File::create(cache_path.join(&file)).unwrap();
        f.write(&res.into_bytes()).unwrap();
    }

    for note in notes.notes.values() {
        let res = toml::to_string(&note).unwrap();
        let mut f = fs::File::create(cache_path.join(&note.id)).unwrap();
        f.write(&res.into_bytes()).unwrap();
    }

    Ok(())
}

fn analyze() -> Result<()> {
    let (nnotes, nlinks) = notes::Notes::from_cache().notes.values()
        .fold((0, 0), |a,b| (a.0 + 1, a.1 + b.outgoing.len()));

    println!("Found {} notes with {} outgoing links", nnotes, nlinks);

    Ok(())
}

fn watch(_config: &config::Config) -> Result<()> {
    use crossbeam_channel::unbounded;
    let (s, r) = unbounded();

    let mut watcher = notify::recommended_watcher(move |res| {
        match res {
            Ok(event) => {
                let event: notify::event::Event = event;

                let path = utils::diff_paths(event.paths.first().unwrap(), std::env::current_dir().unwrap()).unwrap();
                if path.extension().and_then(std::ffi::OsStr::to_str) != Some("md") {
                    return;
                }
                match event.kind {
                   notify::EventKind::Modify(notify::event::ModifyKind::Data(_)) => {
                       let path = path.to_str().unwrap().to_string();

                       println!("Update file {} ..", path);
                       notes::Notes::from_files(&path).unwrap()
                           .notes.into_values()
                           .for_each(|x| s.send(x).unwrap());
                   },
                   _ => {}
               }
           },
           Err(e) => println!("watch error: {:?}", e),
        }
    })?;

    watcher.watch(Path::new("."), RecursiveMode::Recursive)
        .context("Cannot create file watcher")?;

    println!("Watching for file changes ..");

    let mut notes = notes::Notes::from_files("**/*.md")?
        .extend(notes::Notes::from_files("**/*.bib")?);

    while let Ok(x) = r.recv() {
        notes.notes.insert(x.id.clone(), x);
        notes.update_incoming_links();

        write_notes(&notes)?;
    }

    Ok(())
}

fn publish(_config: &config::Config) -> Result<()> {
    let published_path = config::get_config_path()
        .parent().unwrap().join("published");

    let mut hash: HashMap<String, (String, String)> = fs::read_to_string(&published_path)
        .map(|x| toml::from_str(&x).unwrap())
        .unwrap_or(HashMap::new());

    for note in notes::Notes::from_cache().notes.values() {
        match hash.get(&note.id).clone() {
            Some(x) => {
                if note.hash() == x.0 {
                    continue;
                }

                println!("Changed {}", note.id);
                hash.insert(note.id.clone(), (note.hash(), "".to_string()));
            },
            None => {
                println!("Publish {}", note.id);
            }
        }
    }

    Ok(())
}
