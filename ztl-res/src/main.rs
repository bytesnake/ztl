use clap::{Parser, Subcommand};
use std::io::{stdin, Read};

mod state;
mod utils;
mod utils_pdf;
mod protocol;
mod error;

#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None)]
pub(crate) struct Cli {
    /// Target width (images are resized to this width)
    #[arg(short, long, default_value_t=utils::get_winwidth())]
    pub width: u16,

    /// Target height (images are cropped exceeding this height)
    #[arg(short, long)]
    pub height: Option<usize>,

    /// Zoom
    #[arg(short, long, default_value_t = 2.0)]
    pub zoom: f32,

    /// From external resource
    #[arg(short, long)]
    pub resource: Option<String>,
}

fn main() -> error::Result<()> {
    let mut cli = Cli::parse();

    // open STDIN
    let mut state = state::new(cli.clone())?;

    let stdin = stdin();
    for line in stdin.lines() {
        let obj: protocol::Message = serde_json::from_str(&line?)?;

        match obj {
            protocol::Message::Render { dest } => {
                state.render(dest);
            },
            protocol::Message::Search { page, text } => {
                state.search(page, text);
            },
            protocol::Message::Switch(source) => {
                cli.resource = match source {
                    protocol::Source::Notes => None,
                    protocol::Source::Resource(x) => Some(x),
                };

                state = state::new(cli.clone())?;
            },
        }
    }

    Ok(())
}
