pub mod config;
pub mod utils;
pub mod notes;
pub mod error;

#[cfg(feature = "parser")]
pub mod parser;

#[cfg(feature = "htmlrender")]
pub mod tera;

use std::fs;
use std::path::{Path, PathBuf};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use error::{Result, ParseReport};

/// Note key (any length, any character)
pub type Key = String;

/// Outgoing link targeted by a note
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Outgoing {
    /// Target note 
    pub target: Key,
    /// Comment on the purpose of the link
    pub comment: String,
    /// Displayed label of the link
    pub label: String,
    /// View modifiers (such as page number, anchor, search pattern etc.)
    pub view: IndexMap<String, String>,
    /// Span information, where to find the link in source
    pub span: Span,
}

/// Location in a file
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct LineColumn {
    pub line: usize,
    #[serde(default)]
    pub column: Option<usize>,
}

/// Location in a file with starting and ending index
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Span {
    /// Optional source relative to ZTL root
    #[serde(default)]
    pub source: Option<PathBuf>,
    /// Start position
    pub start: LineColumn,
    /// End position
    pub end: LineColumn
}

/// Note entry
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Note {
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NodeOutgoing {
    target: Key,
    source: String,
    header: String,
    view: Option<String>,
    index: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct File {
    source: PathBuf,
    spans: IndexMap<String, FileSpan>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileSpan {
    target: Key,
    header: String,
    kind: Option<String>,
    outgoing: IndexMap<String, NodeOutgoing>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Card {
    Cloze {
        description: String,
        target: String,
    },
    Assumption {
        target: String,
    },
}

impl Note {
    pub fn from_path(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }

    pub fn hash(&self) -> String {
        self.hash.clone()
    }

    pub(crate) fn is_tex(&self) -> bool {
        self.span.source.as_ref().map(|x| x.to_str().unwrap().ends_with(".tex")).unwrap_or(false)
    }

    pub(crate) fn outgoing_spans(&self, notes: &notes::Notes, report: &mut ParseReport) -> Result<FileSpan> {
        self.outgoing.iter().enumerate().filter_map(|(idx, s)| {
            let key = format!("{}:{},{}:{}", s.span.start.line,s.span.start.column.unwrap_or(1),s.span.end.line,s.span.end.column.unwrap_or(1));

            let target_note = match notes.notes.get(&s.target) {
                Some(x) => x,
                None => {
                    report.append(error::ParseReport::new(
                        &self.span,
                        &s.span,
                        "invalid reference"));

                    return None;
                }
            };

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
                source: target_note.span.source.as_ref().unwrap().display().to_string(),
                index: idx,
                view,
            };

            Some(Ok((key, target_node)))
        }).collect::<Result<IndexMap<_, _>>>().map(|spans|
            FileSpan {
                target: self.id.clone(), header: self.header.clone(), kind: self.kind.clone(), outgoing: spans
            })
    }

    pub(crate) fn start_line(&self) -> String {
        self.span.start.line.to_string()
    }

    pub(crate) fn end_line(&self) -> String {
        (self.span.end.line + 1).to_string()
    }

    pub fn has_changed(&self, root: &Path) -> bool {
        let content = match fs::read_to_string(root.join(".ztl").join("notes").join(&self.id)) {
            Ok(x) => x,
            Err(_) => return false,
        };

        let old: Note = toml::from_str(&content).unwrap();

        old.hash != self.hash
    }
}

impl PartialEq for Note {
    fn eq(&self, other: &Note) -> bool {
        self.hash == other.hash
    }
}
