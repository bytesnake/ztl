use comrak::nodes::{AstNode, NodeValue, NodeHeading};
use comrak::{format_html, parse_document, Arena, Options};
use std::collections::HashMap;
use std::path::PathBuf;
use anyhow::Result;

use crate::notes::{Outgoing, LineColumn, Span, Note};

pub(crate) fn analyze<'a>(arena: &'a Arena<AstNode<'a>>, content: &str, source: &PathBuf) -> Result<Vec<Note>> {
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
            } else if level == levels.len() {
            levels[level-1] = key.clone();
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
        let root = arena.alloc(NodeValue::Document.into());
        root.append(child);

        nodes.push(
            (key.clone(), header, parent, root, span));
    }
    nodes.last_mut().map(|x| x.4.end = LineColumn {
        line: content.split("\n").count() - 1,
        column: None,
    });

    // parse notes to HTML and outgoing
    let notes = nodes.into_iter().map(|(key, header, parent, node, span)| {
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
        let html = String::from_utf8(html).unwrap();

        Note {
            id: key,
            header,
            parent,
            outgoing,
            incoming: Vec::new(),
            hash: crate::utils::hash(&html),
            html,
            span,
            file: None,
        }
    }).collect::<Vec<_>>();

    Ok(notes)
}
