use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("invalid message")]
    InvalidMessage(#[from] serde_json::Error),
    #[error("invalid IO")]
    InvalidIO(#[from] std::io::Error),
    #[error("ZTL folder `.ztl` not found in {0}")]
    ZtlNotFound(PathBuf),
    #[error("operation with mupdf failed")]
    MuPdf(#[from] mupdf::Error),
    #[error("webdriver failed")]
    Webdriver(#[from] thirtyfour::error::WebDriverError),
}
