use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::io::{self, Write};
use anyhow::Result;
use once_cell::sync::Lazy;
use scraper::{Html, Selector};
use regex::Regex;
use markup5ever::interface::tree_builder::TreeSink;

use crate::notes::{Outgoing, LineColumn, Span, Note};
use crate::config::Config;

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

fn latex_to_html(config: &Config, content: String) -> String {
    let tmp_dir = tempfile::TempDir::new().unwrap();

    let out_file = tmp_dir.path().join("main.tex");
    let mut f = std::fs::File::create(out_file.to_str().unwrap()).unwrap();
    let content = format!(r#"\documentclass{{article}}
\usepackage[destlabel=true, backref=false]{{hyperref}}
\usepackage{{amsmath, amsfonts, amsthm, thmtools, enumitem, mdframed}}

\DeclareMathOperator{{\prox}}{{prox}}

\declaretheorem[name=Satz]{{theorem}}
\declaretheorem[name=Beispiel]{{example}}
\begin{{document}}
{}
\end{{document}}
    "#, content); //&config.latex.preamble, content);

    f.write(content.as_bytes()).unwrap();

    let out_dir = tmp_dir.path().to_str().unwrap();
    let out = Command::new("make4ht")
        .args(["-c", "/home/losch@alabsad.fau.de/Note/.ztl/thmtav.cfg", "-m", "draft", out_file.to_str().unwrap()])
        .current_dir(out_dir)
        .output().unwrap();

    if !out.status.success() {
        io::stdout().write_all(&out.stdout).unwrap();
        io::stderr().write_all(&out.stderr).unwrap();

        String::new()
    } else {
        let cont = std::fs::read_to_string(tmp_dir.path().join("main.html")).unwrap();

        let mut document = Html::parse_document(&cont);

        let rm = document.root_element().descendants().filter(|x| x.value().is_comment()).map(|x| x.id()).collect::<Vec<_>>();
        for id in rm {
            document.remove_from_parent(&id);
        }

        let rm = document.select(&Selector::parse("a").unwrap()).filter(|x| x.attr("href").is_none()).map(|x| x.id()).collect::<Vec<_>>();

        for id in rm {
            document.remove_from_parent(&id);
        }

        let rm = document.root_element().descendants().filter(|x| x.value().as_text().map(|x| x.trim().is_empty()).unwrap_or(false)).map(|x| x.id()).collect::<Vec<_>>();
        for id in rm {
            document.remove_from_parent(&id);
        }

        let body = document.select(&Selector::parse("body div").unwrap()).next().unwrap();
        body.html()
    }
}

pub(crate) fn analyze(config: &Config, content: &str, source: &PathBuf) -> Result<Vec<Note>> {
    let mut levels: Vec<LatexNote> = Vec::new();
    let mut notes = Vec::new();

    static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\\hyperref\[(.*?)\]\{(.*?)\}").unwrap());

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
                        column: Some(x.get(0).unwrap().start()),
                    },
                    end: LineColumn {
                        line: span.start.line + l,
                        column: Some(x.get(0).unwrap().end()),
                    },
                };

                Outgoing {
                    target: x.get(1).unwrap().as_str().to_string(),
                    comment: String::new(),
                    label: x.get(2).unwrap().as_str().to_string(),
                    view: HashMap::new(),
                    span
                }
            }).collect::<Vec<_>>()
        ).flatten().collect();

        let content = note.content.join("\n");
        let html = latex_to_html(&config, content.clone());

        Ok(Note {
            id: note.label,
            header: note.title,
            parent: note.parent,
            outgoing,
            incoming: Vec::new(),
            html,
            span,
            file: None,
            hash: crate::utils::hash(&content),
        })
    }).collect()
}
