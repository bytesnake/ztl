use std::io::Write;
use clap::Parser;
use std::fs;
use std::collections::HashMap;
use glob::glob;
use sha2::Digest;

mod commands;
mod config;
mod markdown;

fn main() {
    let cli = commands::Cli::parse();

    // parse configuration
    let config = config::get_config_path();
    let config = fs::read_to_string(&config).unwrap();
    let config: config::Config = toml::from_str(&config).unwrap();

    match cli.command {
        Some(commands::Commands::Build) => build(),
        Some(commands::Commands::Publish) => publish(&config),
        None => analyze(),
        _ => panic!("Command {:?} is not supported", cli.command)
    }
}

fn build() {
    println!("Rebuilding notes from scratch ..");

    // create single bibtex file string and parse
    let bibs = glob("**/*.bib").unwrap()
        .filter_map(|x| x.ok())
        .map(|x| fs::read_to_string(&x).unwrap())
        .collect::<Vec<_>>()
        .join("\n");

    let bibs = biblatex::Bibliography::parse(&bibs).unwrap();

    //dbg!(&bibs);

    let arena = comrak::Arena::new();
    let (notes, spans): (Vec<Vec<markdown::Note>>, Vec<(String, markdown::Spans)>) = glob("**/*.md").unwrap()
        .filter_map(|x| x.ok())
        .map(|x| {
            let content = fs::read_to_string(&x).unwrap();

            let notes = markdown::analyze(&arena, &content, &x);

            let spans = notes.iter().map(|x| {
                (x.start_line(), x.outgoing_spans())
            }).collect();

            (notes, (x.display().to_string(), spans))
        }).unzip();

    let notes = notes.into_iter()
        .flatten()
        .collect::<Vec<_>>();

    //let texs = glob("**/*.tex").unwrap().collect::<Vec<_>>();

    // write results to cache and toml files
    let cache_path = config::get_config_path()
        .parent().unwrap().join("cache");

    for note in notes {
        let _ = fs::create_dir(&cache_path);

        let res = toml::to_string(&note).unwrap();
        let mut f = fs::File::create(cache_path.join(&note.id)).unwrap();
        f.write(&res.into_bytes()).unwrap();
    }

    for (file, spans) in spans {
        if spans.len() == 0 {
            continue;
        }

        dbg!(&spans);
        let res = toml::to_string(&spans).unwrap();

        // hash file name
        let mut sha256 = sha2::Sha256::new();
        sha256.update(&file);
        let file = format!("{:X}", sha256.finalize());

        let mut f = fs::File::create(cache_path.join(&file)).unwrap();
        f.write(&res.into_bytes()).unwrap();
    }

}

fn analyze() {
    let (nnotes, nlinks) = markdown::Note::all().into_iter()
        .fold((0, 0), |a,b| (a.0 + 1, a.1 + b.outgoing.len()));

    println!("Found {} notes with {} outgoing links", nnotes, nlinks);
}

fn publish(config: &config::Config) {
    let published_path = config::get_config_path()
        .parent().unwrap().join("published");

    use std::collections::HashMap;
    let mut hash: HashMap<String, (String, String)> = fs::read_to_string(&published_path)
        .map(|x| toml::from_str(&x).unwrap())
        .unwrap_or(HashMap::new());

    for note in markdown::Note::all() {
        match hash.get(&note.id) {
            Some(x) => {
                if note.hash() == x.0 {
                    return;
                }

                println!("Changed {}", note.id);
            },
            None => {
                println!("Publish {}", note.id);
            }
        }
    }
}
