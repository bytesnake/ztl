use indexmap::IndexMap;
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
    pub view: IndexMap<String, String>,
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
    pub kind: Option<String>,
    pub parent: Option<Key>,
    #[serde(default)]
    pub children: Vec<Key>,
    pub outgoing: Vec<Outgoing>,
    pub incoming: Vec<Key>,
    pub html: String,
    pub span: Span,
    pub resource: Option<String>,
    pub hash: String,
    #[serde(default)]
    pub public: bool,
    #[serde(default)]
    pub cards: Vec<Card>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct NodeOutgoing {
    target: Key,
    source: String,
    header: String,
    view: Option<String>,
    index: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Spans {
    target: Key,
    header: String,
    kind: Option<String>,
    outgoing: IndexMap<String, NodeOutgoing>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum Card {
    Cloze {
        description: String,
        target: String,
    },
    Assumption {
        target: String,
    },
}

impl Note {
    pub(crate) fn hash(&self) -> String {
        self.hash.clone()
    }

    pub(crate) fn is_tex(&self) -> bool {
        self.span.source.as_ref().map(|x| x.ends_with(".tex")).unwrap_or(false)
    }

    pub(crate) fn outgoing_spans(&self, notes: &Notes) -> Result<Spans> {
        self.outgoing.iter().enumerate().map(|(idx, s)| {
            let key = format!("{}:{},{}:{}", s.span.start.line,s.span.start.column.unwrap_or(1),s.span.end.line,s.span.end.column.unwrap_or(1));

            let target_note = notes.notes.get(&s.target)
                .with_context(|| format!("Could not get reference {} in line {}", &s.target, &s.span.start.line))?;

            let mut view = None;
            if let Some(anchor) = s.view.get("anchor") {
                let mut anchor = anchor.replacen(".", " ", 1);
                if let Some(r) = anchor.get_mut(0..1) {
                    r.make_ascii_uppercase();
                }
                view = Some(anchor);
            }
            if let Some(page) = s.view.get("page") {
                view = Some(format!("p. {}", page));
            }

            let target_node = NodeOutgoing {
                target: target_note.id.clone(),
                header: target_note.header.clone(),
                source: target_note.span.source.clone().unwrap(),
                index: idx,
                view,
            };

            Ok((key, target_node))
        }).collect::<Result<IndexMap<_, _>>>().map(|spans|
            Spans {
                target: self.id.clone(), header: self.header.clone(), kind: self.kind.clone(), outgoing: spans
            })
    }

    pub(crate) fn start_line(&self) -> String {
        self.span.start.line.to_string()
    }

    pub(crate) fn end_line(&self) -> String {
        (self.span.end.line + 1).to_string()
    }

    pub(crate) fn has_changed(&self) -> bool {
        let content = match fs::read_to_string(
            crate::config::get_config_path().parent().unwrap()
            .join("cache").join(&self.id)) {
            Ok(x) => x,
            Err(_) => return false,
        };

        let old: Note = toml::from_str(&content).unwrap();

        old.hash != self.hash
    }
}

#[derive(Debug)]
pub(crate) struct Notes {
    pub(crate) notes: IndexMap<Key, Note>,
}

impl Notes {
    pub fn from_cache() -> Self {
        let cache_path = crate::config::get_config_path()
            .parent().unwrap().join("cache");

        let notes = glob::glob(&format!("{}/*", cache_path.to_str().unwrap()))
            .unwrap().filter_map(|x| x.ok())
            .filter(|x| x.file_name().unwrap().len() != 64)
            .filter(|x| x.extension().is_none())
            .map(|x| toml::from_str(&std::fs::read_to_string(&x).expect(&format!("Could not open file {}", x.display()))).unwrap())
            .map(|x: Note| (x.id.clone(), x))
            .collect();

        Self { notes }
    }

    pub fn empty() -> Self {
        Self {
            notes: IndexMap::new(),
        }
    }

    pub fn with_files(mut self, pat: &str, config: &Config) -> Result<Self> {
        let notes = self.update_files(pat, config)?;

        self.notes.extend(notes);

        Ok(self)
    }

    pub fn update_files(&mut self, pat: &str, config: &Config) -> Result<IndexMap<Key, Note>> {
        let arena = comrak::Arena::new();
        glob(pat).unwrap()
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
            .map(|note| {
                let mut note = note?;
                note.public = note.span.source.as_ref().map(|x| config.public.contains(&x)).unwrap_or(false);

                if !note.is_tex() {
                    return Ok(note);
                }

                if let Some(x) = self.notes.get(&note.id) {
                    if x.hash == note.hash {
                        // take old HTML artifact without re-rendering
                        note.html = x.html.clone();
                        return Ok(note);
                    }
                }

                note.html = crate::latex::latex_to_html(&config, note.html);

                Ok(note)
            })
            .map(|x| x.map(|x| (x.id.clone(), x)))
            .collect()
    }

    pub fn update_incoming_links(&mut self) {
        let mut incoming: IndexMap<Key, Vec<Key>> = IndexMap::new();
        let mut children: IndexMap<Key, Vec<Key>> = IndexMap::new();

        for note in self.notes.values() {
            for link in &note.outgoing {
                let elms = incoming.entry(link.target.clone()).or_insert(vec![]);
                elms.push(note.id.clone());
            }

            if let Some(par) = &note.parent {
                let elms = children.entry(par.clone()).or_insert(vec![]);
                elms.push(note.id.clone());
            }
        }

        for note in self.notes.values_mut() {
            if incoming.contains_key(&note.id) {
                note.incoming = incoming.get(&note.id).unwrap().clone();
            }

            if children.contains_key(&note.id) {
                note.children = children.get(&note.id).unwrap().clone();
            }
        }
    }

    pub fn spans(&self) -> Result<Vec<(String, IndexMap<String, Spans>)>> {
        self.notes.values().sorted_by(|a,b| Ord::cmp(&a.span.source, &b.span.source))
            .chunk_by(|n| n.span.source.clone().unwrap())
            .into_iter()
            .map(|(file, notes)| {
                let notes = notes.into_iter().map(|note|
                    note.outgoing_spans(&self).map(|s| (format!("{}:{}", note.start_line(), note.end_line()), s))
                ).collect::<Result<IndexMap<_, Spans>>>();

                notes.map(|notes| (file, notes))
            })
            .collect()
    }
}

