use std::collections::HashMap;
use std::fs;

use serde::{Serialize, Deserialize};
use itertools::Itertools;
use glob::glob;
use anyhow::{Result, Context};

use crate::config::Config;

pub(crate) type Key = String;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Outgoing {
    pub target: Key,
    pub comment: String,
    pub label: String,
    pub view: HashMap<String, String>,
    pub span: Span,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub(crate) struct LineColumn {
    pub line: usize,
    #[serde(default)]
    pub column: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub(crate) struct Span {
    #[serde(default)]
    pub source: Option<String>,
    pub start: LineColumn,
    pub end: LineColumn
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Note {
    pub id: Key,
    pub header: String,
    pub parent: Option<Key>,
    pub outgoing: Vec<Outgoing>,
    pub incoming: Vec<Key>,
    pub html: String,
    pub span: Span,
    pub file: Option<String>,
    pub hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct NodeOutgoing {
    target: Key,
    source: String,
    header: String,
    index: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Spans {
    target: Key,
    header: String,
    outgoing: HashMap<String, NodeOutgoing>,
}

impl Note {
    pub(crate) fn hash(&self) -> String {
        self.hash.clone()
    }

    pub(crate) fn outgoing_spans(&self, notes: &Notes) -> Result<Spans> {
        self.outgoing.iter().enumerate().map(|(idx, s)| {
            let key = format!("{}:{},{}:{}", s.span.start.line,s.span.start.column.unwrap_or(1),s.span.end.line,s.span.end.column.unwrap_or(1));

            let target_note = notes.notes.get(&s.target)
                .with_context(|| format!("Could not get reference {} in line {}", &s.target, &s.span.start.line))?;

            let target_node = NodeOutgoing {
                target: target_note.id.clone(),
                header: target_note.header.clone(),
                source: target_note.span.source.clone().unwrap(),
                index: idx,
            };

            Ok((key, target_node))
        }).collect::<Result<HashMap<_, _>>>().map(|spans|
            Spans {
                target: self.id.clone(), header: self.header.clone(), outgoing: spans
            })
    }

    pub(crate) fn start_line(&self) -> String {
        self.span.start.line.to_string()
    }

    pub(crate) fn has_changed(&self) -> bool {
        let content = fs::read_to_string(
            crate::config::get_config_path().parent().unwrap()
            .join("cache").join(&self.id)).unwrap();

        let old: Note = toml::from_str(&content).unwrap();

        old.hash != self.hash
    }
}

#[derive(Debug)]
pub(crate) struct Notes {
    pub(crate) notes: HashMap<Key, Note>,
}

impl Notes {
    pub fn from_cache() -> Self {
        let cache_path = crate::config::get_config_path()
            .parent().unwrap().join("cache");

        let notes = glob::glob(&format!("{}/*", cache_path.to_str().unwrap()))
            .unwrap().filter_map(|x| x.ok())
            .filter(|x| x.file_name().unwrap().len() != 64)
            .map(|x| toml::from_str(&std::fs::read_to_string(&x).unwrap()).unwrap())
            .map(|x: Note| (x.id.clone(), x))
            .collect();

        Self { notes }
    }

    pub fn from_files(pat: &str, config: &Config) -> Result<Self> {
        let arena = comrak::Arena::new();
        let notes: Result<HashMap<Key, Note>> = glob(pat).unwrap()
            .filter_map(|x| x.ok())
            .filter(|x| !x.display().to_string().contains(".ztl"))
            .map(|x| {
                let content = fs::read_to_string(&x)?;

                match x.extension().and_then(|x| x.to_str()) {
                    Some("md") => crate::markdown::analyze(&arena, &content, &x),
                    Some("bib") => crate::bibtex::analyze(&content, &x),
                    Some("tex") => crate::latex::analyze(config, &content, &x),
                    _ => panic!(""),
                }
            })
        .flatten_ok()
        .map(|x| x.map(|x| (x.id.clone(), x)))
        .collect();

        notes.map(|notes| Self { notes })
    }

    pub fn extend(mut self, other: Self) -> Self {
        self.notes.extend(other.notes);

        self
    }

    pub fn update_incoming_links(&mut self) {
        let mut incoming: HashMap<Key, Vec<Key>> = HashMap::new();

        for note in self.notes.values() {
            for link in &note.outgoing {
                let elms = incoming.entry(link.target.clone()).or_insert(vec![]);
                elms.push(note.id.clone());
            }
        }

        for note in self.notes.values_mut() {
            if incoming.contains_key(&note.id) {
                note.incoming = incoming.get(&note.id).unwrap().clone();
            }
        }
    }

    pub fn spans(&self) -> Result<Vec<(String, HashMap<String, Spans>)>> {
        self.notes.values().sorted_by(|a,b| Ord::cmp(&a.span.source, &b.span.source))
            .chunk_by(|n| n.span.source.clone().unwrap())
            .into_iter()
            .map(|(file, notes)| {
                let notes = notes.into_iter().map(|note|
                    note.outgoing_spans(&self).map(|s| (note.start_line(), s))
                ).collect::<Result<HashMap<_, Spans>>>();

                notes.map(|notes| (file, notes))
            })
            .collect()
    }
}

