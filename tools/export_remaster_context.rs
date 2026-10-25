//! Decode source backgrounds/audio and record the current port's complete scene timelines.
//! Export only: never changes artwork, original resources, or runtime configuration.
#[path = "../src/animation.rs"]
mod animation;
#[path = "../src/audio.rs"]
mod audio;
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
use std::{
    collections::BTreeSet,
    fs,
    io::{BufWriter, Write},
    path::Path,
    process::{Command, Stdio},
};

fn quoted(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c < ' ' => out.push_str(&format!("\\u{:04x}", c as u32)),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}
fn commands(commands: &[animation::Command]) -> String {
    format!(
        "[{}]",
        commands
            .iter()
            .map(|c| format!(
                "{{\"opcode\":\"0x{:04x}\",\"args\":{:?},\"text\":{}}}",
                c.opcode,
                c.args,
                quoted(&c.text)
            ))
            .collect::<Vec<_>>()
            .join(",")
    )
}
fn draw(d: &animation::Draw, order: usize) -> String {
    format!("{{\"order\":{order},\"sheet\":{},\"frame\":{},\"x\":{},\"y\":{},\"width\":{},\"height\":{},\"flip_x\":{},\"flip_y\":{},\"clip_xyxy_exclusive\":{:?}}}",
        quoted(&d.name),d.frame,d.x,d.y,d.width,d.height,d.flip_x,d.flip_y,d.clip)
}
fn rgb(pixels: &[u32]) -> Vec<u8> {
    pixels
        .iter()
        .flat_map(|p| [(p >> 16) as u8, (p >> 8) as u8, *p as u8])
        .collect()
}
fn ppm(path: &Path, width: usize, height: usize, pixels: &[u8]) -> std::io::Result<()> {
    let mut f = BufWriter::new(fs::File::create(path)?);
    write!(f, "P6\n{width} {height}\n255\n")?;
    f.write_all(pixels)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err(
            "Usage: export-remaster-context ORIGINAL_DIR IMPORTED_PACK EMPTY_OUTPUT".into(),
        );
    }
    let original = Path::new(&args[1]);
    let pack = Path::new(&args[2]);
    let out = Path::new(&args[3]);
    if out.exists() && out.read_dir()?.next().is_some() {
        return Err("Output must be empty".into());
    }
    for part in [
        "backgrounds",
        "audio",
        "timelines",
        "stills",
        "videos",
        "raw-scripts",
    ] {
        fs::create_dir_all(out.join(part))?;
    }
    let assets = resources::AssetStore::load(original)?;
    let art = remaster::RemasterAssets::load_with_painted_art(pack, &assets, true)?;
    if art.remade_frames != assets.sprites.values().map(Vec::len).sum::<usize>() {
        return Err("Expected the complete imported sprite pack".into());
    }
    let coverage = art.sprite_coverage();
    fs::write(out.join("palette.json"), format!("{:?}\n", assets.palette))?;
    let mut backgrounds = Vec::new();
    for (name, im) in &assets.backgrounds {
        ppm(
            &out.join("backgrounds").join(format!("{name}.ppm")),
            im.width as usize,
            im.height as usize,
            &im.pixels
                .iter()
                .flat_map(|&i| assets.palette[i as usize])
                .collect::<Vec<_>>(),
        )?;
        backgrounds.push(format!(
            "{{\"name\":{},\"width\":{},\"height\":{}}}",
            quoted(name),
            im.width,
            im.height
        ));
    }
    fs::write(
        out.join("backgrounds.json"),
        format!("[{}]\n", backgrounds.join(",")),
    )?;
    let sounds = audio::waves(&fs::read(original.join("SCRANTIC.EXE"))?)?;
    let mut audio_meta = Vec::new();
    for (i, pcm) in sounds.iter().enumerate() {
        let sfx = audio::WAVE_SFX_NUMBERS[i];
        let file = format!("WAVESFX{sfx:02}.wav");
        let n = pcm.len() as u32;
        let pad = n % 2;
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36 + n + pad).to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&(audio::SAMPLE_RATE as u32).to_le_bytes());
        wav.extend_from_slice(&(audio::SAMPLE_RATE as u32).to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&8u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&n.to_le_bytes());
        wav.extend_from_slice(pcm);
        if pad != 0 {
            wav.push(0);
        }
        fs::write(out.join("audio").join(&file), wav)?;
        audio_meta.push(format!("{{\"file\":{},\"resource_id\":{},\"sfx_number\":{},\"sample_rate\":{},\"channels\":1,\"bits\":8,\"samples\":{},\"duration_seconds\":{:.9}}}",
            quoted(&file),i+1,sfx,audio::SAMPLE_RATE,n,n as f64/audio::SAMPLE_RATE as f64));
    }
    fs::write(
        out.join("audio.json"),
        format!("[{}]\n", audio_meta.join(",")),
    )?;
    let raw: Vec<_> = assets
        .raw
        .iter()
        .filter(|(n, _)| n.ends_with(".TTM"))
        .map(|(n, b)| animation::Clip::parse(n, b))
        .collect::<Result<_, _>>()?;
    for clip in &raw {
        let mut f = BufWriter::new(fs::File::create(
            out.join("raw-scripts").join(format!("{}.jsonl", clip.name)),
        )?);
        for (index, frame) in clip.frames.iter().enumerate() {
            writeln!(
                f,
                "{{\"source_frame\":{index},\"delay_ticks\":{},\"commands\":{}}}",
                frame.delay_ticks,
                commands(&frame.commands)
            )?;
        }
    }
    let clips = scheduler::build_scene_clips(&assets, &raw)?;
    let mut summaries = Vec::new();
    for clip in &clips {
        let stem = clip.name.trim_end_matches(".TTM");
        let mut timeline = BufWriter::new(fs::File::create(
            out.join("timelines").join(format!("{stem}.jsonl")),
        )?);
        let mut player = animation::Player::new();
        player.set_sprite_coverage(coverage.clone());
        let mut renderer = hd::HdPresentation::new(640, 360, &assets.palette);
        let mut still_renderer = hd::HdPresentation::new(960, 540, &assets.palette);
        let checkpoints: BTreeSet<_> = (0..8)
            .map(|i| 1 + (clip.frames.len() - 1) * i / 7)
            .collect();
        let mut video = Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-nostdin",
                "-f",
                "rawvideo",
                "-pixel_format",
                "rgb24",
                "-video_size",
                "640x360",
                "-framerate",
                "20",
                "-i",
                "pipe:0",
                "-an",
                "-c:v",
                "libx264",
                "-preset",
                "veryfast",
                "-crf",
                "21",
                "-pix_fmt",
                "yuv420p",
                "-movflags",
                "+faststart",
                "-threads",
                "2",
                "-n",
            ])
            .arg(out.join("videos").join(format!("{stem}.mp4")))
            .stdin(Stdio::piped())
            .spawn()?;
        let mut pipe = BufWriter::new(video.stdin.take().ok_or("Missing video input")?);
        let mut ticks = 0u64;
        let mut next_preview = 0u64;
        let mut video_frames = 0;
        let mut used_backgrounds = BTreeSet::new();
        let mut used_sprites = BTreeSet::new();
        let mut shots = Vec::new();
        while let Some(delay) = player.step(clip, &assets) {
            let bg = player.current_background.as_deref();
            if let Some(n) = bg {
                used_backgrounds.insert(n.to_owned());
            }
            for d in &player.draws {
                used_sprites.insert(d.name.clone());
            }
            let draws = player
                .draws
                .iter()
                .enumerate()
                .map(|(i, d)| draw(d, i))
                .collect::<Vec<_>>()
                .join(",");
            let transition = player
                .transition
                .map(|t| {
                    format!(
                        "{{\"kind\":{},\"rect_xywh\":{:?},\"duration_ms\":{}}}",
                        quoted(&format!("{:?}", t.kind)),
                        t.rect,
                        t.duration_ms
                    )
                })
                .unwrap_or("null".into());
            writeln!(timeline,"{{\"frame\":{},\"tick\":{ticks},\"time_seconds\":{:.9},\"delay_ticks\":{delay},\"logical_size\":[{},{}],\"background\":{},\"draws\":[{}],\"sfx_requests\":{:?},\"transition\":{},\"commands\":{}}}",
                player.frame,ticks as f64/60.,player.width,player.height,bg.map(quoted).unwrap_or("null".into()),draws,player.sounds,transition,commands(&clip.frames[player.frame-1].commands))?;
            if next_preview < ticks + delay as u64 {
                renderer.update(
                    &player,
                    &assets,
                    &art,
                    true,
                    presentation::ColorMode::Classic,
                )?;
                let bytes = rgb(&renderer.pixels);
                while next_preview < ticks + delay as u64 {
                    pipe.write_all(&bytes)?;
                    video_frames += 1;
                    next_preview += 3;
                }
            }
            if checkpoints.contains(&player.frame) {
                still_renderer.update(
                    &player,
                    &assets,
                    &art,
                    true,
                    presentation::ColorMode::Classic,
                )?;
                let file = format!("{stem}-{:04}.ppm", player.frame);
                ppm(
                    &out.join("stills").join(&file),
                    960,
                    540,
                    &rgb(&still_renderer.pixels),
                )?;
                shots.push(format!("{{\"file\":{},\"frame\":{},\"tick\":{ticks},\"background\":{},\"logical_size\":[{},{}]}}",quoted(&file),player.frame,bg.map(quoted).unwrap_or("null".into()),player.width,player.height));
            }
            ticks += delay as u64;
        }
        pipe.flush()?;
        drop(pipe);
        if !video.wait()?.success() {
            return Err(format!("Video encoding failed: {stem}").into());
        }
        timeline.flush()?;
        summaries.push(format!("{{\"scene\":{},\"frames\":{},\"ticks\":{ticks},\"duration_seconds\":{:.9},\"video_frames\":{video_frames},\"backgrounds\":[{}],\"sprite_sets\":[{}],\"stills\":[{}]}}",
            quoted(stem),player.frame,ticks as f64/60.,used_backgrounds.iter().map(|s|quoted(s)).collect::<Vec<_>>().join(","),used_sprites.iter().map(|s|quoted(s)).collect::<Vec<_>>().join(","),shots.join(",")));
        println!(
            "{stem}: {} frames, {ticks} ticks, {video_frames} preview frames",
            player.frame
        );
    }
    fs::write(
        out.join("scenes.json"),
        format!("[{}]\n", summaries.join(",")),
    )?;
    println!(
        "Exported {} backgrounds, {} sounds, {} scene previews",
        backgrounds.len(),
        sounds.len(),
        clips.len()
    );
    Ok(())
}
