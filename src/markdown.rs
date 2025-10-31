use comrak::nodes::{AstNode, NodeValue, NodeHeading};
use comrak::{format_html, parse_document, Arena, Options};
use indexmap::IndexMap;
use std::path::PathBuf;
use anyhow::Result;

use crate::notes::{Outgoing, LineColumn, Span, Note};

pub(crate) fn analyze<'a>(arena: &'a Arena<AstNode<'a>>, content: &str, source: &PathBuf) -> Result<Vec<Note>> {
    let root = parse_document(&arena, content, &Options::default());

    // first separate document into notes
    let mut nodes: Vec<(String, String, Option<String>, &'a AstNode<'a>, Span, usize)> = vec![];
    let mut levels = Vec::new();

    for child in root.children() {
        let (key, header, parent, level) = if let NodeValue::Heading(NodeHeading { level, .. }) = &child.data.borrow().value {
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
            } else if level == levels.len() {
                levels[level-1] = key.clone();
            } else {
                levels.truncate(level);
                levels[level-1] = key.clone();
            };

            let parent = levels.get(level - 2).map(|x: &String| x.to_string());
            (key, header, parent, level)
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

        let root = arena.alloc(NodeValue::Document.into());
        root.append(child);

        nodes.push(
            (key.clone(), header, parent, root, span, level));
    }

    // find ending of notes
    for i in 0..nodes.len() {
        let mut index = nodes.len();
        'inner: for j in (i+1)..nodes.len() {
            if nodes[j].5 <= nodes[i].5 {
                index = j;
                break 'inner;
            }
        }
            
        if index < nodes.len() {
            nodes[i].4.end = LineColumn {
                line: nodes[index].4.start.line - 2,
                column: None,
            };
        } else {
            nodes[i].4.end = LineColumn {
                line: content.split("\n").count() - 2,
                column: None,
            };
        };
    }

    // parse notes to HTML and outgoing
    let notes = nodes.into_iter().map(|(key, header, parent, node, span, _)| {
        let mut outgoing: Vec<Outgoing> = vec![];

        for node in node.descendants() {
            let pos = node.data.borrow().sourcepos;

            if let NodeValue::Link(link) = &node.data.borrow().value {
                let parts = link.url.split("#").collect::<Vec<_>>();
                let (target, parts) = parts.split_at(1);
                let view = parts.into_iter()
                    .map(|x| x.splitn(2, "=").collect::<Vec<_>>())
                    .map(|x| (x[0].to_string(), x[1].to_string()))
                    .collect::<IndexMap<_, _>>();

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
            for child in node.children() {
                let replace_by = if let NodeValue::Heading(NodeHeading { .. }) = child.data.borrow_mut().value {
                    let label = match &child.first_child().unwrap().data.borrow().value {
                        NodeValue::Text(text) => text.clone(),
                        _ => panic!("No label available"),
                    };

                    //// check that the first character is ascii and lower-case
                    if label.starts_with(|x: char| !x.is_ascii() || x.is_ascii_uppercase()) {
                        continue;
                    }

                    let parts = label.splitn(2, " ").collect::<Vec<_>>();
                    let (key, header) = (parts[0].to_string(), parts[1].to_string());
                    
                    Some(format!("#{} — {}", key, header))
                } else {
                    None
                };

                if let Some(text) = replace_by {
                    child.data.borrow_mut().value = NodeValue::HtmlInline(format!("<h3>{}</h3>", text));

                    child.first_child().unwrap().detach();
                }
            }
        }

        let mut html = vec![];
        let mut opts = Options::default();
        opts.render.unsafe_ = true;
        format_html(&node, &opts, &mut html).unwrap();
        let html = String::from_utf8(html).unwrap();

        Note {
            id: key,
            header,
            kind: None,
            parent,
            children: Vec::new(),
            outgoing,
            incoming: Vec::new(),
            hash: crate::utils::hash(&html),
            html,
            span,
            target: None,
            public: false,
            cards: Vec::new(),
        }
    }).collect::<Vec<_>>();

    Ok(notes)
}
