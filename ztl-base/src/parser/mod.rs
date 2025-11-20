mod markdown;
mod latex;
mod bibtex;

use std::{path::PathBuf, fs};
use indexmap::{IndexSet, IndexMap};

use itertools::Itertools;
use glob::glob;
use glob_match::glob_match;

use comrak::Arena;
use comrak::nodes::AstNode;

use crate::{Note, Key, File, FileSpan, notes::{Notes, Change}, error::*, config::Config, utils};

impl Notes {
    pub fn update_files(mut self, pattern: &str, config: &Config, report: &mut ParseReport) -> Result<Self> {
        let arena = Arena::new();

        // find all files in cache, matching the pattern
        let files = self.files.iter()
            .filter(|(_,v)| glob_match(pattern, v.source.to_str().unwrap()))
            .map(|(_, v)| v.source.clone())
            .collect::<IndexSet<_>>();

        // find all files in filesystem, matching the pattern
        let files_local = glob(pattern).unwrap()
            .filter_map(|x| x.ok())
            .filter(|x| !x.display().to_string().contains(".ztl"))
            .collect::<IndexSet<_>>();

        // record removed files, and remove their notes as well
        let mut notes_removed = IndexMap::new();
        for file in files.difference(&files_local) {
            self.changes.push(Change::FileRemoved(file.clone()));
            
            let hash = utils::hash(&file.to_str().unwrap());
            for span in self.files.get(&hash).unwrap().spans.values() {
                notes_removed.insert(span.target.clone(), file.clone());
            }

            self.files.remove(&hash).unwrap();
        }

        // record new files, add their notes as well
        let mut changed_notes = Vec::new();
        for file in files_local.difference(&files) {
            let notes = match Self::parse_file(&arena, &file, config) {
                Ok(x) => x,
                Err(Error::Parse(x)) => {report.append(x); continue },
                x => x?,
            };

            if notes.len() == 0 {
                continue;
            }

            self.changes.push(Change::FileAdded(file.clone()));

            for note in &notes {
                if notes_removed.contains_key(&note.id) {
                    self.changes.push(Change::NoteMoved(note.id.clone(), notes_removed.get(&note.id).unwrap().clone(), file.clone()));
                    notes_removed.remove(&note.id).unwrap();
                } else {
                    self.changes.push(Change::NoteAdded(note.id.clone()));
                }
            }

            changed_notes.extend(notes.into_iter());
        }

        // and all remaining files in both sets
        for file in files_local.intersection(&files) {
            let notes = match Self::parse_file(&arena, &file, config) {
                Ok(x) => x,
                Err(Error::Parse(x)) => {report.append(x); continue },
                x => x?,
            };

            let hash = utils::hash(&file.to_str().unwrap());
            let notes_in_file = self.files.get(&hash).unwrap().spans.iter().map(|x| x.1.target.clone())
                .collect::<IndexSet<_>>();

            for note in notes_in_file.difference(&notes.iter().map(|x| x.id.clone()).collect::<IndexSet<_>>()) {
                notes_removed.insert(note.clone(), file.clone());
            }

            changed_notes.extend(notes.into_iter());
        }

        let changed_keys = changed_notes.iter().map(|x| x.id.clone()).collect::<IndexSet<_>>();
        let notes_removed = notes_removed.keys().cloned()
            .collect::<IndexSet<String>>();

        // possibly update notes
        for mut note in changed_notes {
            let (new, changed, old_html) = {
                match self.notes.get(&note.id) {
                    Some(old_note) => (false, *old_note != note, old_note.html.clone()),
                    None => (true, true, String::new()),
                }
            };
            
            note.public = note.span.source.as_ref()
                .map(|x| x.display().to_string())
                .map(|x| config.public.contains(&x)).unwrap_or(false);

            if note.is_tex() {
                if changed {
                    note.html = latex::latex_to_html(&config, &note)?;
                } else {
                    note.html = old_html;
                }
            }

            self.notes.insert(note.id.clone(), note.clone());

            if changed {
                self.changes.push(Change::NoteChanged(note.id.clone()));
            }
            if new {
                self.changes.push(Change::NoteAdded(note.id.clone()));
            }
        }

        // find all notes, previously removed, and not newly added
        // remove those finally
        for key in notes_removed.difference(&changed_keys) {
            let note = self.notes.remove(key).unwrap();
            self.changes.push(Change::NoteRemoved(key.clone(), note));
        }

        // update file spans for all modified keys
        for (k, v) in self.spans(changed_keys, report)? {
            self.files.insert(k, v);
        }

        Ok(self)
    }

    fn parse_file<'a>(arena: &'a Arena<AstNode<'a>>, p: &PathBuf, config: &Config) -> Result<Vec<Note>> {
        let content = fs::read_to_string(&p)?;

        match p.extension().and_then(|x| x.to_str()) {
            Some("md") => markdown::analyze(&arena, &content, &p),
            Some("bib") => bibtex::analyze(&content, &p),
            Some("tex") => latex::analyze(config, &content, &p),
            _ => panic!(""),
        }
    }

    pub fn spans(&self, keys: IndexSet<Key>, report: &mut ParseReport) -> Result<IndexMap<Key, File>> {
        keys.into_iter().map(|x| self.notes.get(&x).unwrap())
            .sorted_by(|a,b| Ord::cmp(&a.span.source, &b.span.source))
            .chunk_by(|n| n.span.source.clone().unwrap())
            .into_iter()
            .map(|(file, notes)| {
                let spans = notes.into_iter().map(|note|
                    note.outgoing_spans(&self, report).map(|s| (format!("{}:{}", note.start_line(), note.end_line()), s))
                ).collect::<Result<IndexMap<_, FileSpan>>>()?;

                let hash = utils::hash(&file.display().to_string());

                Ok((hash, File { source: PathBuf::from(file), spans }))
            })
            .collect()
    }
}
