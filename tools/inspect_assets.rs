#[path = "../src/resources.rs"]
mod resources;
use std::{fs, io::Write};
fn main() -> Result<(), String> {
    let root = std::env::args().nth(1).unwrap_or("original".into());
    let out = std::env::args().nth(2).unwrap_or("evidence/assets".into());
    fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    let a = resources::AssetStore::load(root)?;
    for (name, body) in &a.raw {
        if name.ends_with(".TTM") {
            for (tag, b) in resources::chunks(body)? {
                if &tag == b"TT3:" {
                    fs::write(format!("{out}/{name}.bin"), resources::decompress(b)?)
                        .map_err(|e| e.to_string())?;
                }
            }
        }
    }
    for (name, img) in &a.backgrounds {
        let mut f = fs::File::create(format!("{out}/{name}.ppm")).map_err(|e| e.to_string())?;
        write!(f, "P6\n{} {}\n255\n", img.width, img.height).map_err(|e| e.to_string())?;
        for &p in &img.pixels {
            f.write_all(&a.palette[p as usize])
                .map_err(|e| e.to_string())?;
        }
    }
    for (name, frames) in &a.sprites {
        let cellw = frames.iter().map(|f| f.width).max().unwrap_or(1) as usize;
        let cellh = frames.iter().map(|f| f.height).max().unwrap_or(1) as usize;
        let cols = frames.len().min(10);
        let rows = frames.len().div_ceil(cols);
        let (w, h) = (cols * (cellw + 4), rows * (cellh + 4));
        let mut pix = vec![40u8; w * h * 3];
        for (i, img) in frames.iter().enumerate() {
            for y in 0..img.height as usize {
                for x in 0..img.width as usize {
                    let p = img.pixels[y * img.width as usize + x];
                    if p == 0 {
                        continue;
                    }
                    let dst = ((i / cols * (cellh + 4) + y) * w + i % cols * (cellw + 4) + x) * 3;
                    pix[dst..dst + 3].copy_from_slice(&a.palette[p as usize]);
                }
            }
        }
        let mut f = fs::File::create(format!("{out}/{name}.ppm")).map_err(|e| e.to_string())?;
        write!(f, "P6\n{w} {h}\n255\n").map_err(|e| e.to_string())?;
        f.write_all(&pix).map_err(|e| e.to_string())?;
    }
    println!(
        "Decoded {} backgrounds, {} sprite sheets, {} resources",
        a.backgrounds.len(),
        a.sprites.len(),
        a.raw.len()
    );
    Ok(())
}
