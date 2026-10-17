//! Native interpreter for the DGDS TTM drawing stream used by Screen Antics.
//! Opcodes are decoded from the supplied archive, never translated to canned motion.
use crate::resources::{chunks, decompress, AssetStore, Image};
use std::collections::BTreeMap;

pub const WIDTH: usize = 640;
pub const HEIGHT: usize = 480;

#[derive(Clone, Debug)]
pub struct Command {
    pub opcode: u16,
    pub args: Vec<i16>,
    pub text: String,
}
#[derive(Clone, Debug)]
pub struct Frame {
    pub commands: Vec<Command>,
    pub delay_ticks: u32,
}
#[derive(Clone, Debug)]
pub struct Clip {
    pub name: String,
    pub frames: Vec<Frame>,
    /// TTM sequence labels, retained even though the standalone player traverses them linearly.
    pub labels: BTreeMap<i16, usize>,
}
impl Clip {
    pub fn parse(name: &str, data: &[u8]) -> Result<Self, String> {
        let sections = chunks(data)?;
        let packed = sections
            .iter()
            .find(|(id, _)| id == b"TT3:")
            .ok_or("TTM has no TT3 section")?
            .1;
        let data = decompress(packed)?;
        let mut pos = 0;
        let mut frames = Vec::new();
        let mut commands = Vec::new();
        let mut labels = BTreeMap::new();
        let mut delay_ticks = 1;
        while pos < data.len() {
            let code = word(&data, &mut pos)? as u16;
            let opcode = code & 0xfff0;
            let count = code & 15;
            let mut args = Vec::new();
            let mut text = String::new();
            if count == 15 {
                if opcode == 0xaf10 || opcode == 0xaf20 {
                    let n = word(&data, &mut pos)?;
                    if !(0..=4096).contains(&n) {
                        return Err("Invalid polygon size".into());
                    }
                    for _ in 0..n * 2 {
                        args.push(word(&data, &mut pos)?);
                    }
                } else {
                    let mut bytes = Vec::new();
                    loop {
                        let a = *data.get(pos).ok_or("Truncated TTM string")?;
                        let b = *data.get(pos + 1).ok_or("Truncated TTM string")?;
                        pos += 2;
                        if a != 0 {
                            bytes.push(a);
                        }
                        if b != 0 {
                            bytes.push(b);
                        }
                        if a == 0 || b == 0 {
                            break;
                        }
                    }
                    text = String::from_utf8_lossy(&bytes).to_uppercase();
                }
            } else {
                for _ in 0..count {
                    args.push(word(&data, &mut pos)?);
                }
            }
            if matches!(opcode, 0x1100 | 0x1110) && !args.is_empty() {
                labels.insert(args[0], frames.len());
                // ADS initializes each labeled actor sequence with its own interval.
                delay_ticks = 1;
            }
            if opcode == 0x1020 && !args.is_empty() {
                delay_ticks = ticks(args[0]);
            }
            if opcode == 0x2020 && args.len() == 2 {
                delay_ticks = ticks((args[0] as i32 + args[1] as i32).div_euclid(2) as i16);
            }
            if opcode == 0x0ff0 {
                frames.push(Frame {
                    commands: std::mem::take(&mut commands),
                    delay_ticks,
                });
            } else {
                commands.push(Command { opcode, args, text });
            }
        }
        if !commands.is_empty() {
            frames.push(Frame {
                commands,
                delay_ticks,
            });
        }
        if frames.is_empty() {
            return Err("Empty animation".into());
        }
        Ok(Self {
            name: name.into(),
            frames,
            labels,
        })
    }
    /// Deterministic indexed frame rendering, convenient for captures and diagnostics.
    #[allow(dead_code)]
    pub fn render(&self, assets: &AssetStore, frame: usize) -> Vec<u8> {
        let mut player = Player::new();
        for _ in 0..=frame.min(self.frames.len().saturating_sub(1)) {
            player.step(self, assets);
        }
        player.pixels
    }
}
fn ticks(n: i16) -> u32 {
    n.max(1) as u32
}
fn word(data: &[u8], pos: &mut usize) -> Result<i16, String> {
    let b = data.get(*pos..*pos + 2).ok_or("Truncated TTM operand")?;
    *pos += 2;
    Ok(i16::from_le_bytes([b[0], b[1]]))
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Draw {
    pub name: String,
    pub frame: usize,
    pub x: i32,
    pub y: i32,
    pub flip_x: bool,
    pub flip_y: bool,
    pub width: u32,
    pub height: u32,
    pub clip: [i32; 4],
}

/// Provenance travels with a pixel through saves, restores and stored areas.
/// Zero identifies a palette primitive; other IDs index this stable source table.
/// Captures in this format restore to their original coordinates.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PixelSource {
    Background(String),
    Sprite(Draw),
}

/// Presentation events carry source-scene coordinates. Wipes copy the current
/// composition onto the previous displayed picture inside this rectangle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransitionKind {
    Dissolve,
    Immediate,
    OutsideIn,
    Interlaced,
    LeftToRight,
    RightToLeft,
    TopToBottom,
    BottomToTop,
    InsideOut,
    FadeOut,
    FadeIn,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Transition {
    pub kind: TransitionKind,
    /// x, y, width, height in the original logical scene coordinate space.
    pub rect: [i32; 4],
    /// Presentation policy for CPU-speed-dependent original wipe loops.
    pub duration_ms: u32,
}

pub struct Player {
    pub pixels: Vec<u8>,
    pub origins: Vec<u32>,
    pub sources: Vec<PixelSource>,
    source_ids: BTreeMap<PixelSource, u32>,
    pub width: u32,
    pub height: u32,
    pub frame: usize,
    pub sounds: Vec<u16>,
    pub current_background: Option<String>,
    pub draws: Vec<Draw>,
    pub transition: Option<Transition>,
    last_transition: Option<(Transition, u64)>,
    background: Vec<u8>,
    stored: Vec<u8>,
    background_origins: Vec<u32>,
    stored_origins: Vec<u32>,
    slots: BTreeMap<i16, String>,
    getputs: BTreeMap<i16, (i32, i32, Image, Vec<u32>)>,
    bmp: i16,
    brush: i16,
    getput: i16,
    fg: u8,
    bg: u8,
    clip: [i32; 4],
}
impl Default for Player {
    fn default() -> Self {
        Self::new()
    }
}
impl Player {
    pub fn new() -> Self {
        Self {
            pixels: vec![0; WIDTH * HEIGHT],
            origins: vec![0; WIDTH * HEIGHT],
            sources: Vec::new(),
            source_ids: BTreeMap::new(),
            background: vec![0; WIDTH * HEIGHT],
            stored: vec![0; WIDTH * HEIGHT],
            background_origins: vec![0; WIDTH * HEIGHT],
            stored_origins: vec![0; WIDTH * HEIGHT],
            width: WIDTH as u32,
            height: 350,
            frame: 0,
            sounds: Vec::new(),
            current_background: None,
            draws: Vec::new(),
            transition: None,
            last_transition: None,
            slots: BTreeMap::new(),
            getputs: BTreeMap::new(),
            bmp: 0,
            brush: 0,
            getput: 0,
            fg: 15,
            bg: 15,
            clip: [0, 0, 640, 480],
        }
    }
    pub fn reset(&mut self) {
        *self = Self::new();
    }
    pub fn step(&mut self, clip: &Clip, assets: &AssetStore) -> Option<u32> {
        self.sounds.clear();
        self.draws.clear();
        self.transition = None;
        let frame = clip.frames.get(self.frame)?;
        // DGDS assembles every frame from its persistent background and stored area.
        self.pixels.copy_from_slice(&self.background);
        self.origins.copy_from_slice(&self.background_origins);
        for i in 0..self.pixels.len() {
            if self.stored[i] != 0 {
                self.pixels[i] = self.stored[i];
                self.origins[i] = self.stored_origins[i];
            }
        }
        for command in &frame.commands {
            self.execute(command, assets);
        }
        // ADS can hold/repaint a source frame across several scheduler ticks.
        // Repainting an unchanged wipe target must not restart its presentation.
        if let Some(event) = self.transition {
            let signature = self.pixels.iter().fold(0xcbf29ce484222325u64, |hash, &p| {
                (hash ^ p as u64).wrapping_mul(0x100000001b3)
            });
            let identity = (event, signature);
            if self.last_transition == Some(identity) {
                self.transition = None;
            }
            self.last_transition = Some(identity);
        } else {
            self.last_transition = None;
        }
        self.frame += 1;
        Some(frame.delay_ticks)
    }
    fn source_id(&mut self, source: PixelSource) -> u32 {
        if let Some(&id) = self.source_ids.get(&source) {
            return id;
        }
        let id = self.sources.len() as u32 + 1;
        self.sources.push(source.clone());
        self.source_ids.insert(source, id);
        id
    }
    fn execute(&mut self, command: &Command, assets: &AssetStore) {
        let a = &command.args;
        let arg = |n: usize| a.get(n).copied().unwrap_or(0) as i32;
        match command.opcode {
            0xf010 => {
                if let Some(image) = assets.backgrounds.get(&command.text) {
                    self.current_background = Some(command.text.clone());
                    self.width = image.width;
                    self.height = image.height;
                    self.pixels.fill(0);
                    self.origins.fill(0);
                    let id = self.source_id(PixelSource::Background(command.text.clone()));
                    self.blit(image, 0, 0, false, false, false, None, id);
                    self.background.copy_from_slice(&self.pixels);
                    self.background_origins.copy_from_slice(&self.origins);
                    self.stored.fill(0);
                    self.stored_origins.fill(0);
                }
            }
            0xc050 | 0x1300 => {
                if arg(0) > 0 {
                    self.sounds.push(arg(0) as u16);
                }
            }
            0xf020 => {
                self.slots.insert(self.bmp, command.text.clone());
            }
            0x1050 => self.bmp = arg(0) as i16,
            0x1030 => self.brush = arg(0) as i16,
            0x0080 => {
                self.slots.remove(&self.bmp);
            }
            0x1120 => self.getput = arg(0) as i16,
            0x2000 => {
                self.fg = arg(0) as u8;
                self.bg = arg(1) as u8;
            }
            0x4000 => {
                self.clip = [
                    arg(0).max(0),
                    arg(1).max(0),
                    (arg(2) + 1).min(640),
                    (arg(3) + 1).min(480),
                ]
            }
            0x0020 => {
                self.background.copy_from_slice(&self.pixels);
                self.background_origins.copy_from_slice(&self.origins);
            }
            0x4200 => {
                let mut buffer = self.stored.clone();
                copy_region(&self.pixels, &mut buffer, arg(0), arg(1), arg(2), arg(3));
                self.stored = buffer;
                copy_region(
                    &self.origins,
                    &mut self.stored_origins,
                    arg(0),
                    arg(1),
                    arg(2),
                    arg(3),
                );
            }
            0x4210 => {
                let (x, y, w, h) = (arg(0), arg(1), arg(2).clamp(0, 640), arg(3).clamp(0, 480));
                let mut image = Image {
                    width: w as u32,
                    height: h as u32,
                    pixels: vec![0; (w * h) as usize],
                };
                let mut origins = vec![0; (w * h) as usize];
                for yy in 0..h {
                    for xx in 0..w {
                        if (0..640).contains(&(x + xx)) && (0..480).contains(&(y + yy)) {
                            image.pixels[(yy * w + xx) as usize] =
                                self.pixels[((y + yy) * 640 + x + xx) as usize];
                            origins[(yy * w + xx) as usize] =
                                self.origins[((y + yy) * 640 + x + xx) as usize];
                        }
                    }
                }
                self.getputs.insert(self.getput, (x, y, image, origins));
            }
            0xa600 => {
                if let Some((x, y, image, origins)) = self.getputs.get(&(arg(0) as i16)).cloned() {
                    // Restore rectangles ignore the drawing clip and preserve black pixels.
                    self.blit(&image, x, y, false, false, false, None, 0);
                    for yy in 0.max(-y)..(image.height as i32).min(480 - y) {
                        for xx in 0.max(-x)..(image.width as i32).min(640 - x) {
                            self.origins[((y + yy) * 640 + x + xx) as usize] =
                                origins[yy as usize * image.width as usize + xx as usize];
                        }
                    }
                }
            }
            0xa500 | 0xa510 | 0xa520 | 0xa530 => {
                let (frame, slot) = if a.len() >= 4 {
                    (arg(2) as usize, arg(3) as i16)
                } else {
                    (self.brush as usize, self.bmp)
                };
                if let Some(image) = self
                    .slots
                    .get(&slot)
                    .and_then(|name| assets.sprites.get(name))
                    .and_then(|images| images.get(frame))
                {
                    let scale = if a.len() >= 6 && arg(4) > 0 && arg(5) > 0 {
                        Some((arg(4), arg(5)))
                    } else {
                        None
                    };
                    let draw = Draw {
                        name: self.slots.get(&slot).cloned().unwrap_or_default(),
                        frame,
                        x: arg(0),
                        y: arg(1),
                        flip_x: command.opcode & 0x20 != 0,
                        flip_y: command.opcode & 0x10 != 0,
                        width: scale.map_or(image.width, |(w, _)| w as u32),
                        height: scale.map_or(image.height, |(_, h)| h as u32),
                        clip: self.clip,
                    };
                    let id = self.source_id(PixelSource::Sprite(draw.clone()));
                    self.draws.push(draw);
                    self.blit(
                        image,
                        arg(0),
                        arg(1),
                        command.opcode & 0x20 != 0,
                        command.opcode & 0x10 != 0,
                        true,
                        scale,
                        id,
                    );
                }
            }
            0xa010..=0xa090 => {
                let kind = match command.opcode {
                    0xa010 => TransitionKind::Dissolve,
                    0xa020 => TransitionKind::Immediate,
                    0xa030 => TransitionKind::OutsideIn,
                    0xa040 => TransitionKind::Interlaced,
                    0xa050 => TransitionKind::LeftToRight,
                    0xa060 => TransitionKind::RightToLeft,
                    0xa070 => TransitionKind::TopToBottom,
                    0xa080 => TransitionKind::BottomToTop,
                    _ => TransitionKind::InsideOut,
                };
                let x = arg(0).clamp(0, self.width as i32);
                let y = arg(1).clamp(0, self.height as i32);
                let w = arg(2).max(0).min(self.width as i32 - x);
                let h = arg(3).max(0).min(self.height as i32 - y);
                self.transition = Some(Transition {
                    kind,
                    rect: [x, y, w, h],
                    duration_ms: if kind == TransitionKind::Immediate {
                        0
                    } else {
                        350
                    },
                });
            }
            0x4110 | 0x4120 => {
                self.transition = Some(Transition {
                    kind: if command.opcode == 0x4110 {
                        TransitionKind::FadeOut
                    } else {
                        TransitionKind::FadeIn
                    },
                    rect: [0, 0, self.width as i32, self.height as i32],
                    duration_ms: if arg(3) <= 0 { 0 } else { 8000 / arg(3) as u32 },
                });
            }
            0xa000 => self.pixel(arg(0), arg(1), self.fg),
            0xa0a0 => self.line(arg(0), arg(1), arg(2), arg(3)),
            0xa100 | 0xa110 => {
                let (x, y, w, h) = (arg(0), arg(1), arg(2), arg(3));
                if command.opcode == 0xa100 {
                    for yy in y.max(0)..(y + h).min(480) {
                        for xx in x.max(0)..(x + w).min(640) {
                            self.pixel(xx, yy, self.bg);
                        }
                    }
                }
                self.line(x, y, x + w - 1, y);
                self.line(x, y, x, y + h - 1);
                self.line(x + w - 1, y, x + w - 1, y + h - 1);
                self.line(x, y + h - 1, x + w - 1, y + h - 1);
            }
            0xa400 | 0xa420 => {
                let (x, y, w, h) = (arg(0), arg(1), arg(2), arg(3));
                if w <= 0 || h <= 0 {
                    return;
                }
                let rx = w as f64 / 2.;
                let ry = h as f64 / 2.;
                for yy in y.max(0)..(y + h).min(480) {
                    for xx in x.max(0)..(x + w).min(640) {
                        let dist = ((xx - x) as f64 - rx).powi(2) / rx.powi(2)
                            + ((yy - y) as f64 - ry).powi(2) / ry.powi(2);
                        if dist <= 1. {
                            if dist >= 1. - 2. / rx.min(ry) {
                                self.pixel(xx, yy, self.fg);
                            } else if command.opcode == 0xa400 {
                                self.pixel(xx, yy, self.bg);
                            }
                        }
                    }
                }
            }
            0xb600 => {
                let source = match arg(4) {
                    0 => &self.background,
                    1 => &self.stored,
                    _ => &self.pixels,
                }
                .clone();
                let target = match arg(5) {
                    0 => &mut self.background,
                    1 => &mut self.stored,
                    _ => &mut self.pixels,
                };
                copy_region(&source, target, arg(0), arg(1), arg(2), arg(3));
                let source = match arg(4) {
                    0 => &self.background_origins,
                    1 => &self.stored_origins,
                    _ => &self.origins,
                }
                .clone();
                let target = match arg(5) {
                    0 => &mut self.background_origins,
                    1 => &mut self.stored_origins,
                    _ => &mut self.origins,
                };
                copy_region(&source, target, arg(0), arg(1), arg(2), arg(3));
            }
            // Palette selection uses the archive's single common palette. Sequence control,
            // sample playback, allocation hints, and timing have no drawing side effects.
            _ => {}
        }
    }
    fn pixel(&mut self, x: i32, y: i32, color: u8) {
        if x >= self.clip[0]
            && x < self.clip[2]
            && y >= self.clip[1]
            && y < self.clip[3]
            && (0..640).contains(&x)
            && (0..480).contains(&y)
        {
            self.pixels[(y * 640 + x) as usize] = color;
            self.origins[(y * 640 + x) as usize] = 0;
        }
    }
    fn line(&mut self, mut x: i32, mut y: i32, x2: i32, y2: i32) {
        let dx = (x2 - x).abs();
        let sx = if x < x2 { 1 } else { -1 };
        let dy = -(y2 - y).abs();
        let sy = if y < y2 { 1 } else { -1 };
        let mut error = dx + dy;
        for _ in 0..65536 {
            self.pixel(x, y, self.fg);
            if x == x2 && y == y2 {
                break;
            }
            let e = error * 2;
            if e >= dy {
                error += dy;
                x += sx;
            }
            if e <= dx {
                error += dx;
                y += sy;
            }
        }
    }
    fn blit(
        &mut self,
        image: &Image,
        x: i32,
        y: i32,
        flip_x: bool,
        flip_y: bool,
        transparent: bool,
        scale: Option<(i32, i32)>,
        origin: u32,
    ) {
        let (w, h) = scale.unwrap_or((image.width as i32, image.height as i32));
        if w <= 0 || h <= 0 || image.width == 0 || image.height == 0 {
            return;
        }
        for yy in 0.max(-y)..h.min(480 - y) {
            for xx in 0.max(-x)..w.min(640 - x) {
                let mut sx = xx * image.width as i32 / w;
                let mut sy = yy * image.height as i32 / h;
                if flip_x {
                    sx = image.width as i32 - 1 - sx;
                }
                if flip_y {
                    sy = image.height as i32 - 1 - sy;
                }
                if let Some(&color) = image.pixels.get((sy * image.width as i32 + sx) as usize) {
                    if transparent {
                        if color != 0 {
                            self.pixel(x + xx, y + yy, color);
                            if x + xx >= self.clip[0]
                                && x + xx < self.clip[2]
                                && y + yy >= self.clip[1]
                                && y + yy < self.clip[3]
                            {
                                self.origins[((y + yy) * 640 + x + xx) as usize] = origin;
                            }
                        }
                    } else {
                        self.pixels[((y + yy) * 640 + x + xx) as usize] = color;
                        self.origins[((y + yy) * 640 + x + xx) as usize] = origin;
                    }
                }
            }
        }
    }
}
fn copy_region<T: Copy>(source: &[T], target: &mut [T], x: i32, y: i32, w: i32, h: i32) {
    for yy in y.max(0)..(y + h).min(480) {
        for xx in x.max(0)..(x + w).min(640) {
            let p = (yy * 640 + xx) as usize;
            target[p] = source[p];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn supplied_archive_scripts_decode_and_render_to_completion() {
        let assets = AssetStore::load(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("original")
                .as_path(),
        )
        .unwrap();
        let mut count = 0;
        let mut frames = 0;
        for (name, bytes) in &assets.raw {
            if !name.ends_with(".TTM") {
                continue;
            }
            let clip = Clip::parse(name, bytes).unwrap_or_else(|error| panic!("{name}: {error}"));
            let mut player = Player::new();
            while player.step(&clip, &assets).is_some() {
                frames += 1;
            }
            assert_eq!(player.frame, clip.frames.len(), "{name}");
            assert_eq!(player.pixels.len(), WIDTH * HEIGHT);
            count += 1;
        }
        assert_eq!(count, 41);
        assert_eq!(frames, 6631);
    }
    #[test]
    fn resolved_hd_draws_match_original_jogging_commands() {
        let assets = AssetStore::load(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("original")
                .as_path(),
        )
        .unwrap();
        let clip = Clip::parse("MJJOG.TTM", &assets.raw["MJJOG.TTM"]).unwrap();
        let mut player = Player::new();
        player.step(&clip, &assets);
        player.step(&clip, &assets);
        assert_eq!(player.current_background.as_deref(), Some("ISLETEMP.SCR"));
        let walk = player
            .draws
            .iter()
            .find(|d| d.name == "JOHNWALK.BMP")
            .unwrap();
        assert_eq!(
            (walk.frame, walk.x, walk.y, walk.flip_x),
            (4, 459, 218, true)
        );
        assert!(!walk.flip_y);
        assert_eq!(walk.width, assets.sprites["JOHNWALK.BMP"][4].width);
        assert_eq!(walk.clip, [0, 0, 640, 480]);
    }

    fn empty_assets() -> AssetStore {
        AssetStore {
            palette: [[0; 3]; 256],
            backgrounds: BTreeMap::new(),
            sprites: BTreeMap::new(),
            raw: BTreeMap::new(),
        }
    }
    fn op(opcode: u16, args: &[i16]) -> Command {
        Command {
            opcode,
            args: args.to_vec(),
            text: String::new(),
        }
    }
    #[test]
    fn wipe_events_have_source_regions_and_held_repaints_do_not_restart() {
        let assets = empty_assets();
        let wipe = Frame {
            commands: vec![op(0xa050, &[194, 123, 175, 142])],
            delay_ticks: 1,
        };
        let clip = Clip {
            name: "wipe".into(),
            frames: vec![
                wipe.clone(),
                wipe.clone(),
                Frame {
                    commands: vec![],
                    delay_ticks: 1,
                },
                wipe,
            ],
            labels: BTreeMap::new(),
        };
        let mut player = Player::new();
        player.step(&clip, &assets);
        assert_eq!(
            player.transition,
            Some(Transition {
                kind: TransitionKind::LeftToRight,
                rect: [194, 123, 175, 142],
                duration_ms: 350
            })
        );
        player.step(&clip, &assets);
        assert!(player.transition.is_none(), "held repaint restarted wipe");
        player.step(&clip, &assets);
        assert!(player.transition.is_none());
        player.step(&clip, &assets);
        assert!(
            player.transition.is_some(),
            "later distinct wipe was suppressed"
        );
    }
    #[test]
    fn actual_gulliver_script_exposes_both_original_wipe_directions() {
        let assets = AssetStore::load(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("original")
                .as_path(),
        )
        .unwrap();
        let clip = Clip::parse("GJGULIVR.TTM", &assets.raw["GJGULIVR.TTM"]).unwrap();
        let mut player = Player::new();
        let mut events = Vec::new();
        while player.step(&clip, &assets).is_some() {
            if let Some(event) = player.transition {
                events.push(event);
            }
        }
        assert_eq!(
            events.iter().map(|e| e.kind).collect::<Vec<_>>(),
            vec![TransitionKind::LeftToRight, TransitionKind::RightToLeft]
        );
        assert!(events.iter().all(|e| e.rect == [194, 123, 175, 142]));
    }
    #[test]
    fn stored_area_uses_zero_key_but_getput_restores_zero_opaquely() {
        // DGDS composition intentionally transBlits the stored overlay. A captured
        // get/put rectangle, in contrast, is an opaque copy including index zero.
        let assets = empty_assets();
        let clip = Clip {
            name: "overlay".into(),
            frames: vec![Frame {
                commands: vec![],
                delay_ticks: 1,
            }],
            labels: BTreeMap::new(),
        };
        let mut player = Player::new();
        player.background.fill(5);
        player.stored[1] = 2;
        player.step(&clip, &assets);
        assert_eq!(&player.pixels[..2], &[5, 2]);
        player.pixels[0] = 0;
        player.execute(&op(0x4210, &[0, 0, 1, 1]), &assets);
        player.pixels[0] = 5;
        player.execute(&op(0xa600, &[0]), &assets);
        assert_eq!(player.pixels[0], 0);
    }

    #[test]
    fn parser_rejects_truncated_operands_and_strings() {
        for script in [vec![0x21, 0x10], vec![0x1f, 0xf0, b'A']] {
            let mut packed = vec![0];
            packed.extend_from_slice(&(script.len() as u32).to_le_bytes());
            packed.extend(script);
            let mut ttm = b"TT3:".to_vec();
            ttm.extend_from_slice(&(packed.len() as u32).to_le_bytes());
            ttm.extend(packed);
            assert!(Clip::parse("broken.TTM", &ttm).is_err());
        }
    }
    #[test]
    fn delays_are_sequence_local_and_remain_in_ticks() {
        // DELAY 30; refresh; sequence 7; refresh; DELAY 2; refresh.
        let words: [u16; 9] = [0x1021, 30, 0x0ff0, 0x1111, 7, 0x0ff0, 0x1021, 2, 0x0ff0];
        let script: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        let mut packed = vec![0];
        packed.extend_from_slice(&(script.len() as u32).to_le_bytes());
        packed.extend(script);
        let mut ttm = b"TT3:".to_vec();
        ttm.extend_from_slice(&(packed.len() as u32).to_le_bytes());
        ttm.extend(packed);
        let clip = Clip::parse("timing.TTM", &ttm).unwrap();
        assert_eq!(
            clip.frames
                .iter()
                .map(|f| f.delay_ticks)
                .collect::<Vec<_>>(),
            [30, 1, 2]
        );
    }
    #[test]
    fn mirrored_sprite_obeys_clip_and_zero_transparency() {
        let mut player = Player::new();
        player.clip = [1, 0, 3, 2];
        let image = Image {
            width: 3,
            height: 1,
            pixels: vec![1, 0, 2],
        };
        player.pixels.fill(9);
        player.blit(&image, 0, 0, true, false, true, None, 0);
        assert_eq!(&player.pixels[..4], &[9, 9, 1, 9]);
    }
}
