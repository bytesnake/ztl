use sha2::Digest;

pub fn line_col_to_byte_offset(text: &str, line: usize, column: usize) -> Option<usize> {
    let mut byte_offset = 0;
    let mut current_line = 1;
    let mut current_col = 1;

    for c in text.chars() {
        if current_line == line && current_col == column {
            return Some(byte_offset);
        }

        if c == '\n' {
            // of the column exceeds, return end of line
            if current_line == line {
                return Some(byte_offset);
            }

            current_line += 1;
            current_col = 1;
        } else {
            current_col += 1;
        }

        byte_offset += c.len_utf8();
    }

    // Handle the case where the line/column is at the end of the text
    if current_line == line && current_col == column {
        Some(byte_offset)
    } else {
        None
    }
}

//pub(crate) fn render_html(_config: &Config, content: &str, key: &str) {
//    let mut f = std::fs::File::create(".ztl/cache/out.html").unwrap();
//    f.write(r#"
//<html>
//<head><style>
//math{font-size:17px;}
//math[display="block"] {
//padding: 10px 0 10px 0;
//}
//mtable{padding: 5px 0 0 0;}
//mtd{padding: 0 5px 0 0 !important;}
//mspace{margin-left:0.17em;}
//.head {
//display: block;
//margin-bottom: 5px;
//}
//dl {
//  display: grid;
//  grid-template-columns: max-content auto;
//}
//
//dt {
//  grid-column-start: 1;
//}
//
//dd {
//  grid-column-start: 2;
//}
//
//div[class^='columns-']{
//    display: flex;
//    justify-content: space-between;
//}
//
//figcaption {
//    border: 1px black solid;
//    border-width: 1 0 1 0;
//    padding-bottom: 1px;
//}
//
//figcaption .id {
//    font-weight: bold;
//}
//
//.cmbx-10 {
//    font-weight: bold;
//}
//
//.cmti-10 {
//    font-style: italic;
//}
//
//.comment {
//    color: lightseagreen;
//}
//
//figure {
//    margin-left: 0;
//    margin-right: 0;
//}
//
//.small-caps {
//    font-variant: small-caps;
//}
//
//</style></head>
//<body>"#.as_bytes()).unwrap();
//
//    f.write(content.as_bytes()).unwrap();
//    f.write(b"</body></html>").unwrap();
//    f.flush().unwrap();
//
//    let p = std::env::current_dir().unwrap().join(".ztl").join("cache");
//    let p = p.into_os_string().into_string().unwrap();
//    let mut out = UnixStream::connect("/tmp/socket_zettel").unwrap();
//    out.write_all(format!("170,{},{}\n", key, p).as_bytes()).unwrap();
//    out.flush().unwrap();
//    let mut buf = Vec::new();
//    let _ = out.read(&mut buf);
//
//    //let out = Command::new("bash").current_dir(".ztl/cache/").arg("-C").arg(&config.render).arg("out.html").arg(key).spawn().unwrap().wait().unwrap(); 
//}

//pub(crate) fn show_note(_config: &Config, key: &str) {
//    let win_width = get_winwidth();
//
//    print!("{esc}c", esc = 27 as char);
//    let p = Path::new(".ztl/cache").join(key).with_extension("sixel");
//    if win_width > 1200 {
//        let mut fs = std::fs::File::open(p).unwrap();
//        let mut buf = Vec::new();
//        fs.read_to_end(&mut buf).unwrap();
//
//        let _ = std::io::stdout().write_all(&buf);
//        return;
//    }
//
//    let p2 = Path::new(".ztl/cache").join(key).with_extension("sixel2");
//    let encoder = Encoder::new().unwrap();
//    encoder.set_output(&p2).unwrap();
//    encoder.set_width(SizeSpecification::Pixel(win_width as u64)).unwrap();
//    encoder.set_diffusion(DiffusionMethod::Atkinson).unwrap();
//    encoder.set_quality(Quality::High).unwrap();
//    encoder.set_resampling(ResampleMethod::Bicubic).unwrap();
//    encoder.set_num_colors(255).unwrap();
//    encoder.set_color_select( ColorSelectionMethod::Histogram).unwrap();
//    encoder.encode_file(&p).unwrap();
//
//
//    let mut fs = std::fs::File::open(Path::new(".ztl/cache").join(key).with_extension("sixel2")).unwrap();
//    let mut buf = Vec::new();
//    fs.read_to_end(&mut buf).unwrap();
//
//    let _ = std::io::stdout().write_all(&buf);
//}

pub fn hash(content: &str) -> String {
    let mut sha256 = sha2::Sha256::new();
    sha256.update(content);
    format!("{:X}", sha256.finalize())
}

//fn get_winwidth() -> u16 {
//    ioctl_read_bad!(winsize, TIOCGWINSZ, winsize);
//
//    let mut w = winsize { ws_row: 0, ws_col: 0, ws_xpixel: 0, ws_ypixel: 0 };
//    let r = unsafe { winsize(1, &mut w) };
//
//    match r {
//        Ok(0) => {
//            w.ws_xpixel
//        },
//        _ => {
//            panic!("Could not get window width!");
//        }
//    }
//}
