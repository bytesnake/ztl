use std::path::{Path, PathBuf, Component};

pub(crate) fn search_upwards<F: Fn(&Path) -> bool>(p: &Path, cond: F) -> Option<&Path> {
    let mut cwd = Some(p);
    while let Some(p) = cwd {
        if cond(p) {
            return Some(p);
        }

        cwd = p.parent();
    }

    None
}

pub(crate) fn find_root(arg: &str) -> std::result::Result<PathBuf, String> {
    let path = if arg == "" {
        std::env::current_dir().unwrap()
    } else {
        PathBuf::from(arg)
    };

    Ok(search_upwards(&path, |x| x.join(".ztl").exists())
        .map(|x| x.to_path_buf())
        .map(|x| std::fs::canonicalize(x).unwrap())
        .unwrap_or(PathBuf::new()))
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

