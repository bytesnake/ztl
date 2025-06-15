#![allow(unstable)]

use std::path::*;
use std::process::Command;
use std::io::{Read, Write};
use sha2::Digest;
use scraper::{Html, Selector, Element};
use std::collections::HashMap;
use regex::Regex;
use std::os::unix::net::UnixStream;
use libc::{c_int, c_ulong, c_ushort, STDOUT_FILENO};
use libc::{ioctl, TIOCGWINSZ, winsize};
use nix::ioctl_read_bad;
use sixel_rs::{
    encoder::Encoder,
    optflags::{DiffusionMethod, Quality, SizeSpecification, ResampleMethod, ColorOption, ColorSelectionMethod},
    status::{Error, Status},
};

use crate::config::Config;
use crate::notes::Note;

pub(crate) fn render_html(config: &Config, content: &str, key: &str) {
    let mut f = std::fs::File::create(".ztl/cache/out.html").unwrap();
    f.write(r#"
<html>
<head><style>
math{font-size:17px;}
math[display="block"] {
padding: 10px 0 10px 0;
}
mtable{padding: 5px 0 0 0;}
mtd{padding: 0 5px 0 0 !important;}
mspace{margin-left:0.17em;}
.head {
display: block;
margin-bottom: 5px;
}
dl {
  display: grid;
  grid-template-columns: max-content auto;
}

dt {
  grid-column-start: 1;
}

dd {
  grid-column-start: 2;
}

div[class^='columns-']{
    display: flex;
    justify-content: space-between;
}

figcaption {
    border: 1px black solid;
    border-width: 1 0 1 0;
    padding-bottom: 1px;
}

figcaption .id {
    font-weight: bold;
}

.cmbx-10 {
    font-weight: bold;
}

.cmti-10 {
    font-style: italic;
}

.comment {
    color: lightseagreen;
}

figure {
    margin-left: 0;
    margin-right: 0;
}

.small-caps {
    font-variant: small-caps;
}

</style></head>
<body>"#.as_bytes()).unwrap();

    f.write(content.as_bytes()).unwrap();
    f.write(b"</body></html>").unwrap();
    f.flush().unwrap();

    let p = std::env::current_dir().unwrap().join(".ztl").join("cache");
    let p = p.into_os_string().into_string().unwrap();
    let mut out = UnixStream::connect("/tmp/socket_zettel").unwrap();
    out.write_all(format!("170,{},{}\n", key, p).as_bytes()).unwrap();
    out.flush().unwrap();
    let mut buf = Vec::new();
    let _ = out.read(&mut buf);

    //let out = Command::new("bash").current_dir(".ztl/cache/").arg("-C").arg(&config.render).arg("out.html").arg(key).spawn().unwrap().wait().unwrap(); 
}

pub(crate) fn show_note(config: &Config, key: &str) {
    let win_width = get_winwidth();

    print!("{esc}c", esc = 27 as char);
    let p = Path::new(".ztl/cache").join(key).with_extension("sixel");
    if win_width > 1200 {
        let mut fs = std::fs::File::open(p).unwrap();
        let mut buf = Vec::new();
        fs.read_to_end(&mut buf).unwrap();

        let _ = std::io::stdout().write_all(&buf);
        return;
    }

    let p2 = Path::new(".ztl/cache").join(key).with_extension("sixel2");
    let encoder = Encoder::new().unwrap();
    encoder.set_output(&p2).unwrap();
    encoder.set_width(SizeSpecification::Pixel(win_width as u64)).unwrap();
    encoder.set_diffusion(DiffusionMethod::Atkinson).unwrap();
    encoder.set_quality(Quality::High).unwrap();
    encoder.set_resampling(ResampleMethod::Bicubic).unwrap();
    encoder.set_num_colors(255).unwrap();
    encoder.set_color_select( ColorSelectionMethod::Histogram).unwrap();
    encoder.encode_file(&p).unwrap();


    let mut fs = std::fs::File::open(Path::new(".ztl/cache").join(key).with_extension("sixel2")).unwrap();
    let mut buf = Vec::new();
    fs.read_to_end(&mut buf).unwrap();

    let _ = std::io::stdout().write_all(&buf);
}

pub(crate) fn hash(content: &str) -> String {
    let mut sha256 = sha2::Sha256::new();
    sha256.update(content);
    format!("{:X}", sha256.finalize())
}

pub(crate) fn cleanup_links(content: &str, notes: &HashMap<std::string::String, Note>, hash: &HashMap<String, (String, String)>) -> String {
    let mut content = content.to_string();
    let document = Html::parse_document(&content);
    let binding = Selector::parse("a").unwrap();
    let links = document.select(&binding);

    for link in links {
        let href = link.attr("href").unwrap();
        let target = notes.get(href.splitn(2, "#").next().unwrap()).unwrap();
        let link = match &target.target {
            Some(x) => x,
            _ => continue,
        };

        let (mut anchor, mut page) = (None, None);
        for elm in href.split("#").skip(1) {
            let parts = elm.splitn(2, "=").collect::<Vec<_>>();
            if parts.len() == 1 {
                anchor = Some(parts[0]);
            } else if parts[0] == "page" {
                page = Some(parts[1]);
            }
        }

        let res = match (anchor, page) {
            (Some(a), _) => format!("{}#{}", link, a),
            (_, Some(a)) => format!("{}#page={}", link, a),
            (None, None) => link.clone(),
        };

        content = content.replace(&href, &res);
    }

    let re = Regex::new(r#"(?i)<a\s+[^>]*href="([^"]*)""#).unwrap();
    let content = re.replace_all(&content, |caps: &regex::Captures| {
        let old_href = &caps[1];

        if old_href.starts_with("http") {
            return caps[0].to_string();
        } 

        let refer = hash.get(old_href).map(|x| x.1.clone()).unwrap_or("".into());
        caps[0].replace(old_href, &format!("https://zettel.haus/@losch/{}", refer))
    })
    .to_string();

    return content;
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

fn get_winwidth() -> u16 {
    ioctl_read_bad!(winsize, TIOCGWINSZ, winsize);

    let mut w = winsize { ws_row: 0, ws_col: 0, ws_xpixel: 0, ws_ypixel: 0 };
    let r = unsafe { winsize(1, &mut w) };

    match r {
        Ok(0) => {
            w.ws_xpixel
        },
        _ => {
            panic!("Could not get window width!");
        }
    }
}
