use std::path::PathBuf;
use anyhow::Result;

use crate::notes::{LineColumn, Span, Note};

pub(crate) fn analyze(content: &str, source: &PathBuf) -> Result<Vec<Note>> {
    let mut spans = Vec::new();

    for (i, line) in content.split("\n").enumerate() {
        if line.trim().starts_with("@") {
            spans.push((i,0))
        }
        if line.trim().starts_with("}") {
            spans.last_mut().map(|x| x.1 = i);
        }
    }

    let bib = biblatex::Bibliography::parse(&content).unwrap();

    bib.into_iter().zip(spans.into_iter()).map(|(bib, span)| {
        let span = Span {
            source: Some(source.to_str().unwrap().to_string()),
            start: LineColumn { line: span.0, column: None },
            end: LineColumn { line: span.1, column: None },
        };
        let target = bib.file().ok().or_else(|| bib.url().ok()).clone();

        Ok(Note {
            id: bib.key.clone(),
            header: bib.title().ok().and_then(|x| x.first()).map(|x| x.v.get()).unwrap_or("").to_string(),
            parent: None,
            outgoing: Vec::new(),
            incoming: Vec::new(),
            html: String::new(),
            span,
            target,
            hash: crate::utils::hash(&bib.key),
            public: false,
        })
    }).collect()
}
