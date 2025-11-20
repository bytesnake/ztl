use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use serde::Deserialize;

use crate::error::Result;

#[derive(Deserialize, Debug, Clone)]
pub struct Latex {
    pub preamble: PathBuf,
    pub build: String,
}

/// Preview notes with defined template and geckodriver
#[derive(Deserialize, Debug, Clone)]
pub struct Preview {
    pub template: String,
    #[serde(default = "default_http_server")]
    pub http_server: String,
    pub geckodriver: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub latex: Latex,
    pub preview: Preview,
    pub toot: Option<String>,
    #[serde(default)]
    pub public: Vec<String>,
    #[serde(default)]
    pub root: PathBuf,
}

impl Config {
    pub fn from_root(root: &Path) -> Result<Config> {
        let config = fs::read_to_string(&root.join(".ztl").join("config"))?;
        let mut config: Config = toml::from_str(&config)?;

        config.public = config.public.into_iter().map(|x| glob::glob(&x).unwrap()).flatten().map(|x| x.unwrap().display().to_string()).collect();
        config.root = root.to_path_buf();

        Ok(config)
    }

    pub fn ztl_root(&self) -> PathBuf {
        self.root.join(".ztl")
    }

    pub fn latex_preamble(&self) -> PathBuf {
        self.root.join(".ztl").join(&self.latex.preamble)
    }

    pub fn empty(path: &Path) -> Result<()> {
        let content = r#"
public = []

[preview]
template = ".ztl/template.html"

[latex]
preamble = ".ztl/preamble.text"
build = "/usr/bin/make4ht -m draft {file}"
"#;

        let mut f = fs::File::create(path)?;
        f.write(content.as_bytes())?;

        Ok(())
    }
}

fn default_http_server() -> String {
    "127.0.0.1:1111".into()
}
