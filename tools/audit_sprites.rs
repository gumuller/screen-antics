//! Export original indexed frames as straight RGBA for the independent visual audit.
#[path = "../src/resources.rs"]
mod resources;
use std::{fs, io::Write, path::Path};
fn main() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    let original = args.get(1).map(String::as_str).unwrap_or("original");
    let output = args
        .get(2)
        .map(String::as_str)
        .unwrap_or("evidence/sprite-review/original");
    let a = resources::AssetStore::load(original)?;
    let root = Path::new(output);
    fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let mut m = fs::File::create(root.join("frames.tsv")).map_err(|e| e.to_string())?;
    writeln!(m, "sheet\tindex\twidth\theight\tfile").map_err(|e| e.to_string())?;
    for (name, frames) in &a.sprites {
        fs::create_dir_all(root.join(name)).map_err(|e| e.to_string())?;
        for (i, img) in frames.iter().enumerate() {
            let namefile = format!("{name}/{i:03}.rgba");
            let mut b = Vec::new();
            b.extend_from_slice(b"SAHD");
            b.extend_from_slice(&img.width.to_le_bytes());
            b.extend_from_slice(&img.height.to_le_bytes());
            for &p in &img.pixels {
                b.extend_from_slice(&a.palette[p as usize]);
                b.push(if p == 0 { 0 } else { 255 });
            }
            fs::write(root.join(&namefile), b).map_err(|e| e.to_string())?;
            writeln!(m, "{name}\t{i}\t{}\t{}\t{namefile}", img.width, img.height)
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
