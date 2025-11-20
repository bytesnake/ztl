use std::path::PathBuf;
use line_numbers::LinePositions;

use crate::{LineColumn, Span, Note, error::ParseReport, error::*};

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

    let bib = biblatex::Bibliography::parse(&content)
        .map_err(|err| {
            // find line of offending bib entry
            let line_positions = LinePositions::from(content);
            let (line1, col1) = line_positions.from_offset(err.span.start);
            let (line2, col2) = line_positions.from_offset(err.span.end);

            let (start, end) = spans.iter().filter(|x| line1.as_usize() <= x.1 && line1.as_usize() >= x.0)
                .next().unwrap();

            let note = Span {
                source: Some(source.clone()),
                start: LineColumn { line: start + 1, column: None },
                end: LineColumn { line: *end, column: None },
            };

            let problem = Span {
                source: Some(source.clone()),
                start: LineColumn { line: line1.as_usize() + 1, column: Some(col1) },
                end: LineColumn { line: line2.as_usize() + 1, column: Some(col2) },
            };

            Error::Parse(ParseReport::new(&note, &problem, &format!("{}", err.kind)))
        })?;

    bib.into_iter().zip(spans.into_iter()).map(|(bib, span)| {
        let span = Span {
            source: Some(source.clone()),
            start: LineColumn { line: span.0 + 1, column: None },
            end: LineColumn { line: span.1, column: None },
        };

        // resource field get precedence to file field, get precedence to 
        // URL field
        let resource = bib.get_as::<String>("resource")
            .or(bib.file().map(|x| format!("file:{}", x)))
            .or(bib.url().map(|x| format!("url:{}", x)))
            .ok();

        let header = bib.title().ok().
            and_then(|x| x.first()).map(|x| x.v.get())
            .unwrap_or("").to_string();

        let kind = Some(bib.entry_type.to_string());

        Ok(Note {
            id: bib.key.clone(),
            header, kind, 
            parent: None,
            children: Vec::new(),
            outgoing: Vec::new(),
            incoming: Vec::new(),
            html: String::new(),
            span,
            resource,
            hash: crate::utils::hash(&bib.key),
            public: false,
            cards: Vec::new(),
        })
    }).collect()
}
