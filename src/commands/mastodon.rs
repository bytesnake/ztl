use indexmap::IndexMap;
use which::which;
use std::fs;
use std::io::Write;
use regex::Regex;
use scraper::{Html, Selector};

use ztl_base::{config, notes::Notes, Note, error::Result};
use crate::commands::{self, result::Output};

pub(crate) fn publish(config: config::Config, cmds: commands::Publish) -> Result<Output> {
    let published_path = config.ztl_root().join("published");

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
        return Ok(Output::Mastodon);
    }

    let mut queue: Vec<String> = Vec::new();
    let notes = Notes::from_cache(&config.root)?.notes;
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

                let html = cleanup_links(&html, &notes, &hash);

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
                    if note.resource.is_some() {
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

                let html = cleanup_links(&html, &notes, &hash);

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

    Ok(Output::Mastodon)
}

#[allow(dead_code)]
pub(crate) fn cleanup_links(content: &str, notes: &IndexMap<String, Note>, hash: &IndexMap<String, (String, String)>) -> String {
    let mut content = content.to_string();
    let document = Html::parse_document(&content);
    let binding = Selector::parse("a").unwrap();
    let links = document.select(&binding);

    for link in links {
        let href = link.attr("href").unwrap();
        let target = notes.get(href.splitn(2, "#").next().unwrap()).unwrap();
        let link = match &target.resource {
            Some(x) => x,
            _ => continue,
        };

        let (mut anchor, mut page) = (None, None);
        for elm in href.split("#").skip(1) {
            let parts = elm.splitn(2, "=").collect::<Vec<_>>();
            if parts.len() == 1 {
                anchor = Some(parts[0]);
            } else if parts[0] == "page" {
                page = Some(parts[1]);
            }
        }

        let res = match (anchor, page) {
            (Some(a), _) => format!("{}#{}", link, a),
            (_, Some(a)) => format!("{}#page={}", link, a),
            (None, None) => link.clone(),
        };

        content = content.replace(&href, &res);
    }

    let re = Regex::new(r#"(?i)<a\s+[^>]*href="([^"]*)""#).unwrap();
    let content = re.replace_all(&content, |caps: &regex::Captures| {
        let old_href = &caps[1];

        if old_href.starts_with("http") {
            return caps[0].to_string();
        } 

        let refer = hash.get(old_href).map(|x| x.1.clone()).unwrap_or("".into());
        caps[0].replace(old_href, &format!("https://zettel.haus/@losch/{}", refer))
    })
    .to_string();

    return content;
}

