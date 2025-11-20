use std::path::PathBuf;
use std::fs::File;
use std::io::Write;

use mupdf::{pdf::PdfDocument, Rect};
use thirtyfour::prelude::*;
use tokio::runtime::Runtime;
use magick_rust::{MagickError, MagickWand, magick_wand_genesis, PixelWand};

use ztl_base::Note;
use crate::{error::{Result, Error}, utils, Cli, protocol::{Destination, DestinationKind}};

pub(crate) trait State {
    fn render(&self, view: Destination) -> String;

    fn search(&self, page: Option<u32>, text: Option<String>) {
    }
}

pub(crate) fn new(cli: Cli) -> Result<Box<dyn State>> {
    match cli.resource {
        None => {
            let cwd = std::env::current_dir()?;

            // search upwards until ZTL folder is found
            utils::search_upwards(&cwd, |p| p.join(".ztl").exists())
                .ok_or(Error::ZtlNotFound(cwd.clone()))
                .and_then(|x| {
                    let rt = Runtime::new().unwrap();

                    let res = rt.block_on(async {
                        Notes::setup_selenium(&cli).await
                    });

                    return Ok(Box::new(Notes(x.to_path_buf().join(".ztl"), res?, rt, cli)) as Box<dyn State>);
                })
        },
        Some(ref resource) => {
            let buf = utils::fetch_and_cache(&resource)?;

            if resource.ends_with(".pdf") {
                PdfDocument::from_bytes(&buf)
                    .map(|pdf| Box::new(PdfReader(pdf, cli.clone())) as Box<dyn State>)
                    .map_err(Error::from)
            } else {
                todo!();
            }
        },
    }
}

pub(crate) struct Notes(PathBuf, WebDriver, Runtime, Cli);

impl State for Notes {
    fn render(&self, view: Destination) -> String {
        let path = self.2.block_on(async {
            self.snapshot(view).await
        });

        magick_wand_genesis();

        let wand = MagickWand::new();
        wand.read_image(&path.unwrap().to_str().unwrap()).unwrap();
        wand.trim_image(0.0).unwrap();

        let mut bw = PixelWand::new();
        bw.set_color("white").unwrap();

        wand.border_image(
            &bw,
            10,
            10,
            magick_rust::bindings::CompositeOperator::Over,
        ).unwrap();
        wand.set_image_resolution(30., 30.).unwrap();
        //wand.fit(398, 550);
        //wand.crop_image(region[2], region[3], region[0] as isize, region[1] as isize).unwrap();
        let out = wand.write_image_blob("sixel").unwrap();

        std::io::stdout()
            .write_all(&out).unwrap();

        // generate HTML and compare to cached version
        String::new()
    }
}

impl Notes {
    async fn setup_selenium(cli: &Cli) -> Result<WebDriver> {
        let mut caps = DesiredCapabilities::firefox();
        
        caps.set_headless().unwrap();
        caps.add_arg(&format!("-width={}", cli.width)).unwrap();
        caps.add_arg("-height=2000").unwrap();

        let server_url = "http://localhost:4444";

        WebDriver::new(server_url, caps).await
            .map_err(Error::from)
    }

    async fn snapshot(&self, view: Destination) -> Result<PathBuf> {
        let note = Note::from_path(&self.0.join("notes").join(&view.target)).unwrap();
        let html = note.render_html(".ztl/templates/*");

        let html_cache = self.0.join("cache").join(&view.target).with_extension("html");
        let mut f = File::create(&html_cache).unwrap();
        f.write(&html.as_bytes()).unwrap();

        let url = format!("file://{}", html_cache.to_str().unwrap());
        self.1.goto(&url).await?;
        self.1.execute(&format!("document.body.style.zoom={}", self.3.zoom), Vec::new()).await?;
        self.1.screenshot(&html_cache.with_extension("png")).await?;

        Ok(html_cache.with_extension("png"))
    }
}

pub(crate) struct PdfReader(PdfDocument, Cli);

impl State for PdfReader {
    fn render(&self, view: Destination) -> String {
        let (page, kind) = view.position.unwrap();
        let page = self.0.load_page(page as i32).unwrap();
        let bounds = page.bounds().unwrap();

        let (x0, y0) = match view.context {
            Some(ctx) => {
                (ctx.1.x0 as i32, ctx.1.y0 as i32)
            },
            None =>
                match kind {
                    DestinationKind::XYZ { x, y, .. } => (x as i32, bounds.y1 as i32 - y as i32),
                    _ => (0, 0)
                }
        };

        // load page and render rect
        crate::utils_pdf::render_rect(page, &self.1, x0, y0).unwrap();

        String::new()
    }

    fn search(&self, page: Option<u32>, text: Option<String>) {
        let res = match text {
            None => crate::utils_pdf::named_destinations(&self.0),
            Some(pattern) => crate::utils_pdf::search(&self.0, &pattern),
        };

        println!("{}", serde_json::to_string(&res).unwrap());
    }
}
