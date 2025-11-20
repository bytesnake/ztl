use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Rect {
    pub(crate) x0: f32, 
    pub(crate) y0: f32, 
    pub(crate) x1: f32, 
    pub(crate) y1: f32
}

impl Rect {
    pub(crate) fn new(x0: f32, y0: f32, x1: f32, y1: f32) -> Self {
        Self { x0, y0, x1, y1 }
    }

    pub(crate) fn from_mupdf(a: mupdf::Rect) -> Self {
        Self {
            x0: a.x0, y0: a.y1, x1: a.x1, y1: a.y1 
        }
    }
}

pub(crate) type Page = u32;

/// Possible destination 
///
/// Querying the document results in a list of destinations, which
/// can be used to render a view of the document.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Destination {
    /// Target string, can be note key, search pattern or anchor name
    pub(crate) target: String,
    /// Position in the document, valid for PDF files
    pub(crate) position: Option<(Page, DestinationKind)>,
    /// The outer context of a destination (such as the paragraph)
    pub(crate) context: Option<(String, Rect)>,
}

/// Destination kind, modeled after PDF queries
#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum DestinationKind {
    XYZ {
        x: f32,
        y: f32,
        z: Option<f32>,
    },
    Fit,
    Bound(Rect),
}

#[derive(Deserialize, Serialize, Debug)]
pub(crate) enum Source {
    Notes,
    Resource(String),
}

#[derive(Deserialize, Serialize, Debug)]
pub(crate) struct SearchMeta {
    page: Option<u32>,
    text: Option<String>,
    anchor: Option<String>,
}


#[derive(Deserialize, Serialize, Debug)]
#[serde(tag = "type")]
pub(crate) enum Message {
    Switch(Source),
    Search { page: Option<u32>, text: Option<String>, },
    Render { dest: Destination },
}
