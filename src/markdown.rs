use comrak::nodes::{AstNode, NodeValue, NodeHeading};
use comrak::{format_html, parse_document, Arena, Options};
use sha2::Digest;
use serde::{Serialize, Deserialize};
use std::path::PathBuf;

use std::collections::HashMap;

pub(crate) type Key = String;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Outgoing {
    target: Key,
    comment: String,
    label: String,
    view: HashMap<String, String>,
    span: Span,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub(crate) struct LineColumn {
    line: usize,
    #[serde(default)]
    column: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub(crate) struct Span {
    #[serde(default)]
    source: Option<String>,
    start: LineColumn,
    end: LineColumn
}

pub(crate) type Spans = HashMap<String, HashMap<String, String>>;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Note {
    pub(crate) id: Key,
    header: String,
    parent: Option<Key>,
    pub(crate) outgoing: Vec<Outgoing>,
    incoming: Vec<Key>,
    html: String,
    span: Span,
}

impl Note {
    pub fn all() -> Vec<Note> {
        let cache_path = crate::config::get_config_path()
            .parent().unwrap().join("cache");

        glob::glob(&format!("{}/*", cache_path.to_str().unwrap()))
            .unwrap().filter_map(|x| x.ok())
            .map(|x| toml::from_str(&std::fs::read_to_string(&x).unwrap()).unwrap())
            .collect()
    }

    pub(crate) fn hash(&self) -> String {
        let mut sha256 = sha2::Sha256::new();
        sha256.update(&self.html);
        format!("{:X}", sha256.finalize())
    }

    pub(crate) fn outgoing_spans(&self) -> HashMap<String, String> {
        let mut map = self.outgoing.iter().enumerate().map(|(idx, s)| {
            let key = format!("{}:{},{}:{}", s.span.start.line,s.span.start.column.unwrap_or(1),s.span.end.line,s.span.end.column.unwrap_or(1));

            (key, idx.to_string())
        }).collect::<HashMap<String,String>>();

        map.insert("target".to_string(), self.id.clone());

        map
    }

    pub(crate) fn start_line(&self) -> String {
        self.span.start.line.to_string()
    }
}

pub(crate) fn analyze<'a>(arena: &'a Arena<AstNode<'a>>, content: &str, source: &PathBuf) -> Vec<Note> {
    let root = parse_document(&arena, content, &Options::default());

    // first separate document into notes
    let mut nodes: Vec<(String, String, Option<String>, &'a AstNode<'a>, Span)> = vec![];
    let mut levels = Vec::new();

    for child in root.children() {
        let (key, header, parent) = if let NodeValue::Heading(NodeHeading { level, .. }) = &child.data.borrow().value {
            let label = match &child.first_child().unwrap().data.borrow().value {
                NodeValue::Text(text) => text.clone(),
                _ => panic!("No label available"),
            };

            // check that the first character is ascii and lower-case
            if label.starts_with(|x: char| !x.is_ascii() || x.is_ascii_uppercase()) {
                if nodes.len() > 0 {
                    nodes[nodes.len() - 1].3.append(child);
                }

                continue;
            }

            let parts = label.splitn(2, " ").collect::<Vec<_>>();
            let (key, header) = (parts[0].to_string(), parts[1].to_string());

            let level = *level as usize;
            if level > levels.len() + 1 {
                panic!("Numbering not consistent!");
            } else if level == levels.len() + 1 {
                levels.push(key.clone());
            } else {
                levels.truncate(level);
            };

            let parent = levels.get(level - 2).map(|x: &String| x.to_string());
            (key, header, parent)
        } else {
            if nodes.len() > 0 {
                nodes[nodes.len() - 1].3.append(child);
            }
            continue;
        };

        let pos = LineColumn {
            line: child.data.borrow().sourcepos.start.line,
            column: None,
        };

        let span = Span {
            source: Some(source.display().to_string()),
            start: pos.clone(),
            end: Default::default(),
        };

        nodes.last_mut().map(|x| x.4.end = pos);

        nodes.push(
            (key.clone(), header, parent, child, span));
    }
    nodes.last_mut().map(|x| x.4.end = LineColumn {
        line: content.split("\n").count() - 1,
        column: None,
    });

    // parse notes to HTML and outgoing
    let mut notes = nodes.into_iter().map(|(key, header, parent, node, span)| {
        let mut outgoing: Vec<Outgoing> = vec![];

        for node in node.descendants() {
            let pos = node.data.borrow().sourcepos;

            if let NodeValue::Link(link) = &node.data.borrow().value {
                let parts = link.url.split("#").collect::<Vec<_>>();
                let (target, parts) = parts.split_at(1);
                let view = parts.into_iter()
                    .map(|x| x.splitn(2, "=").collect::<Vec<_>>())
                    .map(|x| (x[0].to_string(), x[1].to_string()))
                    .collect::<HashMap<_, _>>();

                let label = match &node.first_child().unwrap().data.borrow().value {
                    NodeValue::Text(text) => text.clone(),
                    _ => panic!("No label available"),
                };

                let span = Span {
                    source: None,
                    start: LineColumn {
                        line: pos.start.line,
                        column: Some(pos.start.column),
                    },
                    end: LineColumn {
                        line: pos.end.line,
                        column: Some(pos.end.column),
                    }
                };

                outgoing.push(Outgoing {
                    target: target[0].to_string(),
                    comment: link.title.clone(),
                    label,
                    view,
                    span,
                });
            }
        }

        let mut html = vec![];
        format_html(&node, &Options::default(), &mut html).unwrap();

        Note {
            id: key,
            header,
            parent,
            outgoing,
            incoming: Vec::new(),
            html: String::from_utf8(html).unwrap(),
            span
        }
    }).collect::<Vec<_>>();

    let mut incoming: HashMap<Key, Vec<Key>> = HashMap::new();

    for note in &notes {
        for link in &note.outgoing {
            let elms = incoming.entry(link.target.clone()).or_insert(vec![]);
            elms.push(note.id.clone());
        }
    }

    for note in &mut notes {
        if incoming.contains_key(&note.id) {
            note.incoming = incoming.get(&note.id).unwrap().clone();
        }
    }

    notes
}
