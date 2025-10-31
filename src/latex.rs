use std::path::PathBuf;
use std::process::Command;
use std::io::{self, Write};
use std::fs;
use anyhow::Result;
use once_cell::sync::Lazy;
use scraper::{Html, Selector, Element};
use regex::Regex;
use markup5ever::interface::tree_builder::TreeSink;

use crate::notes::{Outgoing, LineColumn, Span, Note, Card};
use crate::config::{Config, self};

#[derive(Default, Debug)]
struct LatexNote {
    env_kind: String,
    label: String,
    title: String,
    content: Vec<String>,
    start: usize,
    end: usize,
    parent: Option<String>,
}

pub fn latex_to_html(_config: &Config, content: String) -> String {
    let preamble = fs::read_to_string(config::get_config_path().parent().unwrap().join("preamble.tex")).unwrap_or(r#"\usepackage[destlabel=true, backref=false]{{hyperref}}
\usepackage{{amsmath, amsfonts, amsthm, thmtools, enumitem, mdframed}}
"#.to_string());

    let tmp_dir = tempfile::TempDir::new().unwrap();

    let out_file = tmp_dir.path().join("main.tex");
    let mut f = std::fs::File::create(out_file.to_str().unwrap()).unwrap();

    f.write(preamble.as_bytes()).unwrap();
    f.write(b"\\begin{document}").unwrap();
    f.write(content.as_bytes()).unwrap();
    f.write(b"\\end{document}").unwrap();

    let out_dir = tmp_dir.path().to_str().unwrap();
    let cfg_dir = std::env::current_dir().unwrap().join(".ztl").join("thmtav.cfg");
    let out = Command::new("make4ht")
        .args(["-a", "debug", "-c", cfg_dir.to_str().unwrap(), "-m", "draft", out_file.to_str().unwrap()])
        .current_dir(out_dir)
        .output().unwrap();

    if !out.status.success() {
        io::stdout().write_all(&out.stdout).unwrap();
        io::stderr().write_all(&out.stderr).unwrap();

        String::new()
    } else {
        let cont = std::fs::read_to_string(tmp_dir.path().join("main.html")).unwrap();

        let mut document = Html::parse_document(&cont);

        // remove all comments from HTML
        let rm = document.root_element().descendants().filter(|x| x.value().is_comment()).map(|x| x.id()).collect::<Vec<_>>();
        for id in rm {
            document.remove_from_parent(&id);
        }

        // remove empty links which are used as anchors
        let rm = document.select(&Selector::parse("a").unwrap()).filter(|x| x.attr("href").is_none()).map(|x| x.id()).collect::<Vec<_>>();

        for id in rm {
            document.remove_from_parent(&id);
        }

        // remove middle mrow for overline
        let rm = document.select(&Selector::parse("mover mrow mrow").unwrap())
            .map(|x| x.parent_element().unwrap())
            .map(|x| (x.id(), x.parent_element().unwrap().id())).collect::<Vec<_>>();

        for (a,b) in rm {
            document.reparent_children(&a,&b);
            document.remove_from_parent(&a);
        }

        let body = document.select(&Selector::parse("body div").unwrap()).next().unwrap();
        body.html()
    }
}

pub(crate) fn analyze(_config: &Config, content: &str, source: &PathBuf) -> Result<Vec<Note>> {
    let mut levels: Vec<LatexNote> = Vec::new();
    let mut notes = Vec::new();

    static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\\r\{(.*?)\}\{(.*?)\}").unwrap());
    static RE_CLOZE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"\\cloze(?:\[(?P<title>.*?)\])?\{(?P<target_id>.*?)\}\{(?P<content>.*?)\}"#).unwrap());
    static RE_ASSUMP: Lazy<Regex> = Lazy::new(|| Regex::new(r#"\\requires\{(?P<requirement>.*?)\}\{(?P<expression>.*?)\}"#).unwrap());

    for (i, line) in content.lines().map(|x| x.trim()).enumerate() {
        if line.starts_with("\\begin{") && line.contains("label") && line.contains("name") {
            let mut iter = line.chars().skip(7);
            let env_kind: String = iter.by_ref().take_while(|x| *x != '}').collect();

            let inner = iter.by_ref().skip(1)
                .take_while(|x| *x != ']').collect::<String>();

            let inner = inner.split(",")
                .map(|x| x.splitn(2, "=").map(str::trim).collect::<Vec<_>>())
                .filter(|x| x.len() == 2)
                .collect::<Vec<_>>();

            let mut note = LatexNote::default();
            for i in inner {
                match i[0] {
                    "label" => note.label = i[1].to_string(),
                    "name" => note.title = i[1].to_string(),
                    _ => {},
                }
            }
            note.env_kind = env_kind;
            note.start = i + 1;

            if let Some(level) = levels.last() {
                note.parent = Some(level.label.clone());
            }

            levels.push(note);
        }

        if let Some(mut note) = levels.pop() {
            if line.contains(&format!("\\end{{{}}}", &note.env_kind)) {
                note.content.push(line.to_string());
                note.end = i;
                notes.push(note);
            } else {
                note.content.push(line.to_string());
                levels.push(note);
            }
        }
    }

    if levels.len() > 0 {
        dbg!(&levels);
        panic!("Inbalance in environments found for {}", source.display());
    }

    notes.into_iter().map(|note| {
        let span = Span {
            source: Some(source.display().to_string()),
            start: LineColumn {
                line: note.start,
                column: None
            },
            end: LineColumn {
                line: note.end,
                column: None
            }
        };

        let outgoing = note.content.iter().enumerate().map(|(l,x)|
            RE.captures_iter(&x).into_iter().map(|x| {
                let span = Span {
                    source: None,
                    start: LineColumn {
                        line: span.start.line + l,
                        column: Some(x.get(0).unwrap().start() + 2),
                    },
                    end: LineColumn {
                        line: span.start.line + l,
                        column: Some(x.get(0).unwrap().end() + 1),
                    },
                };

                let parts = x.get(1).unwrap().as_str().split("#").collect::<Vec<_>>();
                let keywords = parts[1..].into_iter()
                    .map(|x| x.splitn(2, "=").collect::<Vec<_>>())
                    .map(|x| {
                        if x.len() == 1 {
                            return ("anchor".to_string(), x[0].to_string());
                        } else {
                            return (x[0].to_string(), x[1].to_string());
                        }
                    })
                    .collect();

                Outgoing {
                    target: parts[0].to_string(),
                    comment: String::new(),
                    label: x.get(2).unwrap().as_str().to_string(),
                    view: keywords,
                    span
                }
            }).collect::<Vec<_>>()
        ).flatten().collect();

        let content = note.content.join("\n");

        let clozes = RE_CLOZE.captures_iter(&content)
            .map(|caps| {
                let description = caps.name("title").map_or("", |m| m.as_str()).to_string();
                let target_id = caps.name("target_id").unwrap().as_str();
                let _content = caps.name("content").unwrap().as_str();

                Card::Cloze {
                    description,
                    target: target_id.to_string(),
                }
            });

        let assumptions = RE_ASSUMP.captures_iter(&content)
            .map(|caps| {
                let requirement = caps.name("requirement").unwrap().as_str();
                //let expression = caps.name("expression").unwrap().as_str();

                Card::Assumption {
                    target: requirement.to_string(),
                }
            });

        let mut cards = clozes.chain(assumptions)
            .collect::<Vec<_>>();

        cards.sort_by_key(|x| match x {
                Card::Cloze { target, ..} => target.clone(),
                Card::Assumption { target, .. } => target.clone(), });

        cards.dedup_by_key(|x| match x {
                Card::Cloze { target, .. } => target.clone(),
                Card::Assumption { target, .. } => target.clone(), });

        Ok(Note {
            id: note.label,
            header: note.title,
            kind: Some(note.env_kind),
            parent: note.parent,
            children: Vec::new(),
            outgoing,
            incoming: Vec::new(),
            span,
            target: None,
            hash: crate::utils::hash(&content),
            html: content,
            public: false,
            cards,
        })
    }).collect()
}
