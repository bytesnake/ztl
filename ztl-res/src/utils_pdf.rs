use mupdf::{Matrix, Colorspace, ImageFormat, DisplayList, Device, Pixmap, IRect, Rect, pdf::PdfObject, TextPageFlags, Page, Document, pdf::PdfDocument};
use ztl_base::error::Result;
use std::io::Write;
use indexmap::IndexMap;
use itertools::Itertools;

use crate::Cli;
use crate::protocol::{Destination, DestinationKind, Rect as PRect};

pub(crate) fn render_rect(page: Page, cli: &Cli, x0: i32, y0: i32) -> Result<()> {
    let mut buf = Vec::new();
    let zoom = cli.zoom;
    let bounds = page.bounds().unwrap();

    let rect = IRect { x0: x0, y0: y0, x1: bounds.x1 as i32, y1: bounds.y1 as i32 };
    //let mut rect_target = IRect { x0: (rect.x0 as f32 * zoom) as i32, y0: (rect.y0 as f32 * zoom) as i32, x1: (rect.x1 as f32 * zoom) as i32, y1: (rect.y1 as f32 * zoom) as i32 };

    let mut rect_target = IRect { x0: (rect.x0 as f32 * zoom) as i32, y0: (rect.y0 as f32 * zoom) as i32, x1: (rect.x0 as f32 * zoom) as i32 + cli.width as i32, y1: (rect.y0 as f32 * zoom) as i32 + cli.height.unwrap() as i32};
    //rect_target.x1 = rect_target.x0 + i32::min(rect_target.x1 - rect_target.x0, cli.width as i32);
    //if let Some(height) = cli.height {
    //    rect_target.y1 = rect_target.y0 + i32::min(rect_target.y1 - rect_target.y0, height as i32);
    //}

    // render page to displaylist
    let mut bounds = page.bounds().unwrap();
    bounds.x0 *= zoom;
    bounds.y0 *= zoom;
    bounds.x1 *= zoom;
    bounds.y1 *= zoom;

    dbg!(&page.bounds());
    let displaylist = DisplayList::new(bounds.clone()).unwrap();
    page.run(&Device::from_display_list(&displaylist).unwrap(), &Matrix::IDENTITY).unwrap();

    // construct pixmap with white color
    let mut pxmap = Pixmap::new_with_rect(&Colorspace::device_rgb(), rect_target.clone(), false).unwrap();

    pxmap.clear_with(255).unwrap();

    let dev = Device::from_pixmap_with_clip(&pxmap, rect_target).unwrap();
    //let dev = Device::from_pixmap(&pxmap).unwrap();

    displaylist.run(&dev, &Matrix::new_scale(zoom, zoom), bounds);
    pxmap.write_to(&mut buf, ImageFormat::PNG).unwrap();

    {
        let mut f = std::fs::File::create("/tmp/out.png").unwrap();
        f.write_all(&buf).unwrap();
    }

    use magick_rust::{MagickError, MagickWand, magick_wand_genesis};

    magick_wand_genesis();
    let wand = MagickWand::new();
    wand.read_image_blob(&buf).unwrap();
    //wand.set_image_resolution(30., 30.).unwrap();
    //wand.crop_image((cli.width as f32 * zoom) as usize, (cli.height.unwrap() as f32 * zoom) as usize - 2, 0, 0).unwrap();
    //wand.crop_image(region[2], region[3], region[0] as isize, region[1] as isize).unwrap();
    let out = wand.write_image_blob("sixel").unwrap();

    let mut f = std::fs::File::create("/tmp/out.sixel").unwrap();
    f.write_all(&out).unwrap();

    std::io::stdout()
        .write_all(&out).unwrap();
    println!("");

    std::io::stdout().flush().unwrap();

    Ok(())
}

pub(crate) fn search(pdf: &PdfDocument, pattern: &str) -> Vec<Destination> {
    let mut dests = pdf.pages().unwrap().into_iter().enumerate()
        .map(|(page, x)| x.unwrap().search(pattern, 10).unwrap().into_iter().map(|x| (page, x)).collect::<Vec<_>>())
        .flatten()
        .map(|(page, x)| Destination {
            target: pattern.to_string(),
            position: Some((page as u32, DestinationKind::Bound(PRect::new(x.ul.x, x.ul.y, x.lr.x, x.lr.y)))),
            context: Some((String::new(), PRect::new(0.0, 0.0, 0.0, 0.0))),
        })
        .collect::<Vec<_>>();

    dests.sort_by_key(|x| x.position.as_ref().unwrap().0);

    dests.into_iter().chunk_by(|x| x.position.as_ref().unwrap().0).into_iter()
        .map(|(page, group)| {
            let mut group = group.collect::<Vec<_>>();
            let mut page = pdf.load_page(page as i32).unwrap();
            // find region in possible blocks
            let text_page = page.to_text_page(TextPageFlags::empty()).unwrap();
            for block in text_page.blocks() {
                let mut text = None;
                for idx in 0..group.len() {
                    let bounds = match &group[idx].position.as_ref().unwrap().1 {
                        DestinationKind::Bound(r) => r,
                        _ => unreachable!(),
                    };

                    if block.bounds().contains(bounds.x0 as f32, bounds.y0 as f32) {
                        if text.is_none() {
                            text = Some(block.lines().map(|x| x.chars().filter_map(|x| x.char()).collect::<String>()).collect::<Vec<_>>().join("\n"));
                        }

                        group[idx].context = Some((text.clone().unwrap(), PRect::from_mupdf(block.bounds())));
                    }

                }
            }

            group
        })
        .flatten()
        .collect::<Vec<_>>()
}

pub(crate) fn named_destinations(pdf: &PdfDocument) -> Vec<Destination> {
    let dests = PdfObject::new_name("Dests").unwrap();
    let dests = pdf.load_name_tree(dests).unwrap();

    // generate map of indirect indices to page numbers
    let mut map = IndexMap::new();
    for i in 0.. {
        let page = match pdf.find_page(i as i32) {
            Ok(x) => x,
            _ => break,
        };

        map.insert(page.as_indirect().unwrap(), i);
    }

    let mut out = Vec::new();

    if !dests.is_dict().unwrap() {
        return Vec::new();
    }

    let len = dests.dict_len().unwrap();
    for i in 0..len {
        let (key, val) = (dests.get_dict_key(i as i32).unwrap().unwrap(), dests.get_dict_val(i as i32).unwrap().unwrap());
        // check that dictionary contains target directly
        if let Some(dest) = parse_destination(&key, &val, &map) {
            out.push(dest);
            continue;
        }

        // otherwise check if we have to unpack first
        let dict = val.get_dict("D").unwrap().unwrap().resolve().unwrap().unwrap();

        if let Some(dest) = parse_destination(&key, &dict, &map) {
            out.push(dest);
            continue;
        }
    }

    out
}

fn parse_destination(key: &PdfObject, val: &PdfObject, map: &IndexMap<i32, u32>) -> Option<Destination> {
    // should contain an array to destinations
    if !val.is_array().unwrap() || !key.is_name().unwrap() {
        return None;
    }
    let key = key.as_name().map(|x| String::from_utf8(x.to_vec()).unwrap()).unwrap();

    let mut page = 0;
    let mut kind = DestinationKind::Fit;

    for i in 0..val.len().unwrap() {
        let elm = match val.get_array(i as i32).unwrap() {
            Some(x) => x,
            _ => continue,
        };

        match i {
            0 if elm.is_indirect().unwrap() => {
                //dbg!(&elm.as_indirect().unwrap());
                page = *map.get(&elm.as_indirect().unwrap()).unwrap()
            },
            1 if elm.is_name().unwrap() => {
                let name = elm.as_name().map(|x| String::from_utf8(x.to_vec()).unwrap()).unwrap();
                match name.as_str() {
                    "Fit" => { kind = DestinationKind::Fit; },
                    "XYZ" => { kind = DestinationKind::XYZ { x: 0., y: 0., z: None } },
                    _ => panic!(""),
                }
            },
            2 if elm.is_number().unwrap() => {
                let elm = elm.as_float().unwrap();
                match &mut kind {
                    DestinationKind::XYZ { x, .. } => *x = elm,
                    _ => return None,
                }
            },
            3 if elm.is_number().unwrap() => {
                let elm = elm.as_float().unwrap();
                match &mut kind {
                    DestinationKind::XYZ { y, .. } => *y = elm,
                    _ => return None,
                }
            },
            4 if elm.is_null().unwrap() => {},
            4 if elm.is_number().unwrap() => {
                let elm = elm.as_float().unwrap();
                match &mut kind {
                    DestinationKind::XYZ { z, .. } => *z = Some(elm),
                    _ => return None,
                }
            },
            _ => return None,
        }
    }

    Some(Destination {
        target: key, position: Some((page, kind)), context: None 
    })

}
