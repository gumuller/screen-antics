//! Exhaustive indexed playback plus evenly-spaced true-color review captures.
#[path = "../src/animation.rs"]
mod animation;
#[path = "../src/audio.rs"]
mod audio;
#[path = "../src/hd.rs"]
mod hd;
#[path = "../src/hd2d.rs"]
mod hd2d;
#[path = "../src/presentation.rs"]
mod presentation;
#[path = "../src/remaster.rs"]
mod remaster;
#[path = "../src/resources.rs"]
mod resources;
#[path = "../src/scheduler.rs"]
mod scheduler;
use std::{fs, io::Write, path::Path};
fn main() -> Result<(), String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    // Optional positional arguments: imported pack, output folder, aspect, WIDTHxHEIGHT, hd2d.
    let mut args = std::env::args().skip(1);
    let pack = args.next().map(std::path::PathBuf::from);
    let out = args
        .next()
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| root.join("captures/review"));
    let aspect = remaster::AspectMode::parse(&args.next().unwrap_or("auto".into()))?;
    let size = args.next().unwrap_or("960x540".into());
    let (width, height) = size.split_once('x').ok_or("Expected WIDTHxHEIGHT")?;
    let (width, height) = (
        width.parse::<usize>().map_err(|_| "Invalid width")?,
        height.parse::<usize>().map_err(|_| "Invalid height")?,
    );
    let use_effects = match args.next().as_deref() {
        None => false,
        Some("hd2d") => true,
        Some(x) => return Err(format!("Unknown review effects {x}")),
    };
    let mut post = hd2d::Hd2dEffects::new();
    fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    let assets = resources::AssetStore::load(root.join("original"))?;
    let art = remaster::RemasterAssets::load_with_painted_art(
        pack.as_deref().unwrap_or(&root.join("assets/remastered")),
        &assets,
        pack.is_some(),
    )?;
    if pack.is_some() && art.remade_frames != assets.sprites.values().map(Vec::len).sum::<usize>() {
        return Err(format!(
            "Incomplete review pack: {} replacement frames",
            art.remade_frames
        ));
    }
    let coverage = pack.as_ref().map(|_| art.sprite_coverage());
    let sounds = audio::SoundBank::load(
        &fs::read(root.join("original/SCRANTIC.EXE")).map_err(|e| e.to_string())?,
        pack.as_deref(),
    )?;
    // Every canvas, including source backgrounds not reached by scene browsing,
    // must render exact source samples at 1/4 output size with registered staging.
    for (name, variants) in &art.widescreen {
        for wide in variants {
            let clip = animation::Clip {
                name: name.clone(),
                labels: Default::default(),
                frames: vec![animation::Frame {
                    delay_ticks: 1,
                    commands: vec![animation::Command {
                        opcode: 0xf010,
                        args: vec![],
                        text: name.clone(),
                    }],
                }],
            };
            let mut player = animation::Player::new();
            player.step(&clip, &assets);
            let (w, h) = (wide.width / 4, wide.height / 4);
            let mut renderer = hd::HdPresentation::new(w, h, &assets.palette);
            renderer.set_aspect(wide.aspect);
            renderer.update(
                &player,
                &assets,
                &art,
                true,
                presentation::ColorMode::Classic,
            )?;
            assert_eq!(
                renderer.stage_transform(),
                [1., wide.stage[0] as f64 / 4., wide.stage[1] as f64 / 4.]
            );
            let expected = remaster::load_image(&wide.path)?;
            for y in 0..h {
                for x in 0..w {
                    assert_eq!(
                        renderer.pixels[y * w + x],
                        expected.pixels[(y * 4 + 2) * wide.width + x * 4 + 2],
                        "{name} {} at {x},{y}",
                        wide.aspect.name()
                    );
                }
            }
            let mut f = fs::File::create(out.join(format!(
                "layout-{name}-{}.ppm",
                wide.aspect.name().replace(':', "_")
            )))
            .map_err(|e| e.to_string())?;
            write!(f, "P6\n{w} {h}\n255\n").map_err(|e| e.to_string())?;
            let bytes: Vec<_> = renderer
                .pixels
                .iter()
                .flat_map(|p| [(p >> 16) as u8, (p >> 8) as u8, *p as u8])
                .collect();
            f.write_all(&bytes).map_err(|e| e.to_string())?;
        }
    }
    if pack.is_some() {
        let output = out.join("audio-pcm");
        fs::create_dir_all(&output).map_err(|e| e.to_string())?;
        for n in audio::WAVE_SFX_NUMBERS {
            let pcm = sounds
                .sound(n as u16, audio::AudioMode::Remastered)
                .ok_or("Missing remastered sound")?;
            fs::write(
                output.join(format!("WAVESFX{n:02}.pcm")),
                pcm.iter().flat_map(|s| s.to_le_bytes()).collect::<Vec<_>>(),
            )
            .map_err(|e| e.to_string())?;
        }
    }
    let raw: Vec<_> = assets
        .raw
        .iter()
        .filter(|(n, _)| n.ends_with(".TTM"))
        .map(|(n, b)| animation::Clip::parse(n, b))
        .collect::<Result<_, _>>()?;
    let clips = scheduler::build_scene_clips(&assets, &raw)?;
    let mut log = String::from("scene\tframes\tsprite_draws\tcaptures\tsources\tsound_requests\n");
    for clip in clips {
        let mut player = animation::Player::new();
        let mut classic = animation::Player::new();
        if let Some(coverage) = &coverage {
            player.set_sprite_coverage(coverage.clone());
        }
        let mut renderer = hd::HdPresentation::new(width, height, &assets.palette);
        renderer.set_aspect(aspect);
        let checkpoints: Vec<_> = (0..8)
            .map(|i| 1 + (clip.frames.len() - 1) * i / 7)
            .collect();
        let mut draws = 0;
        let mut captures = 0;
        let mut sound_requests = 0;
        while player.step(&clip, &assets).is_some() {
            if pack.is_some() {
                classic.step(&clip, &assets);
                assert_eq!(
                    player.pixels, classic.pixels,
                    "{} frame {} changed retro pixels",
                    clip.name, player.frame
                );
            }
            draws += player.draws.len();
            for &n in &player.sounds {
                if audio::index_for_sfx(n as u8).is_some() {
                    assert!(sounds.sound(n, audio::AudioMode::Remastered).is_some());
                    sound_requests += 1;
                }
            }
            if checkpoints.contains(&player.frame) {
                renderer.update(
                    &player,
                    &assets,
                    &art,
                    true,
                    presentation::ColorMode::Classic,
                )?;
                let path = out.join(format!("{}-{:04}.ppm", clip.name, player.frame));
                let mut f = fs::File::create(path).map_err(|e| e.to_string())?;
                write!(f, "P6\n{width} {height}\n255\n").map_err(|e| e.to_string())?;
                let final_pixels = if use_effects {
                    post.render(
                        &renderer.pixels,
                        width,
                        height,
                        renderer.stage_transform(),
                        &player,
                        &art,
                        player.frame as f64 / 60.,
                    )
                } else {
                    &renderer.pixels
                };
                let bytes: Vec<_> = final_pixels
                    .iter()
                    .flat_map(|p| [(p >> 16) as u8, (p >> 8) as u8, *p as u8])
                    .collect();
                f.write_all(&bytes).map_err(|e| e.to_string())?;
                captures += 1;
            }
        }
        let row = format!(
            "{}\t{}\t{draws}\t{captures}\t{}\t{sound_requests}\n",
            clip.name,
            player.frame,
            player.sources.len()
        );
        print!("{row}");
        log.push_str(&row);
    }
    let log_path = if pack.is_some() {
        out.join("scene-review.tsv")
    } else {
        root.join("evidence/scene-review.tsv")
    };
    fs::write(log_path, log).map_err(|e| e.to_string())?;
    Ok(())
}
