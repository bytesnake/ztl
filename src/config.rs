use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize, Debug)]
pub(crate) struct Config {
    bibtex: String,
}

pub(crate) fn get_config_path() -> PathBuf {
    ".ztl/config".into()
}
