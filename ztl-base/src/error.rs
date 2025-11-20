use std::path::PathBuf;
use std::fmt;
use serde::Serialize;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("could not locate ZTL root")]
    RootNotFound(PathBuf),
    #[error("invalid TOML")]
    InvalidDeserialize(#[from] toml::de::Error),
    #[error("invalid TOML")]
    InvalidSerialize(#[from] toml::ser::Error),
    #[error("could not open")]
    InputOutput(#[from] std::io::Error),
    #[error("invalid note configuration in {0}\n{1}")]
    InvalidNote(PathBuf, toml::de::Error),
    #[error("invalid span file")]
    InvalidFileSpan(PathBuf, toml::de::Error),
    #[error("{0}")]
    Parse(ParseReport),
}

impl Error {
    pub fn to_serialize(self) -> ErrorSer {
        match self {
            Error::RootNotFound(x) => ErrorSer::RootNotFound(x),
            Error::InvalidDeserialize(x) => ErrorSer::InvalidDeserialize(x.to_string()),
            Error::InvalidSerialize(x) => ErrorSer::InvalidSerialize(x.to_string()),
            Error::InputOutput(x) => ErrorSer::InputOutput(x.to_string()),
            Error::InvalidNote(p, x) => ErrorSer::InvalidNote(p, x.to_string()),
            Error::InvalidFileSpan(p, x) => ErrorSer::InvalidFileSpan(p, x.to_string()),
            Error::Parse(x) => ErrorSer::Parse(x),
        }
    }
}

#[derive(Debug, Serialize)]
pub enum ErrorSer {
    RootNotFound(PathBuf),
    InvalidDeserialize(String),
    InvalidSerialize(String),
    InputOutput(String),
    InvalidNote(PathBuf, String),
    InvalidFileSpan(PathBuf, String),
    Parse(ParseReport),
}

#[derive(Debug, Serialize)]
pub struct ParseReport {
    inner: Vec<Span>,
}

impl ParseReport {
    pub fn empty() -> Self {
        ParseReport { inner: Vec::new() }
    }

    pub fn new(note: &crate::Span, reference: &crate::Span, reason: &str) -> ParseReport {
        ParseReport {
            inner: vec![Span { note: note.clone(), reference: reference.clone(), reason: reason.to_string() }]
        }
    }

    pub fn append(&mut self, report: ParseReport) {
        self.inner.extend(report.inner)
    }

    pub fn as_err(self) -> Result<()> {
        if self.inner.len() == 0 {
            return Ok(());
        } else {
            Err(Error::Parse(self))
        }
    }
}

#[derive(Debug, Serialize)]
struct Span {
    pub(crate) note: crate::Span,
    pub(crate) reference: crate::Span,
    pub(crate) reason: String,
}


impl fmt::Display for ParseReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use annotate_snippets::renderer::DecorStyle;
        use annotate_snippets::{AnnotationKind, Level, Renderer, Snippet};
        use crate::utils::line_col_to_byte_offset as offset;

        let report = self.inner.iter().map(|span| {
            let binding = span.note.source.clone().unwrap();
            let binding = binding.to_str().unwrap().to_string();

            let content = std::fs::read_to_string(span.note.source.as_ref().unwrap()).unwrap();

            let header_start = offset(&content, span.note.start.line, span.note.start.column.unwrap_or(1)).unwrap();
            let header_end = offset(&content, span.note.end.line, span.note.end.column.unwrap_or(1)).unwrap();

            let offset_start = offset(&content, span.reference.start.line, span.reference.start.column.unwrap_or(1)).unwrap();
            let offset_end = offset(&content, span.reference.end.line, span.reference.end.column.unwrap_or(200)).unwrap();

            Level::ERROR
                .primary_title("could not parse note")
                .element(
                    Snippet::source(content).path(binding)
                    .annotation(AnnotationKind::Primary.span(offset_start..offset_end)
                            .label(&span.reason))
                    .annotation(AnnotationKind::Context.span(header_start..header_end))
                )
            }).collect::<Vec<_>>();

        let renderer = Renderer::styled().decor_style(DecorStyle::Unicode);
        write!(f, "{}", renderer.render(&report))
    }
}
