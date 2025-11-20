use crossbeam_channel::unbounded;
use notify::{Watcher, RecursiveMode};
use std::process::{Command, Stdio};
use std::io::Write;
use std::thread;
use std::sync::{Arc, Mutex};
use std::path::PathBuf;

use tiny_http::{Server, Response};

use ztl_base::{config, error::{Result, ParseReport}};
use crate::{utils, commands::{result::Output, Watch}};

pub(crate) fn http_server(url: String, base: PathBuf, latest: Arc<Mutex<Option<String>>>) {
    thread::spawn(move || {
        let server = Server::http(&url).unwrap();

        for request in server.incoming_requests() {
            let current = latest.lock().unwrap().clone();
            let mut target = request.url().chars();
            target.next();
            let target = target.as_str();

            let target = if !target.is_empty() && target != "favicon.ico" {
                Some(target.to_string())
            } else {
                current
            };

            if let Some(current) = target {
                let note = match ztl_base::Note::from_path(&base.join("notes").join(&current)) {
                    Ok(note) => note,
                    Err(_) => {
                        println!("Could not find {}", current);
                        continue;
                    },
                };

                let html = note.render_html(".ztl/templates/*");
                let response = Response::from_string(html);
                let header = tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf8"[..]).unwrap();
                let response = response.with_header(header);
                request.respond(response).unwrap();
            }
        }
    });
}

pub(crate) fn watch(config: config::Config, cmd: &Watch) -> Result<Output> {
    let (s, r) = unbounded();

    let latest = Arc::new(Mutex::new(None));
    if cmd.http {
        http_server(config.preview.http_server.clone(), config.ztl_root(), latest.clone())
    }

    let c2 = config.clone();
    let root = config.root.clone();
    let mut watcher = notify::recommended_watcher(move |res| {
        match res {
            Ok(event) => {
                let event: notify::event::Event = event;

                let path = utils::diff_paths(event.paths.first().unwrap(), &root).unwrap();

                let path_str = path.display().to_string();
                if path_str.contains(".ztl") && !path_str.ends_with(".sixel.show") {
                    return;
                }

                let ext = path.extension().and_then(std::ffi::OsStr::to_str);
                if ext != Some("md") && 
                   ext != Some("bib") && 
                   ext != Some("tex") && 
                   ext != Some("show") {
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
    }).unwrap();

    watcher.watch(&config.root, RecursiveMode::Recursive).unwrap();

    let mut report = ParseReport::empty();

    let mut notes = crate::Notes::from_cache(&config.ztl_root())?
        .update_files("**/*.bib", &c2, &mut report)?
        .update_files("**/*.md", &c2, &mut report)?
        .update_files("**/*.tex", &c2, &mut report)?;

    report.as_err()?;

    println!("Watching for file changes ..");
    if cmd.http {
        println!("Listening to {}", config.preview.http_server);
    }

    let mut ztl_res = if cmd.show == "sixel" {
        let mut child = Command::new("ztl-res")
            .stdin(Stdio::piped())
            .stdout(Stdio::inherit())
            .spawn().unwrap(); // Launch process

        // Get handles to stdin and stdout
        let stdin = child.stdin.take().expect("Failed to open stdin");

        Some((child, stdin))
    } else {
        None
    };


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
                *latest.lock().unwrap() = Some(key.to_string());

                if let Some(ztl_res) = &mut ztl_res {
                    print!("{esc}c", esc = 27 as char);
                    std::io::stdout().flush().unwrap();
                    writeln!(ztl_res.1, "{{ \"type\": \"Render\", \"view\": \"{}\" }}", key).unwrap();
                    ztl_res.1.flush().unwrap();
                }

                continue;
            }

            print!("{esc}c", esc = 27 as char);


            let mut report = ParseReport::empty();
            notes = match notes.clone().update_files(&path, &config, &mut report) {
                Ok(mut notes) => {
                    notes.update_incoming_links();
                    notes.write_to_cache(&config.ztl_root())?;

                    match &mut ztl_res {
                        None => println!("{}", notes.collect_changes()),
                        Some(ztl_res) => {
                            let key = latest.lock().unwrap();

                            writeln!(ztl_res.1, "{{ \"type\": \"Render\", \"view\": \"{}\" }}", key.as_ref().unwrap()).unwrap();
                            ztl_res.1.flush().unwrap();
                        }
                    }

                    notes
                },
                Err(err) => { println!("{}", err); notes },
            };

            if let Err(err) = report.as_err() {
                println!("{}", err);
            }
        }
    }
}
