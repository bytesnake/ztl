use std::{io::Write, fs};
use indexmap::IndexMap;
use std::path::Path;

use clap::Parser;
use sha2::Digest;
use anyhow::{Result, Context};
use notify::{Watcher, RecursiveMode,};
use which::which;

mod commands;
mod config;
mod notes;
mod markdown;
mod bibtex;
mod latex;
mod utils;
mod anki;

use config::Config;

fn main() -> Result<()> {
    let cli = commands::Cli::parse();

    // parse configuration
    let config = config::get_config_path();
    let config = fs::read_to_string(&config)
        .context("Failed to read configuration file")?;

    let mut config: config::Config = toml::from_str(&config)
        .context("Failed to parse configuration")?;

    config.public = config.public.into_iter().map(|x| glob::glob(&x).unwrap()).flatten().map(|x| x.unwrap().display().to_string()).collect();

    match cli.command {
        Some(commands::Commands::Build) => build(config),
        Some(commands::Commands::Publish(res)) => publish(config, res),
        Some(commands::Commands::Watch) => watch(config),
        Some(commands::Commands::Ankify(ankify)) => anki::ankify(config, &ankify.out),
        Some(commands::Commands::ListVim) => list_vim(config),
        None => analyze(),
    }
}

fn build(config: Config) -> Result<()> {
    println!("Rebuilding notes from scratch ..");

    let mut notes = notes::Notes::from_cache()
        .with_files("**/*.md", &config)?
        .with_files("**/*.bib", &config)?
        .with_files("**/*.tex", &config)?;

    notes.update_incoming_links();

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

fn list_vim(_config: Config) -> Result<()> {
    let notes = notes::Notes::from_cache();

    println!("[");
    for note in notes.notes.values() {
        println!("\t{{ \"key\": \"{}\", \"header\": \"{}\", \"kind\": \"{}\", \"target\": \"{}\" }},", note.id, note.header.replace("\\", "\\\\"), note.kind.as_ref().map(|x| x.as_str()).unwrap_or("note"), format!("{}:{}", note.span.source.as_ref().unwrap_or(&String::new()), note.span.start.line));
    }
    println!("{{}}]");

    Ok(())
}

fn watch(config: config::Config) -> Result<()> {
    use crossbeam_channel::unbounded;
    let (s, r) = unbounded();

    let c2 = config.clone();
    let mut watcher = notify::recommended_watcher(move |res| {
        match res {
            Ok(event) => {
                let event: notify::event::Event = event;

                let path = utils::diff_paths(event.paths.first().unwrap(), std::env::current_dir().unwrap()).unwrap();

                let path_str = path.display().to_string();
                if path_str.contains(".ztl") && !path_str.ends_with(".sixel.show") {
                    return;
                }

                let ext = path.extension().and_then(std::ffi::OsStr::to_str);
                if ext != Some("md") && ext != Some("bib") && ext != Some("tex") && ext != Some("show") {
                    return;
                }
                match event.kind {
                   notify::EventKind::Modify(notify::event::ModifyKind::Data(_)) => {
                       let path = path.to_str().unwrap().to_string();
                       s.send(path).unwrap();
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

    let mut notes = notes::Notes::from_cache()
        .with_files("**/*.md", &c2)?
        .with_files("**/*.bib", &c2)?
        .with_files("**/*.tex", &c2)?;

    loop {
        let mut queue = vec![];
        if let Ok(path) = r.recv() {
            queue.push(path);
            std::thread::sleep(std::time::Duration::from_millis(100));

            while let Ok(path) = r.try_recv() {
                queue.push(path);
            }
        }

        queue.sort();
        queue.dedup();

        for path in queue {
            if path.ends_with(".sixel.show") {
                let key = std::path::Path::new(&path).with_extension("");
                let key = key.file_stem().unwrap().to_str().unwrap();

                if !Path::new(".ztl/cache").join(key).with_extension("sixel").exists() {
                    let note = notes.notes.get(key).unwrap();
                    utils::render_html(&c2, &note.html, &key);
                }

                utils::show_note(&c2, &key);
                continue;
            }

            let new_notes = notes.update_files(&path, &config)?;

            for (key, x) in &new_notes {
                if x.has_changed() {
                    utils::render_html(&c2, &x.html, &key);
                    utils::show_note(&c2, &key);
                }
            }

            notes.notes.extend(new_notes);
            notes.update_incoming_links();

            write_notes(&notes)?;
        }
    }
}

fn publish(config: config::Config, cmds: commands::Publish) -> Result<()> {
    let published_path = config::get_config_path()
        .parent().unwrap().join("published");

    let mut hash: IndexMap<String, (String, String)> = fs::read_to_string(&published_path)
        .map(|x| toml::from_str(&x).unwrap())
        .unwrap_or(IndexMap::new());

    let toot_cmd = config.toot.unwrap_or_else(|| which("toot").unwrap().display().to_string());

    if cmds.delete_all {
        for k in hash.values() {
            let cmd = format!("{} delete {}", toot_cmd, &k.1);

            println!("Deleting {}", &k.1);
            let _out = std::process::Command::new("sh")
                .arg("-c")
                .arg(&cmd)
                .output()
                .expect("failed to execute process");
        }
        return Ok(());
    }

    let mut queue: Vec<String> = Vec::new();
    let notes = notes::Notes::from_cache().notes;
    let mut it = notes.keys();

    loop {
        let key = match queue.pop() {
            Some(x) => x,
            None => match it.next() {
                Some(x) => x.to_string(),
                None => break,
            },
        };

        let note = notes.get(&key).unwrap();

        // skip out of literature notes for now
        if note.html.trim().is_empty() {
            continue;
        }

        let html = note.html.replace("\n", " ").replace("xmlns=\"http://www.w3.org/1998/Math/MathML\"", "");

        match hash.get(&note.id).clone() {
            Some(x) => {
                if note.hash() == x.0 {
                    continue;
                }

                let visibility = match note.public {
                    true => "public",
                    false => "direct",
                };

                let html = utils::cleanup_links(&html, &notes, &hash);

                let cmd = format!("{} post --visibility {} '{}' --status-id {}", toot_cmd, visibility, &html, &x.1);
                println!("Changed {}", note.id);

                let _out = std::process::Command::new("sh")
                    .arg("-c")
                    .arg(&cmd)
                    .output()
                    .expect("failed to execute process");

                hash.get_mut(&note.id).unwrap().0 = note.hash();
            },
            None => {
                let parent = match &note.parent {
                    Some(parent) => {
                        if !hash.contains_key(parent) {
                            queue.push(note.id.clone());
                            queue.push(parent.clone());
                            println!("NEXT ELEMENT {:?}", queue);
                            continue;
                        }

                        Some(hash.get(parent).unwrap().1.clone())
                    },
                    _ => None,
                };

                // check if all outgoing links are available
                let mut some_missing = false;
                for link in &note.outgoing {
                    let note = notes.get(&link.target).unwrap();
                    // continue if this is an outgoing reference
                    if note.target.is_some() {
                        continue;
                    }

                    if !hash.contains_key(&link.target) {
                        if !some_missing {
                            queue.push(note.id.clone());
                        }
                        queue.push(link.target.clone());
                        some_missing = true;
                    }
                }
                if some_missing {
                    continue;
                }

                let html = utils::cleanup_links(&html, &notes, &hash);

                let visibility = match note.public {
                    true => "public",
                    false => "direct",
                };
                let mut cmd = format!("{} post --visibility {} '{}'", toot_cmd, visibility, &html);

                if let Some(parent) = parent {
                    cmd = format!("{} --reply-to {}", cmd, parent);
                };

                let out = std::process::Command::new("sh")
                    .arg("-c")
                    .arg(&cmd)
                    .output()
                    .expect("failed to execute process");

                let err = std::str::from_utf8(&out.stderr).unwrap();
                let out = std::str::from_utf8(&out.stdout).unwrap();
                dbg!(&err);
                let out = out.split("/").collect::<Vec<_>>();
                let out = out[out.len()-1].trim();

                println!("Publish {}", note.id);
                hash.insert(note.id.clone(), (note.hash(), out.to_string()));
            }
        }
    }

    let out_str = toml::to_string(&hash).unwrap();

    let mut file = std::fs::File::create(&published_path).unwrap();
    file.write(out_str.as_bytes()).unwrap();

    Ok(())
}
