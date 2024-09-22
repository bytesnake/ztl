use clap::Parser;
use std::fs;

mod commands;
mod config;

fn main() {
    let cli = commands::Cli::parse();
    dbg!(&cli.command);

    // parse configuration
    let config = config::get_config_path();
    let config = fs::read_to_string(&config).unwrap();
    let config: config::Config = toml::from_str(&config).unwrap();

    dbg!(&config);



    let markdown_input = r"# Test123

The retention rate in humans is dependent on the interval and modality setting.

Test123 [Blub](google123.com) blaa. ![Bla](fdsa) This can be [As seen][GOmb1989].

<john@example.org>

[Test123][ref]

[1]: Blub123

## Bla

+++
title = 'test'
date = '2024-08-12'
+++
";

    //let opts = pulldown_cmark::Options::all();
    //let parser = pulldown_cmark::utils::TextMergeStream::new(pulldown_cmark::Parser::new_ext(markdown_input, opts))
    //    .for_each(|x| {dbg!(&x); });
}
