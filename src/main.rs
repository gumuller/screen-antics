mod animation;
mod audio;
mod effects;
mod hd;
mod hd2d;
#[allow(dead_code)]
mod hd2d_art;
#[allow(dead_code)]
mod hd2d_canvas;
mod hd2d_ending;
#[allow(dead_code)]
mod hd2d_layout;
mod hd2d_motion;
#[allow(dead_code)]
mod hd2d_roles;
#[allow(dead_code)]
mod hd2d_scene;
mod hd2d_story;
mod presentation;
mod remaster;
mod resources;
mod scheduler;
mod sdl;
mod style;
mod timing;
use animation::{Clip, Player};
use audio::{AudioMode, SoundBank};
use hd::HdPresentation;
use presentation::ColorMode;
use remaster::{AspectMode, RemasterAssets};
use resources::AssetStore;
use std::{
    fs,
    io::Write,
    path::PathBuf,
    time::{Duration, Instant},
};
use style::ArtStyle;

struct Options {
    data: PathBuf,
    style: ArtStyle,
    atmosphere: bool,
    painted_art: bool,
    asset_pack: Option<PathBuf>,
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
    aspect: AspectMode,
    audio: AudioMode,
}
fn options() -> Result<Options, String> {
    let mut o = Options {
        data: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("original"),
        style: ArtStyle::Remaster,
        atmosphere: true,
        painted_art: false,
        asset_pack: None,
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
        aspect: AspectMode::Auto,
        audio: AudioMode::Remastered,
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
            "--retro" => o.style = ArtStyle::Original,
            "--style" => {
                o.style = ArtStyle::parse(
                    &args
                        .next()
                        .ok_or("--style needs original, remaster or hd2d")?,
                )?
            }
            "--no-effects" => o.atmosphere = false,
            "--aspect" => {
                o.aspect = AspectMode::parse(&args.next().ok_or("--aspect needs a mode")?)?
            }
            "--audio" => {
                o.audio =
                    AudioMode::parse(&args.next().ok_or("--audio needs remastered or original")?)?
            }
            "--painted-art" => o.painted_art = true,
            "--asset-pack" => {
                o.asset_pack = Some(PathBuf::from(
                    args.next()
                        .ok_or("--asset-pack needs a directory containing sprites/")?,
                ))
            }
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
                println!("Screen Antics — native Rust player\n\n--data DIR       Original RESOURCE.001 and SCRANTIC.EXE directory\n--list           List recovered animation clips\n--scene NAME     Start a specific .TTM clip\n--fullscreen     Fullscreen display\n--screensaver    Fullscreen; exit on keyboard or mouse activity\n--mute           Disable sound\n--vibrant        Richer retro palette; C toggles live\n--retro          Original 256-color art (same as --style original)\n--style MODE     original, remaster, hd2d; H cycles, 1/2/3 select\n--no-effects     Disable HD-2D atmosphere; L toggles live\n--painted-art    Experimental generated pack (known pose mismatches)\n--asset-pack DIR Load an imported pack containing sprites/ and optional backgrounds/\n--aspect MODE    auto, original, 16:9, 21:9 or 43:18; A cycles\n--audio MODE     remastered or original; S toggles\n--resolution WxH Output resolution (default 1920x1080)\n--raw-scripts    Diagnostic linear script playback\n--original-timing Use legacy 50 ms script ticks\n--seconds N      Exit after N seconds and report measured FPS\n--headless       Decode/render without a display\n--frames N       Exit after N script frames\n--capture FILE   Save the final frame as PPM\n\nKeys: Esc quit · Right/Space next scene · Left previous · R restart\n      F fullscreen · P pause · M mute · C colors · H styles · 1/2/3 style · L atmosphere · A aspect · S audio");
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
fn pack_path(o: &Options, style: ArtStyle) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if style == o.style.pack_style() {
        if let Some(path) = &o.asset_pack {
            return path.clone();
        }
    }
    match style {
        ArtStyle::Hd2d => root.join("assets/hd2d-v2"),
        _ => {
            let remaster = root.join("assets/gemini-remaster");
            if remaster.is_dir() && !o.painted_art {
                remaster
            } else {
                root.join("assets/remastered")
            }
        }
    }
}
fn load_art(o: &Options, style: ArtStyle, assets: &AssetStore) -> Result<RemasterAssets, String> {
    if style == ArtStyle::Hd2d {
        return Ok(RemasterAssets {
            sprites: Default::default(),
            backgrounds: Default::default(),
            widescreen: Default::default(),
            remade_frames: 0,
        });
    }
    let path = pack_path(o, style);
    let imported =
        path.join("sprites/import-manifest.json").is_file() || o.asset_pack.as_ref() == Some(&path);
    let art = RemasterAssets::load_with_painted_art(&path, assets, imported || o.painted_art)?;
    if imported && art.remade_frames != assets.sprites.values().map(Vec::len).sum::<usize>() {
        return Err(format!(
            "{}: incomplete sprite pack ({} frames)",
            path.display(),
            art.remade_frames
        ));
    }
    println!(
        "Art pack: {} · {} · {} replacement frames",
        style.name(),
        path.display(),
        art.remade_frames
    );
    Ok(art)
}
fn replay_style(clip: &Clip, frame: usize, assets: &AssetStore, art: &RemasterAssets) -> Player {
    // RGBA coverage affects saved layers. Replay without sound output rather than
    // reusing provenance constructed for a different artwork pack.
    let mut player = Player::new();
    player.set_sprite_coverage(art.sprite_coverage());
    for _ in 0..frame.min(clip.frames.len()) {
        player.step(clip, assets);
    }
    player
}
fn update_picture(
    presentation: &mut HdPresentation,
    scene: &mut Option<hd2d_scene::Scene>,
    style: ArtStyle,
    player: &Player,
    assets: &AssetStore,
    art: &RemasterAssets,
    scene_name: &str,
    aspect: AspectMode,
    color: ColorMode,
    time: f64,
) -> Result<(), String> {
    if style == ArtStyle::Hd2d {
        let scene = scene.as_mut().ok_or("HD-2D scene engine not loaded")?;
        if scene.canvas.width != presentation.width || scene.canvas.height != presentation.height {
            scene.resize(presentation.width, presentation.height);
        }
        scene.render(player, assets, scene_name, aspect, time)?;
        presentation.pixels.copy_from_slice(&scene.canvas.pixels);
        presentation.set_stage_transform(scene.canvas.stage);
        Ok(())
    } else {
        presentation.update(player, assets, art, style != ArtStyle::Original, color)
    }
}
fn displayed_frame<'a>(
    style: ArtStyle,
    atmosphere: bool,
    presentation: &'a HdPresentation,
    fx: &'a hd2d::Hd2dEffects,
) -> &'a [u32] {
    if style == ArtStyle::Hd2d && atmosphere && fx.pixels.len() == presentation.pixels.len() {
        &fx.pixels
    } else {
        &presentation.pixels
    }
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
    let mut alternate_clips = Vec::new();
    if !o.raw_scripts {
        alternate_clips = scheduler::build_hd2d_clips(&assets, &clips)?;
        clips = scheduler::build_scene_clips(&assets, &clips)?;
        if o.style == ArtStyle::Hd2d {
            std::mem::swap(&mut clips, &mut alternate_clips);
        }
    }
    let mut style = o.style;
    let mut loaded_style = style.pack_style();
    let mut art = load_art(&o, loaded_style, &assets)?;
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
    let mut sounds = SoundBank::load(
        &fs::read(o.data.join("SCRANTIC.EXE")).map_err(|e| format!("Original audio: {e}"))?,
        Some(&pack_path(&o, loaded_style)),
    )?;
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
    player.set_sprite_coverage(art.sprite_coverage());
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
    presentation.set_aspect(o.aspect);
    let mut scene_renderer = if style == ArtStyle::Hd2d {
        Some(hd2d_scene::Scene::load(
            &pack_path(&o, style),
            width,
            height,
        )?)
    } else {
        None
    };
    let mut aspect = o.aspect;
    let mut audio_mode = o.audio;
    let mut queued_sounds = 0usize;
    let mut atmosphere = o.atmosphere;
    let mut hd2d_effects = hd2d::Hd2dEffects::new();
    let mut visual_time = 0f64;
    let mut visual_last = Instant::now();
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
    let mut fps_started = start;
    let mut fps_presented = 0usize;
    let mut dirty = true;
    println!(
        "Loaded {} scenes, {} sprite sets, {} sounds; 60 Hz presentation; {}",
        clips.len(),
        assets.sprites.len(),
        sounds.len(),
        if style == ArtStyle::Original {
            colors.name()
        } else {
            style.name()
        }
    );
    println!("Panoramas: {} registered variants; aspect {}; audio {} ({} replacement clips, 44100 Hz PCM16); device {}",scene_renderer.as_ref().map_or_else(||art.widescreen.values().map(Vec::len).sum::<usize>(),|s|s.art.backgrounds.values().map(|v|v.len().saturating_sub(1)).sum()),aspect.name(),audio_mode.name(),sounds.replacement_count(),if display.as_ref().is_some_and(|d|d.audio_available()) {"ready"} else if o.headless {"headless"} else {"unavailable"});
    'running: loop {
        let mut changed = false;
        let previous_style = style;
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
                                style.cycle();
                                transition = None;
                                dirty = true;
                                title_due = Instant::now()
                            }
                            49 | 50 | 51 => {
                                style = match k {
                                    49 => ArtStyle::Original,
                                    50 => ArtStyle::Remaster,
                                    _ => ArtStyle::Hd2d,
                                };
                            }
                            108 => {
                                atmosphere = !atmosphere;
                                dirty = true;
                                title_due = Instant::now();
                            }
                            99 => {
                                colors.toggle();
                                dirty = true;
                                title_due = Instant::now()
                            }
                            97 => {
                                aspect.cycle();
                                presentation.set_aspect(aspect);
                                dirty = true;
                                transition = None;
                                title_due = Instant::now();
                            }
                            115 => {
                                audio_mode.toggle();
                                d.silence();
                                title_due = Instant::now();
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
        if style != previous_style {
            let switching = Instant::now();
            let mut replay_frame = player.frame;
            if !o.raw_scripts && (style == ArtStyle::Hd2d) != (previous_style == ArtStyle::Hd2d) {
                let end_ticks: u32 = clips[idx]
                    .frames
                    .iter()
                    .take(player.frame)
                    .map(|f| f.delay_ticks.max(1))
                    .sum();
                let clock = paused_at.unwrap_or(switching);
                let remaining = script_due.saturating_duration_since(clock).as_secs_f64()
                    / timing::script_interval(1, o.original_timing).as_secs_f64();
                let elapsed_ticks = (end_ticks as f64 - remaining).max(0.);
                let tick = elapsed_ticks.floor() as u32;
                std::mem::swap(&mut clips, &mut alternate_clips);
                replay_frame = scheduler::frames_at_tick(&clips[idx], tick);
                let new_end: u32 = clips[idx]
                    .frames
                    .iter()
                    .take(replay_frame)
                    .map(|f| f.delay_ticks.max(1))
                    .sum();
                script_due = clock
                    + Duration::from_secs_f64(
                        (new_end as f64 - elapsed_ticks).max(0.)
                            * timing::script_interval(1, o.original_timing).as_secs_f64(),
                    );
            }
            if style != ArtStyle::Original && style != loaded_style {
                if style == ArtStyle::Hd2d && scene_renderer.is_none() {
                    scene_renderer = Some(hd2d_scene::Scene::load(
                        &pack_path(&o, style),
                        presentation.width,
                        presentation.height,
                    )?);
                }
                art = load_art(&o, style, &assets)?;
                sounds = SoundBank::load(
                    &fs::read(o.data.join("SCRANTIC.EXE")).map_err(|e| e.to_string())?,
                    Some(&pack_path(&o, style)),
                )?;
                loaded_style = style;
                presentation.invalidate_art();
                hd2d_effects = hd2d::Hd2dEffects::new();
            }
            player = replay_style(
                &clips[idx],
                replay_frame.min(clips[idx].frames.len()),
                &assets,
                &art,
            );
            // Pack I/O/replay must not advance the script, pause or effects clock.
            let delay = switching.elapsed();
            script_due += delay;
            if let Some(paused) = &mut paused_at {
                *paused += delay;
            }
            visual_last = Instant::now();
            present_due = visual_last;
            fps_started = visual_last;
            fps_presented = 0;
            transition = None;
            dirty = true;
            title_due = visual_last;
            if let Some(d) = &display {
                d.silence();
            }
            println!("Art style: {}", style.name());
        }
        if changed {
            transition = Some(effects::Effect::new(
                displayed_frame(style, atmosphere, &presentation, &hd2d_effects),
                None,
            ));
            player.reset();
            visual_time = 0.;
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
                                displayed_frame(style, atmosphere, &presentation, &hd2d_effects),
                                if style == ArtStyle::Hd2d {
                                    None
                                } else {
                                    Some(event)
                                },
                            );
                            script_due += effect.duration();
                            transition = Some(effect);
                        }
                    }
                    if !muted {
                        if let Some(d) = &display {
                            let story_sounds = if style == ArtStyle::Hd2d {
                                hd2d_story::sound_events(&clips[idx].name, player.frame)
                            } else {
                                None
                            };
                            for n in story_sounds.as_ref().unwrap_or(&player.sounds) {
                                if let Some(pcm) = sounds.sound(*n, audio_mode) {
                                    if d.sound(pcm)? {
                                        queued_sounds += 1;
                                    }
                                }
                            }
                        }
                    }
                }
                None => {
                    if !o.headless {
                        transition = Some(effects::Effect::new(
                            displayed_frame(style, atmosphere, &presentation, &hd2d_effects),
                            None,
                        ));
                    }
                    idx = (idx + 1) % clips.len();
                    player.reset();
                    visual_time = 0.;
                    title_due = now;
                }
            }
            if o.frames.is_some_and(|n| count >= n) || o.headless {
                break;
            }
        }
        if o.headless || now >= present_due {
            if dirty || style == ArtStyle::Hd2d {
                update_picture(
                    &mut presentation,
                    &mut scene_renderer,
                    style,
                    &player,
                    &assets,
                    &art,
                    &clips[idx].name,
                    aspect,
                    colors,
                    if o.headless {
                        player.frame as f64 / 60.
                    } else {
                        visual_time
                    },
                )?;
                dirty = false;
            }
            if paused_at.is_none() {
                visual_time += now.saturating_duration_since(visual_last).as_secs_f64();
            }
            visual_last = now;
            let render_time = if o.headless {
                player.frame as f64 / 60.
            } else {
                visual_time
            };
            let frame = if style == ArtStyle::Hd2d && atmosphere {
                let scene = scene_renderer.as_ref().unwrap();
                hd2d_effects.set_world(hd2d::WorldLighting {
                    horizon: scene.layout.horizon,
                    reflection_x: if scene.layout.night { 320. } else { 200. },
                    night: scene.layout.night,
                    outdoors: matches!(
                        scene.layout.kind,
                        hd2d_layout::SceneKind::Island
                            | hd2d_layout::SceneKind::Underwater
                            | hd2d_layout::SceneKind::Beach
                    ),
                });
                hd2d_effects.render(
                    &presentation.pixels,
                    presentation.width,
                    presentation.height,
                    presentation.stage_transform(),
                    &scene.mask,
                    &scene.fx_art,
                    render_time,
                )
            } else {
                &presentation.pixels
            };
            if let Some(d) = &display {
                if now >= title_due {
                    let fps = fps_presented as f64 / fps_started.elapsed().as_secs_f64().max(0.001);
                    fps_started = Instant::now();
                    fps_presented = 0;
                    d.title(&format!(
                        "Screen Antics · {} · {}{} · {} · {} audio{} · {:.1} FPS · H styles · 1/2/3 · L atmosphere · A aspect · S audio · F fullscreen · Esc exit",
                        clips[idx].name,
                        if style == ArtStyle::Original { colors.name() } else { style.name() },
                        if style==ArtStyle::Hd2d && !atmosphere {" (effects off)"}else{""},
                        aspect.name(),audio_mode.name(),if muted {" (muted)"}else{""},
                        fps
                    ));
                    title_due = now + Duration::from_secs(1);
                }
                if let Some(effect) = transition.as_mut() {
                    d.show(effect.frame(
                        frame,
                        presentation.width,
                        presentation.height,
                        presentation.stage_transform(),
                    ))?;
                    if effect.done() {
                        transition = None;
                    }
                } else {
                    d.show(frame)?;
                }
                presents += 1;
                fps_presented += 1;
            }
            present_due = timing::next_present(present_due, Instant::now());
        }
        if o.frames.is_some_and(|n| count >= n)
            || o.seconds
                .is_some_and(|n| start.elapsed().as_secs_f64() >= n)
        {
            // Capture the most recent script state even when the exit falls between presents.
            if dirty {
                update_picture(
                    &mut presentation,
                    &mut scene_renderer,
                    style,
                    &player,
                    &assets,
                    &art,
                    &clips[idx].name,
                    aspect,
                    colors,
                    if o.headless {
                        player.frame as f64 / 60.
                    } else {
                        visual_time
                    },
                )?;
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
    let final_frame = if style == ArtStyle::Hd2d && atmosphere {
        let scene = scene_renderer.as_ref().unwrap();
        hd2d_effects.render(
            &presentation.pixels,
            presentation.width,
            presentation.height,
            presentation.stage_transform(),
            &scene.mask,
            &scene.fx_art,
            if o.headless {
                player.frame as f64 / 60.
            } else {
                visual_time
            },
        )
    } else {
        &presentation.pixels
    };
    if let Some(path) = o.capture {
        capture(&path, final_frame, presentation.width, presentation.height)?;
        println!("Saved {}", path.display())
    }
    let elapsed = start.elapsed().as_secs_f64();
    if display.is_some() {
        println!(
            "Audio: {queued_sounds} clips queued; source {}{}",
            audio_mode.name(),
            if muted { "; muted" } else { "" }
        );
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

#[cfg(test)]
mod style_tests {
    use super::*;
    use crate::{
        animation::{Command, Frame},
        remaster::RgbaImage,
        resources::Image,
    };
    use std::collections::BTreeMap;
    #[test]
    fn switching_packs_preserves_the_frame_and_rebuilds_saved_rgba_layers() {
        let assets = AssetStore {
            palette: [[0; 3]; 256],
            raw: BTreeMap::new(),
            backgrounds: BTreeMap::from([(
                "TEST.SCR".into(),
                Image {
                    width: 640,
                    height: 350,
                    pixels: vec![1; 640 * 350],
                },
            )]),
            sprites: BTreeMap::from([(
                "ACTOR.BMP".into(),
                vec![Image {
                    width: 3,
                    height: 1,
                    pixels: vec![2, 0, 3],
                }],
            )]),
        };
        let art = |pixels: Vec<u32>| RemasterAssets {
            remade_frames: 1,
            widescreen: BTreeMap::new(),
            backgrounds: BTreeMap::new(),
            sprites: BTreeMap::from([(
                "ACTOR.BMP".into(),
                vec![RgbaImage {
                    width: 12,
                    height: 4,
                    pixels: (0..48).map(|i| pixels[(i % 12) / 4]).collect(),
                }],
            )]),
        };
        let first = art(vec![0xffff0000, 0, 0xff00ff00]);
        let third = art(vec![0x80ff8000, 0xff8000ff, 0]);
        let cmd = |opcode, args: Vec<i16>, text: &str| Command {
            opcode,
            args,
            text: text.into(),
        };
        let clip = Clip {
            name: "styles".into(),
            labels: BTreeMap::new(),
            frames: vec![
                Frame {
                    delay_ticks: 1,
                    commands: vec![
                        cmd(0xf010, vec![], "TEST.SCR"),
                        cmd(0xf020, vec![], "ACTOR.BMP"),
                        cmd(0xa500, vec![10, 10, 0, 0], ""),
                        cmd(0x0020, vec![], ""),
                    ],
                },
                Frame {
                    delay_ticks: 1,
                    commands: vec![cmd(0xa050, vec![0, 0, 640, 350], "")],
                },
                Frame {
                    delay_ticks: 1,
                    commands: vec![cmd(0xa060, vec![], "")],
                },
            ],
        };
        let old = replay_style(&clip, 3, &assets, &first);
        let switched = replay_style(&clip, old.frame, &assets, &third);
        let mut direct = Player::new();
        direct.set_sprite_coverage(third.sprite_coverage());
        for _ in 0..3 {
            direct.step(&clip, &assets);
        }
        assert_eq!(switched.frame, old.frame);
        assert_eq!(
            switched.pixels, old.pixels,
            "style switching must not change indexed choreography"
        );
        assert_eq!(switched.origins, direct.origins);
        assert_eq!(switched.sources, direct.sources);
        assert_ne!(
            switched.origins, old.origins,
            "new alpha coverage must replace old saved layers"
        );
    }
}
