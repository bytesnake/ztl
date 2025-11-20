use std::path::PathBuf;
use serde::Serialize;
use std::fmt;
#[cfg(feature = "schedule")]
use colored::Colorize;

use ztl_base::notes::Changes;

pub type Result = ztl_base::error::Result<Output>;

#[derive(Serialize)]
pub enum Output {
    Init { root: PathBuf, existed: bool },
    Build { changes: Changes },
    Analyze { nnotes: usize, nlinks: usize },
    List { notes: Vec<Note> },
    #[cfg(feature = "schedule")]
    Schedule(Vec<ScheduleEntry>),
    #[cfg(feature = "mastodon")]
    Mastodon,
    #[cfg(feature = "anki")]
    Anki,
}

#[derive(Serialize)]
pub(crate) struct Note {
    pub(crate) key: String,
    pub(crate) header: String,
    pub(crate) kind: String,
    pub(crate) target: String
}

#[derive(Serialize)]
#[cfg(feature = "schedule")]
pub(crate) struct ScheduleEntry {
    pub(crate) key: String,
    pub(crate) header: String,
    pub(crate) state: String,
    pub(crate) label: String
}

impl fmt::Display for Output {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Init { root, existed} => {
                match existed {
                    true => write!(f, "ZTL repository already exists in {}\n", root.display())?,
                    false => write!(f, "Initialized empty ZTL repository in {}\n", root.display())?,
                }
            },
            Self::Build { changes } => {
                if !changes.has_any() {
                    write!(f, "Updated ZTL repository, no changes\n")?;
                } else {
                    write!(f, "Updated ZTL repository\n")?;
                    write!(f, "{}\n", changes)?;
                }
            },
            Self::Analyze { nnotes, nlinks } => write!(f, "Found {} notes with {} outgoing links\n", nnotes, nlinks)?,
            Self::List { notes } => {
                for note in notes {
                    write!(f, "{} {}\n", note.key, note.header)?;
                }
            },
            #[cfg(feature = "schedule")]
            Self::Schedule(entries) => {
                for entry in entries {
                    println!("{}\t {} {}", entry.state.magenta(), entry.label.green(), entry.header);
                }
            },
            #[cfg(feature = "mastodon")]
            Self::Mastodon => {},
            #[cfg(feature = "anki")]
            Self::Anki => {},
        }

        Ok(())
    }
}
