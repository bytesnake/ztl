use std::fs;
use std::path::Path;
use indexmap::IndexMap;
use std::io::Write;
use std::fmt;

use crate::{*, error::{Result, Error}};

/// Collection of notes and associated files
#[derive(Debug, Clone)]
pub struct Notes {
    pub notes: IndexMap<Key, Note>,
    pub files: IndexMap<String, File>,
    pub(crate) changes: Vec<Change>,
}

#[derive(Debug, Clone)]
pub enum Change {
    NoteAdded(Key),
    NoteRemoved(Key, Note),
    NoteMoved(Key, PathBuf, PathBuf),
    NoteChanged(Key),
    FileAdded(PathBuf),
    FileRemoved(PathBuf),
}

#[derive(Serialize, Debug, Clone, Hash, Eq, PartialEq)]
pub enum FileChange {
    Noop,
    Add,
    Remove,
}

#[derive(Serialize, Debug, Clone)]
pub enum NoteChange {
    Add,
    Remove, 
    Modified(Option<PathBuf>),
}

#[derive(Serialize, Debug)]
pub struct Changes {
    inner: IndexMap<PathBuf, (FileChange, IndexMap<Key, (String, NoteChange)>)> 
}

impl Changes {
    pub fn has_any(&self) -> bool {
        self.inner.len() > 0
    }
}

impl fmt::Display for Changes {
     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
         for (path, inner) in &self.inner {
             write!(f, "{}", path.to_str().unwrap())?;
             match inner.0 {
                 FileChange::Add => write!(f, "+")?,
                 FileChange::Remove => write!(f, "-")?,
                 FileChange::Noop => {},
             }
             write!(f, "\n")?;

             for (id, (header, change)) in &inner.1 {
                 write!(f, "\t")?;
                 match change {
                     NoteChange::Add => write!(f, "+")?,
                     NoteChange::Remove => write!(f, "-")?,
                     NoteChange::Modified(_) => write!(f, " ")?,
                 }
                 write!(f, "{} {}\n", id, header)?;
             }
             
         }

         Ok(())
     }
}
    
impl Notes {
    pub fn from_cache(root: &Path) -> Result<Self> {
        // read all available notes
        let path = root.join("notes");

        let notes = glob::glob(&format!("{}/*", path.to_str().unwrap()))
            .unwrap().filter_map(|x| x.ok())
            .map(|x| {
                let content = std::fs::read_to_string(&x)?;

                let note: Note = toml::from_str(&content)
                    .map_err(|err| Error::InvalidNote(x, err))?;

                Ok((note.id.clone(), note))
            })
            .collect::<Result<_>>()?;

        // read all available file spans
        let path = root.join("files");

        let files = glob::glob(&format!("{}/*", path.to_str().unwrap()))
            .unwrap().filter_map(|x| x.ok())
            .map(|x| {
                let content = std::fs::read_to_string(&x)?;

                let file: File = toml::from_str(&content)
                    .map_err(|err| Error::InvalidFileSpan(x, err))?;

                let key = utils::hash(file.source.to_str().unwrap());

                Ok((key, file))
            })
            .collect::<Result<_>>()?;

        Ok(Self { notes, files, changes: Vec::new() })
    }

    pub fn empty() -> Self {
        Self {
            notes: IndexMap::new(),
            files: IndexMap::new(),
            changes: Vec::new()
        }
    }

    /// Update incoming links in notes
    ///
    /// Collect outgoing and parent links, and distribute
    /// to incoming and children attributes
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

    pub fn write_to_cache(&self, root: &Path) -> Result<()> {
        // write results to cache and toml files
        let file_path = root.join("files");

        // create folder if note yet exists
        let _ = fs::create_dir(&file_path);

        for (_key, file) in &self.files {
            if file.spans.len() == 0 {
                continue;
            }

            // hash file name
            let fname = utils::hash(&file.source.to_str().unwrap());
            let res = toml::to_string(&file)?;

            let fpath = file_path.join(&fname);
            if fpath.exists() && res == fs::read_to_string(&fpath).unwrap() {
                continue;
            }

            let mut f = fs::File::create(&fpath)?;
            f.write(&res.into_bytes())?;
        }

        let note_path = root.join("notes");

        let _ = fs::create_dir(&note_path);

        for note in self.notes.values() {
            let res = toml::to_string(&note)?;
            let fpath = note_path.join(&note.id);

            if fpath.exists() && res == fs::read_to_string(&fpath).unwrap() {
                continue;
            }
            let mut f = fs::File::create(&fpath)?;
            f.write(&res.into_bytes())?;
        }

        Ok(())
    }

    pub fn collect_changes(&self) -> Changes {
        let mut changes = IndexMap::new();
        let note = |key: &str| self.notes.get(key).unwrap();

        for change in self.changes.clone() {
            match change {
                Change::NoteAdded(key) => {
                    let note = note(&key);
                    let file_change = note.span.source.clone().unwrap();

                    let inner = changes.entry(file_change).or_insert((FileChange::Noop, IndexMap::new()));

                    inner.1.entry(note.id.clone()).or_insert((note.header.clone(), NoteChange::Add))
                        .1 = NoteChange::Add;
                },
                Change::NoteRemoved(_key, note) => {
                    let file_change = note.span.source.clone().unwrap();

                    let inner = changes.entry(file_change).or_insert((FileChange::Noop, IndexMap::new()));

                    inner.1.entry(note.id.clone()).or_insert((note.header.clone(), NoteChange::Remove))
                        .1 = NoteChange::Remove;
                },
                Change::NoteChanged(key) => {
                    let note = note(&key);
                    let file_change = note.span.source.clone().unwrap();

                    let inner = changes.entry(file_change).or_insert((FileChange::Noop, IndexMap::new()));

                    inner.1.entry(note.id.clone()).or_insert((note.header.clone(), NoteChange::Modified(None)))
                        .1 = NoteChange::Modified(None);
                },
                Change::NoteMoved(key, from, to) => {
                    let note = note(&key);
                    let inner = changes.entry(to).or_insert((FileChange::Noop, IndexMap::new()));

                    inner.1.entry(note.id.clone()).or_insert((note.header.clone(), NoteChange::Modified(Some(from.clone()))))
                        .1 = NoteChange::Modified(Some(from.clone()));
                },
                Change::FileAdded(path) => {
                    changes.entry(path).or_insert((FileChange::Add, IndexMap::new()))
                        .0 = FileChange::Add;
                },
                Change::FileRemoved(path) => {
                    changes.entry(path).or_insert((FileChange::Remove, IndexMap::new()))
                        .0 = FileChange::Remove;
                },
            }
        }

        Changes { inner: changes }
    }
}

