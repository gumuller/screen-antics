mod animation;
mod audio;
mod effects;
mod hd;
mod presentation;
mod remaster;
mod resources;
mod scheduler;
mod sdl;
mod timing;
use animation::{Clip, Player};
use hd::HdPresentation;
use presentation::ColorMode;
use remaster::RemasterAssets;
use resources::AssetStore;
use std::{
    fs,
    io::Write,
    path::PathBuf,
    time::{Duration, Instant},
};

struct Options {
    data: PathBuf,
    remastered: bool,
    painted_art: bool,
    resolution: (usize, usize),
    fixed_resolution: bool,
    raw_scripts: bool,
    scene: Option<String>,
    list: bool,
    full: bool,
    saver: bool,
    headless: bool,
    frames: Option<usize>,
    capture: Option<PathBuf>,
    mute: bool,
    color: ColorMode,
    original_timing: bool,
    seconds: Option<f64>,
}
fn options() -> Result<Options, String> {
    let mut o = Options {
        data: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("original"),
        remastered: true,
        painted_art: false,
        resolution: (1920, 1080),
        fixed_resolution: false,
        raw_scripts: false,
        scene: None,
        list: false,
        full: false,
        saver: false,
        headless: false,
        frames: None,
        capture: None,
        mute: false,
        color: ColorMode::Classic,
        original_timing: false,
        seconds: None,
    };
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--data" => o.data = PathBuf::from(args.next().ok_or("--data needs a directory")?),
            "--scene" => {
                o.scene = Some(
                    args.next()
                        .ok_or("--scene needs a TTM name")?
                        .to_uppercase(),
                )
            }
            "--list" => o.list = true,
            "--fullscreen" => o.full = true,
            "--screensaver" => {
                o.full = true;
                o.saver = true
            }
            "--headless" => o.headless = true,
            "--mute" => o.mute = true,
            "--vibrant" => o.color = ColorMode::Vibrant,
            "--retro" => o.remastered = false,
            "--painted-art" => o.painted_art = true,
            "--raw-scripts" => o.raw_scripts = true,
            "--resolution" => {
                o.fixed_resolution = true;
                let size = args.next().ok_or("--resolution needs WIDTHxHEIGHT")?;
                let (w, h) = size.split_once('x').ok_or("Use WIDTHxHEIGHT")?;
                o.resolution = (
                    w.parse().map_err(|_| "Invalid width")?,
                    h.parse().map_err(|_| "Invalid height")?,
                );
                if o.resolution.0 < 320
                    || o.resolution.1 < 200
                    || o.resolution.0 > 7680
                    || o.resolution.1 > 4320
                {
                    return Err("Resolution must be between 320x200 and 7680x4320".into());
                }
            }
            "--original-timing" => o.original_timing = true,
            "--seconds" => {
                o.seconds = Some(
                    args.next()
                        .ok_or("--seconds needs a duration")?
                        .parse()
                        .map_err(|_| "Invalid duration")?,
                )
            }
            "--frames" => {
                o.frames = Some(
                    args.next()
                        .ok_or("--frames needs a count")?
                        .parse()
                        .map_err(|_| "Invalid frame count")?,
                )
            }
            "--capture" => {
                o.capture = Some(args.next().ok_or("--capture needs a PPM path")?.into())
            }
            "--help" | "-h" => {
                println!("Screen Antics — native Rust player\n\n--data DIR       Original RESOURCE.001 and SCRANTIC.EXE directory\n--list           List recovered animation clips\n--scene NAME     Start a specific .TTM clip\n--fullscreen     Fullscreen display\n--screensaver    Fullscreen; exit on keyboard or mouse activity\n--mute           Disable sound\n--vibrant        Richer retro palette; C toggles live\n--retro          Original art instead of remastered art; H toggles\n--painted-art    Experimental generated pack (known pose mismatches)\n--resolution WxH Output resolution (default 1920x1080)\n--raw-scripts    Diagnostic linear script playback\n--original-timing Use legacy 50 ms script ticks\n--seconds N      Exit after N seconds and report measured FPS\n--headless       Decode/render without a display\n--frames N       Exit after N script frames\n--capture FILE   Save the final frame as PPM\n\nKeys: Esc quit · Right/Space next scene · Left previous · R restart\n      F fullscreen · P pause · M mute · C colors · H remaster");
                std::process::exit(0)
            }
            _ => return Err(format!("Unknown option {a}")),
        }
    }
    if o.headless && o.frames.is_none() && o.seconds.is_none() {
        o.frames = Some(120)
    }
    if o.frames == Some(0) {
        return Err("--frames must be positive".into());
    }
    if o.seconds
        .is_some_and(|n| !n.is_finite() || n <= 0. || n > 86400.)
    {
        return Err("--seconds must be between 0 and 86400".into());
    }
    Ok(o)
}
fn capture(path: &PathBuf, pixels: &[u32], width: usize, height: usize) -> Result<(), String> {
    let mut f = fs::File::create(path).map_err(|e| e.to_string())?;
    write!(f, "P6\n{width} {height}\n255\n").map_err(|e| e.to_string())?;
    let rgb: Vec<u8> = pixels
        .iter()
        .flat_map(|p| [(p >> 16) as u8, (p >> 8) as u8, *p as u8])
        .collect();
    f.write_all(&rgb).map_err(|e| e.to_string())
}
fn run() -> Result<(), String> {
    let o = options()?;
    let assets = AssetStore::load(&o.data)?;
    let mut clips = Vec::new();
    for (name, data) in &assets.raw {
        if name.ends_with(".TTM") {
            clips.push(Clip::parse(name, data).map_err(|e| format!("{name}: {e}"))?)
        }
    }
    if clips.is_empty() {
        return Err("No TTM animation resources found".into());
    }
    if o.list {
        for c in &clips {
            println!(
                "{:<16} {} frames, {} sequences",
                c.name,
                c.frames.len(),
                c.labels.len()
            )
        }
        return Ok(());
    }
    if !o.raw_scripts {
        clips = scheduler::build_scene_clips(&assets, &clips)?;
    }
    let art = RemasterAssets::load_with_painted_art(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/remastered"),
        &assets,
        o.painted_art,
    )?;
    println!(
        "HD art: {} backgrounds, {} source-faithful sprite frames; {} experimental replacements",
        art.backgrounds.len(),
        art.sprites.values().map(Vec::len).sum::<usize>() - art.remade_frames,
        art.remade_frames
    );
    let mut idx = if let Some(name) = &o.scene {
        let n = if name.ends_with(".TTM") {
            name.clone()
        } else {
            format!("{name}.TTM")
        };
        clips
            .iter()
            .position(|c| c.name == n)
            .ok_or_else(|| format!("Unknown scene {name}; use --list"))?
    } else {
        clips
            .iter()
            .position(|c| c.name == "MJJOG.TTM")
            .unwrap_or(0)
    };
    let sounds = fs::read(o.data.join("SCRANTIC.EXE"))
        .ok()
        .and_then(|b| audio::waves(&b).ok())
        .unwrap_or_default();
    let mut display = if o.headless {
        None
    } else {
        Some(sdl::Display::new(
            o.full,
            o.resolution.0 as u32,
            o.resolution.1 as u32,
        )?)
    };
    let mut player = Player::new();
    let (width, height) = if o.fixed_resolution {
        o.resolution
    } else {
        display
            .as_ref()
            .map(|d| d.drawable_size())
            .filter(|&(w, h)| w >= 320 && h >= 200 && w <= 7680 && h <= 4320)
            .map(|(w, h)| (w as usize, h as usize))
            .unwrap_or(o.resolution)
    };
    if let Some(d) = display.as_mut() {
        d.resize_surface(width as u32, height as u32)?;
    }
    let mut presentation = HdPresentation::new(width, height, &assets.palette);
    let mut hd = o.remastered;
    let mut transition: Option<effects::Effect> = None;
    let mut colors = o.color;
    let mut paused_at: Option<Instant> = None;
    let mut muted = o.mute;
    let mut count = 0;
    let mut presents = 0;
    let start = Instant::now();
    let mut script_due = start;
    let mut present_due = start;
    let mut title_due = start;
    let mut dirty = true;
    println!(
        "Loaded {} scenes, {} sprite sets, {} sounds; 60 Hz presentation; {}",
        clips.len(),
        assets.sprites.len(),
        sounds.len(),
        if hd { "HD 24-bit" } else { colors.name() }
    );
    'running: loop {
        let mut changed = false;
        if let Some(d) = display.as_mut() {
            for event in d.events() {
                match event {
                    sdl::Event::Quit => break 'running,
                    sdl::Event::Resize(w, h) => {
                        if !o.fixed_resolution && w >= 320 && h >= 200 && w <= 7680 && h <= 4320 {
                            d.resize_surface(w, h)?;
                            presentation.resize(w as usize, h as usize);
                            dirty = true;
                            transition = None;
                        }
                    }
                    sdl::Event::Mouse => {
                        if o.saver && start.elapsed() > Duration::from_millis(800) {
                            break 'running;
                        }
                    }
                    sdl::Event::Key(k) => {
                        if o.saver || k == 27 {
                            break 'running;
                        }
                        match k {
                            32 | 1073741903 => {
                                idx = (idx + 1) % clips.len();
                                changed = true
                            }
                            1073741904 => {
                                idx = (idx + clips.len() - 1) % clips.len();
                                changed = true
                            }
                            114 => changed = true,
                            102 => d.fullscreen(),
                            104 => {
                                hd = !hd;
                                dirty = true;
                                title_due = Instant::now()
                            }
                            99 => {
                                colors.toggle();
                                dirty = true;
                                title_due = Instant::now()
                            }
                            112 => {
                                if let Some(at) = paused_at.take() {
                                    script_due += at.elapsed()
                                } else {
                                    paused_at = Some(Instant::now())
                                }
                            }
                            109 => {
                                muted = !muted;
                                if muted {
                                    d.silence()
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        if changed {
            transition = Some(effects::Effect::new(
                &presentation.pixels,
                None,
                (player.width as usize, player.height as usize),
            ));
            player.reset();
            script_due = Instant::now();
            paused_at = None;
            dirty = true;
            title_due = Instant::now();
            if let Some(d) = &display {
                d.silence()
            }
        }
        let now = Instant::now();
        // A small bounded catch-up prevents a delayed presentation from slowing the script clock.
        for _ in 0..8 {
            if paused_at.is_some() || (!o.headless && now < script_due) {
                break;
            }
            match player.step(&clips[idx], &assets) {
                Some(ticks) => {
                    dirty = true;
                    count += 1;
                    script_due += timing::script_interval(ticks, o.original_timing);
                    if !o.headless {
                        if let Some(event) = player.transition.clone() {
                            let effect = effects::Effect::new(
                                &presentation.pixels,
                                Some(event),
                                (player.width as usize, player.height as usize),
                            );
                            script_due += effect.duration();
                            transition = Some(effect);
                        }
                    }
                    if !muted {
                        if let Some(d) = &display {
                            for n in &player.sounds {
                                if let Some(pcm) = u8::try_from(*n)
                                    .ok()
                                    .and_then(audio::index_for_sfx)
                                    .and_then(|i| sounds.get(i))
                                {
                                    d.sound(pcm)
                                }
                            }
                        }
                    }
                }
                None => {
                    if !o.headless {
                        transition = Some(effects::Effect::new(
                            &presentation.pixels,
                            None,
                            (player.width as usize, player.height as usize),
                        ));
                    }
                    idx = (idx + 1) % clips.len();
                    player.reset();
                    title_due = now;
                }
            }
            if o.frames.is_some_and(|n| count >= n) || o.headless {
                break;
            }
        }
        if o.headless || now >= present_due {
            if dirty {
                presentation.update(&player, &assets, &art, hd, colors);
                dirty = false;
            }
            if let Some(d) = &display {
                if now >= title_due {
                    let fps = presents as f64 / start.elapsed().as_secs_f64().max(0.001);
                    d.title(&format!(
                        "Screen Antics · {} · {} · {:.1} FPS · H art · C colors · P pause · Esc exit",
                        clips[idx].name,
                        if hd{"HD 24-bit"}else{colors.name()},
                        fps
                    ));
                    title_due = now + Duration::from_secs(1);
                }
                if let Some(effect) = transition.as_mut() {
                    d.show(effect.frame(
                        &presentation.pixels,
                        presentation.width,
                        presentation.height,
                    ))?;
                    if effect.done() {
                        transition = None;
                    }
                } else {
                    d.show(&presentation.pixels)?;
                }
                presents += 1;
            }
            present_due = timing::next_present(present_due, Instant::now());
        }
        if o.frames.is_some_and(|n| count >= n)
            || o.seconds
                .is_some_and(|n| start.elapsed().as_secs_f64() >= n)
        {
            // Capture the most recent script state even when the exit falls between presents.
            if dirty {
                presentation.update(&player, &assets, &art, hd, colors)
            }
            break;
        }
        if !o.headless {
            std::thread::sleep(
                present_due
                    .saturating_duration_since(Instant::now())
                    .min(Duration::from_millis(2)),
            )
        }
    }
    if let Some(path) = o.capture {
        capture(
            &path,
            &presentation.pixels,
            presentation.width,
            presentation.height,
        )?;
        println!("Saved {}", path.display())
    }
    let elapsed = start.elapsed().as_secs_f64();
    if display.is_some() {
        println!("Presented {presents} frames in {elapsed:.3}s ({:.2} FPS); executed {count} script frames",presents as f64/elapsed.max(0.001))
    } else {
        println!("Rendered {count} script frames in {elapsed:.3}s")
    }
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Screen Antics: {e}");
        std::process::exit(1)
    }
}
