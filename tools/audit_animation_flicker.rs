//! Find HD changes during identical indexed poses, and capture consecutive actor frames.
#[path = "../src/animation.rs"]
mod animation;
#[path = "../src/hd.rs"]
mod hd;
#[path = "../src/presentation.rs"]
mod presentation;
#[path = "../src/remaster.rs"]
mod remaster;
#[path = "../src/resources.rs"]
mod resources;
#[path = "../src/scheduler.rs"]
mod scheduler;
use std::{fs, io::Write, path::Path};
fn ppm(path: &Path, renderer: &hd::HdPresentation) {
    let mut f = fs::File::create(path).unwrap();
    write!(f, "P6\n{} {}\n255\n", renderer.width, renderer.height).unwrap();
    let bytes: Vec<_> = renderer
        .pixels
        .iter()
        .flat_map(|p| [(p >> 16) as u8, (p >> 8) as u8, *p as u8])
        .collect();
    f.write_all(&bytes).unwrap();
}
fn trace(p: &animation::Player, id: u32, depth: usize) -> String {
    if depth > 5 {
        return format!("#{id} ...");
    }
    match id.checked_sub(1).and_then(|i| p.sources.get(i as usize)) {
        Some(animation::PixelSource::Layer { top, under }) => format!(
            "Layer({}, {})",
            trace(p, *top, depth + 1),
            trace(p, *under, depth + 1)
        ),
        source => format!("#{id} {source:?}"),
    }
}
fn main() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = root.join("captures/flicker-audit");
    fs::create_dir_all(&out).unwrap();
    let assets = resources::AssetStore::load(root.join("original"))?;
    let art = remaster::RemasterAssets::load_with_painted_art(
        &root.join("assets/gemini-remaster"),
        &assets,
        true,
    )?;
    let raw: Vec<_> = assets
        .raw
        .iter()
        .filter(|(n, _)| n.ends_with(".TTM"))
        .map(|(n, b)| animation::Clip::parse(n, b))
        .collect::<Result<_, _>>()?;
    let clips = scheduler::build_scene_clips(&assets, &raw)?;
    let coverage = art.sprite_coverage();
    let mut report = String::from("scene\tframe\tchanged_hd_pixels\tnested_same_sprite_layers\n");
    for clip in clips.iter().filter(|c| {
        args.get(1)
            .map_or(true, |n| c.name.starts_with(&n.to_uppercase()))
    }) {
        let mut p = animation::Player::new();
        p.set_sprite_coverage(coverage.clone());
        let mut renderer = hd::HdPresentation::new(640, 480, &assets.palette);
        renderer.set_aspect(remaster::AspectMode::Original);
        let mut previous_pixels = Vec::new();
        let mut previous_draws = Vec::new();
        let mut previous_origins = Vec::new();
        let mut previous_bg = None;
        let mut previous_hd = Vec::new();
        let mut unstable = 0;
        let mut captures = 0;
        while p.step(clip, &assets).is_some() {
            let identical = p.frame > 1
                && p.pixels == previous_pixels
                && p.draws == previous_draws
                && p.current_background == previous_bg;
            // Render every frame to compare to the preceding frame; small output
            // isolates temporal stability rather than 4K presentation cost.
            renderer.update(&p, &assets, &art, true, presentation::ColorMode::Classic)?;
            let nested=p.sources.iter().filter(|source|match source {
                animation::PixelSource::Layer{top,under}=>top==under || matches!(p.sources.get(*under as usize-1),Some(animation::PixelSource::Layer{top:t,..}) if t==top),_=>false
            }).count();
            if identical && p.origins != previous_origins {
                let changed = renderer
                    .pixels
                    .iter()
                    .zip(&previous_hd)
                    .filter(|(a, b)| a != b)
                    .count();
                if changed > 0 {
                    if args.len() > 1 && captures < 2 {
                        println!(
                            "TRACE {} frame {} commands {:?}",
                            clip.name,
                            p.frame,
                            clip.frames[p.frame - 1].commands
                        );
                        let [scale, ox, oy] = renderer.stage_transform();
                        let mut printed = 0;
                        for (i, (a, b)) in previous_hd.iter().zip(&renderer.pixels).enumerate() {
                            if a == b {
                                continue;
                            }
                            let x = ((i % renderer.width) as f64 + 0.5 - ox) / scale;
                            let y = ((i / renderer.width) as f64 + 0.5 - oy) / scale;
                            if x < 0. || y < 0. || x >= 640. || y >= 480. {
                                continue;
                            }
                            let pos = y as usize * 640 + x as usize;
                            println!(
                                "  {x},{y}: {a:08x}->{b:08x} indexed={} OLD={} NEW={}",
                                p.pixels[pos],
                                trace(&p, previous_origins[pos], 0),
                                trace(&p, p.origins[pos], 0)
                            );
                            printed += 1;
                            if printed >= 3 {
                                break;
                            }
                        }
                    }
                    unstable += 1;
                    report.push_str(&format!(
                        "{}\t{}\t{changed}\t{nested}\n",
                        clip.name, p.frame
                    ));
                    if captures < 4 {
                        let current = renderer.pixels.clone();
                        renderer.pixels.copy_from_slice(&previous_hd);
                        ppm(
                            &out.join(format!("{}-{:04}-before.ppm", clip.name, p.frame)),
                            &renderer,
                        );
                        renderer.pixels = current;
                        ppm(
                            &out.join(format!("{}-{:04}-after.ppm", clip.name, p.frame)),
                            &renderer,
                        );
                        captures += 1;
                    }
                }
            }
            if args.len() > 2 && p.frame <= args[2].parse::<usize>().unwrap_or(0) {
                ppm(
                    &out.join(format!("{}-{:04}.ppm", clip.name, p.frame)),
                    &renderer,
                );
            }
            previous_pixels.clone_from(&p.pixels);
            previous_draws.clone_from(&p.draws);
            previous_origins.clone_from(&p.origins);
            previous_bg = p.current_background.clone();
            previous_hd.clone_from(&renderer.pixels);
        }
        println!("{}: {unstable} unstable identical poses", clip.name);
    }
    fs::write(out.join("identical-poses.tsv"), report).unwrap();
    Ok(())
}
