use std::path::*;
use std::process::Command;
use std::io::Write;
use sha2::Digest;

use crate::config::Config;

pub(crate) fn render_html(config: &Config, content: &str) {
    let mut f = std::fs::File::create(".ztl/cache/out.html").unwrap();
    f.write(content.as_bytes()).unwrap();
    f.flush().unwrap();

    let out = Command::new("bash").current_dir(".ztl/cache/").arg("-C").arg(&config.render).arg("out.html").spawn().unwrap().wait().unwrap(); 
}

pub(crate) fn hash(content: &str) -> String {
    let mut sha256 = sha2::Sha256::new();
    sha256.update(content);
    format!("{:X}", sha256.finalize())
}

pub fn diff_paths<P, B>(path: P, base: B) -> Option<PathBuf>
where
    P: AsRef<Path>,
    B: AsRef<Path>,
{
    let path = path.as_ref();
    let base = base.as_ref();

    if path.is_absolute() != base.is_absolute() {
        if path.is_absolute() {
            Some(PathBuf::from(path))
        } else {
            None
        }
    } else {
        let mut ita = path.components();
        let mut itb = base.components();
        let mut comps: Vec<Component> = vec![];
        loop {
            match (ita.next(), itb.next()) {
                (None, None) => break,
                (Some(a), None) => {
                    comps.push(a);
                    comps.extend(ita.by_ref());
                    break;
                }
                (None, _) => comps.push(Component::ParentDir),
                (Some(a), Some(b)) if comps.is_empty() && a == b => (),
                (Some(a), Some(b)) if b == Component::CurDir => comps.push(a),
                (Some(_), Some(b)) if b == Component::ParentDir => return None,
                (Some(a), Some(_)) => {
                    comps.push(Component::ParentDir);
                    for _ in itb {
                        comps.push(Component::ParentDir);
                    }
                    comps.push(a);
                    comps.extend(ita.by_ref());
                    break;
                }
            }
        }
        Some(comps.iter().map(|c| c.as_os_str()).collect())
    }
}
