use std::io::{Read, Write};
use std::fs;
use sha2::Digest;
use std::path::{PathBuf, Path};
use indexmap::IndexMap;
use mupdf::pdf::PdfObject;
use nix::libc::{TIOCGWINSZ, winsize};
use nix::ioctl_read_bad;

use crate::error;

pub(crate) fn hash(content: &str) -> String {
    let mut sha256 = sha2::Sha256::new();
    sha256.update(content);
    format!("{:X}", sha256.finalize())
}

pub(crate) fn fetch_and_cache(resource: &str) -> error::Result<Vec<u8>> {
    let out_dir = PathBuf::from("/tmp/ztl-res");
    fs::create_dir_all(&out_dir).unwrap();
    let fingerprint = hash(resource);

    let mut buf: Vec<u8> = vec![];
    if out_dir.join(&fingerprint).exists() {
        let mut f = fs::File::open(out_dir.join(&fingerprint)).unwrap();
        f.read_to_end(&mut buf).unwrap();
    } else {
        let mut resp = reqwest::blocking::get(resource).unwrap();
        dbg!(&resp.status());
        resp.copy_to(&mut buf).unwrap();

        let mut f = fs::File::create(out_dir.join(&fingerprint)).unwrap();
        f.write(&buf).unwrap();
    }

    Ok(buf)
}

#[derive(Debug)]
pub(crate) enum Object {
    Null,
    Bool(bool),
    Int(i32),
    Number(f32),
    String(String),
    Name(String),
    Array(Vec<Object>),
    Dict(IndexMap<String, Object>),
    Stream(Vec<u8>),
}

pub(crate) fn pdf_object_to(obj: PdfObject, count: u32) -> Result<Object, mupdf::Error> {
    if count == 0 || obj.is_null()? {
        Ok(Object::Null)
    } else if obj.is_bool()? {
        Ok(Object::Bool(obj.as_bool()?))
    } else if obj.is_int()? {
        Ok(Object::Int(obj.as_int()?))
    } else if obj.is_number()? {
        Ok(Object::Number(obj.as_float()?))
    } else  if obj.is_string()? {
        Ok(Object::String(obj.as_string()?.to_string()))
    } else if obj.is_name()? {
        let name = obj.as_name()?;

        Ok(Object::Name(String::from_utf8(name.to_vec()).unwrap()))
    } else if obj.is_dict()? {
        let mut out = IndexMap::new();
        for i in 0..obj.dict_len()? {
            let (key, val) = (obj.get_dict_key(i as i32)?.unwrap(), obj.get_dict_val(i as i32)?.unwrap());

            let (key, val) = (key.as_name().unwrap(), pdf_object_to(val, count - 1)?);
            let key = String::from_utf8(key.to_vec()).unwrap();

            out.insert(key, val);
        }
        Ok(Object::Dict(out))
    } else if obj.is_array()? {
        let mut out = Vec::new();
        for i in 0..obj.len()? {
            let elm = obj.get_array(i as i32)?;
            if elm.is_none() {
                continue;
            }
            out.push(pdf_object_to(elm.unwrap(), count - 1)?);
        }

        Ok(Object::Array(out))

    } else {
        panic!("");
    }
}

pub(crate) fn get_winwidth() -> u16 {
    ioctl_read_bad!(winsize, TIOCGWINSZ, winsize);

    let mut w = winsize { ws_row: 0, ws_col: 0, ws_xpixel: 0, ws_ypixel: 0 };
    let r = unsafe { winsize(1, &mut w) };

    match r {
        Ok(0) => {
            w.ws_xpixel
        },
        _ => {
            200
        }
    }
}

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
