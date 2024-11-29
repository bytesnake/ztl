use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct Latex {
    pub preamble: String,
    pub build: String,
}

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct Config {
    pub latex: Latex,
    pub render: String,
    pub toot: Option<String>,
    #[serde(default)]
    pub public: Vec<String>,
}

pub(crate) fn get_config_path() -> PathBuf {
    ".ztl/config".into()
}
