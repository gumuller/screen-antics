//! Technical export of registration metadata and native-resolution RGBA originals.
#[path = "../src/resources.rs"]
mod resources;
use std::{fs, io::Write, path::Path};
fn main() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    let original = args.get(1).map(String::as_str).unwrap_or("original");
    let destination = args
        .get(2)
        .map(String::as_str)
        .unwrap_or("evidence/sprite-registration");
    let assets = resources::AssetStore::load(original)?;
    let root = Path::new(destination);
    fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let mut manifest = fs::File::create(root.join("frames.tsv")).map_err(|e| e.to_string())?;
    writeln!(
        manifest,
        "sheet\tindex\twidth\theight\tleft\ttop\tright\tbottom"
    )
    .map_err(|e| e.to_string())?;
    for (name, frames) in &assets.sprites {
        for (i, image) in frames.iter().enumerate() {
            let (w, h) = (image.width as usize, image.height as usize);
            let (mut l, mut t, mut r, mut b) = (w, h, 0, 0);
            for y in 0..h {
                for x in 0..w {
                    if image.pixels[y * w + x] != 0 {
                        l = l.min(x);
                        t = t.min(y);
                        r = r.max(x + 1);
                        b = b.max(y + 1);
                    }
                }
            }
            writeln!(manifest, "{name}\t{i}\t{w}\t{h}\t{l}\t{t}\t{r}\t{b}")
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
