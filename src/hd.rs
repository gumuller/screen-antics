//! True-color presentation of the VM's final pixel provenance.
//! Saves/restores and painter order are resolved by the VM, not guessed from
//! sprite bounding boxes. This also preserves opaque palette-zero primitives.
use crate::{
    animation::{PixelSource, Player},
    presentation::{ColorMode, Presentation},
    remaster::{load_image, AspectMode, RemasterAssets, RgbaImage},
    resources::AssetStore,
};

type Texture<'a> = (&'a RgbaImage, f64, f64, f64, f64, bool, bool, bool);
#[derive(Clone, Copy)]
enum Source<'a> {
    Texture(Texture<'a>),
    Color(u32),
    Layer { top: u32, under: u32 },
    Missing,
}

pub struct HdPresentation {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u32>,
    base: Vec<u32>,
    cache_key: String,
    retro: Presentation,
    aspect: AspectMode,
    stage: [f64; 3],
    loaded_wide: Option<(std::path::PathBuf, RgbaImage)>,
}
impl HdPresentation {
    pub fn new(width: usize, height: usize, palette: &[[u8; 3]; 256]) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; width * height],
            base: vec![0; width * height],
            cache_key: String::new(),
            retro: Presentation::new(palette),
            aspect: AspectMode::Auto,
            stage: [1., 0., 0.],
            loaded_wide: None,
        }
    }
    pub fn set_aspect(&mut self, aspect: AspectMode) {
        self.aspect = aspect;
        self.cache_key.clear();
    }
    pub fn stage_transform(&self) -> [f64; 3] {
        self.stage
    }
    /// A different pack may use the same background name, size and core aspect.
    /// Its cached backdrop must still be rebuilt before rendering new sprites.
    pub fn set_stage_transform(&mut self, stage: [f64; 3]) {
        self.stage = stage;
    }
    pub fn invalidate_art(&mut self) {
        self.cache_key.clear();
        self.loaded_wide = None;
    }
    pub fn resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
        self.pixels.resize(width * height, 0);
        self.base.resize(width * height, 0);
        self.cache_key.clear();
    }
    pub fn update(
        &mut self,
        player: &Player,
        original: &AssetStore,
        art: &RemasterAssets,
        hd: bool,
        color: ColorMode,
    ) -> Result<(), String> {
        let sw = player.width.clamp(1, 640) as usize;
        let sh = player.height.clamp(1, 480) as usize;
        let name = player.current_background.as_deref().unwrap_or("");
        let wide = if hd {
            art.select_widescreen(name, self.width, self.height, self.aspect)
        } else {
            None
        };
        let mut scale = (self.width as f64 / sw as f64).min(self.height as f64 / sh as f64);
        let mut ox = (self.width as f64 - sw as f64 * scale) / 2.;
        let mut oy = (self.height as f64 - sh as f64 * scale) / 2.;
        let mut canvas = [1., 0., 0.];
        if let Some(wide) = wide {
            let s = (self.width as f64 / wide.width as f64)
                .min(self.height as f64 / wide.height as f64);
            let x = (self.width as f64 - wide.width as f64 * s) / 2.;
            let y = (self.height as f64 - wide.height as f64 * s) / 2.;
            scale = s * 4.;
            ox = x + wide.stage[0] as f64 * s;
            oy = y + wide.stage[1] as f64 * s;
            canvas = [s, x, y];
        }
        self.stage = [scale, ox, oy];
        if !hd {
            self.retro.update(&player.pixels, sh, color);
            for y in 0..self.height {
                for x in 0..self.width {
                    let sx = ((x as f64 + 0.5 - ox) / scale).floor() as i32;
                    let sy = ((y as f64 + 0.5 - oy) / scale).floor() as i32;
                    self.pixels[y * self.width + x] =
                        if sx >= 0 && sy >= 0 && sx < sw as i32 && sy < sh as i32 {
                            self.retro.pixels[(sy as usize + (480 - sh) / 2) * 640 + sx as usize]
                        } else {
                            0xff000000
                        };
                }
            }
            return Ok(());
        }
        let panorama = self.aspect != AspectMode::Original
            && matches!(
                name,
                "ISLETEMP.SCR"
                    | "ISLAND2.SCR"
                    | "OCEAN00.SCR"
                    | "OCEAN01.SCR"
                    | "OCEAN02.SCR"
                    | "NIGHT.SCR"
            );
        let key = format!(
            "{name}:{sw}:{sh}:{}:{}:{:?}:{:?}",
            self.width,
            self.height,
            self.aspect,
            wide.map(|w| &w.path)
        );
        if key != self.cache_key {
            if let Some(w) = wide {
                if self.loaded_wide.as_ref().map(|(p, _)| p) != Some(&w.path) {
                    let image = load_image(&w.path)?;
                    if image.width != w.width || image.height != w.height {
                        return Err(format!(
                            "{}: panorama dimensions disagree with layouts.tsv",
                            w.path.display()
                        ));
                    }
                    self.loaded_wide = Some((w.path.clone(), image));
                }
            } else {
                self.loaded_wide = None;
            }
            let background = art.backgrounds.get(name);
            for y in 0..self.height {
                for x in 0..self.width {
                    let sx = (x as f64 + 0.5 - ox) / scale;
                    let sy = (y as f64 + 0.5 - oy) / scale;
                    self.base[y * self.width + x] = if let Some((_, bg)) = &self.loaded_wide {
                        let px = (x as f64 + 0.5 - canvas[1]) / canvas[0];
                        let py = (y as f64 + 0.5 - canvas[2]) / canvas[0];
                        if px >= 0. && py >= 0. && px < bg.width as f64 && py < bg.height as f64 {
                            // Avoid a normalized divide/multiply round trip: at
                            // integer source coordinates it can select the
                            // preceding texel and disturb registered joins.
                            bg.pixels[py as usize * bg.width + px as usize]
                        } else {
                            0xff000000
                        }
                    } else if let Some(bg) = background {
                        let inside = sx >= 0. && sy >= 0. && sx < sw as f64 && sy < sh as f64;
                        if inside {
                            sample_at(
                                bg,
                                sx * bg.width as f64 / sw as f64,
                                sy * bg.height as f64 / sh as f64,
                                false,
                                false,
                            )
                        } else if panorama {
                            // Extend only scenery; the logical stage never stretches or moves.
                            let edge = if name == "ISLAND2.SCR" {
                                sw as f64 - 1.
                            } else {
                                0.
                            };
                            let t = (sx.abs() % 100.) / 100.;
                            let mirrored = if t < 0.5 { t * 2. } else { 2. - t * 2. };
                            let tx = if edge == 0. {
                                mirrored * 40.
                            } else {
                                edge - mirrored * 40.
                            };
                            sample_at(
                                bg,
                                tx * bg.width as f64 / sw as f64,
                                sy.clamp(0., sh as f64 - 0.001) * bg.height as f64 / sh as f64,
                                false,
                                false,
                            )
                        } else {
                            0xff000000
                        }
                    } else {
                        0xff000000
                    };
                }
            }
            self.cache_key = key;
        }
        self.pixels.copy_from_slice(&self.base);
        let palette: [u32; 256] = std::array::from_fn(|i| {
            let [r, g, b] = original.palette[i];
            0xff000000 | ((r as u32) << 16) | ((g as u32) << 8) | b as u32
        });
        // Resolve each stable source once. The per-output-pixel loop only uses
        // indexed lookups; a sprite can survive any number of stored frames.
        let sources: Vec<_> = player
            .sources
            .iter()
            .map(|source| match source {
                PixelSource::Background(n) => art
                    .backgrounds
                    .get(n)
                    .map(|image| {
                        let (w, h) = original
                            .backgrounds
                            .get(n)
                            .map(|b| (b.width, b.height))
                            .unwrap_or((sw as u32, sh as u32));
                        (
                            image,
                            0.,
                            0.,
                            image.width as f64 / w as f64,
                            image.height as f64 / h as f64,
                            false,
                            false,
                            n == name,
                        )
                    })
                    .map(Source::Texture)
                    .unwrap_or(Source::Missing),
                PixelSource::Sprite(draw) => art
                    .sprites
                    .get(&draw.name)
                    .and_then(|frames| frames.get(draw.frame))
                    .map(|image| {
                        (
                            image,
                            draw.x as f64,
                            draw.y as f64,
                            image.width as f64 / draw.width.max(1) as f64,
                            image.height as f64 / draw.height.max(1) as f64,
                            draw.flip_x,
                            draw.flip_y,
                            false,
                        )
                    })
                    .map(Source::Texture)
                    .unwrap_or(Source::Missing),
                PixelSource::Palette(index) => Source::Color(palette[*index as usize]),
                PixelSource::Layer { top, under } => Source::Layer {
                    top: *top,
                    under: *under,
                },
            })
            .collect();
        let xs: Vec<_> = (0..self.width)
            .map(|x| (x as f64 + 0.5 - ox) / scale)
            .collect();
        // Most of the screen is unchanged scenery. Visit logical pixels first,
        // skipping their entire output footprint when the cached backdrop owns
        // them. This keeps 4K composition proportional to the moving artwork.
        let xb: Vec<_> = (0..=sw)
            .map(|x| {
                (ox + x as f64 * scale - 0.5)
                    .ceil()
                    .clamp(0., self.width as f64) as usize
            })
            .collect();
        let yb: Vec<_> = (0..=sh)
            .map(|y| {
                (oy + y as f64 * scale - 0.5)
                    .ceil()
                    .clamp(0., self.height as f64) as usize
            })
            .collect();
        for sy in 0..sh {
            for sx in 0..sw {
                let p = sy * 640 + sx;
                let fallback = palette[player.pixels[p] as usize];
                let origin = player.origins[p];
                let source = if origin == 0 {
                    None
                } else {
                    sources.get(origin as usize - 1)
                };
                if let Some(Source::Texture((_, _, _, _, _, _, _, true))) = source {
                    continue;
                }
                for y in yb[sy]..yb[sy + 1] {
                    if let Some(Source::Texture((image, x0, y0, iw, ih, fx, fy, _))) = source {
                        let v = ((y as f64 + 0.5 - oy) / scale - y0) * ih;
                        for (x, &px) in xs.iter().enumerate().take(xb[sx + 1]).skip(xb[sx]) {
                            let u = (px - x0) * iw;
                            let c = sample_at(image, u, v, *fx, *fy);
                            // Default enhancement keeps original coverage. Custom
                            // partial-alpha cells instead use layered composition.
                            self.pixels[y * self.width + x] = blend(c, fallback, c >> 24);
                        }
                    } else if matches!(source, Some(Source::Layer { .. })) {
                        let py = (y as f64 + 0.5 - oy) / scale;
                        for (x, &px) in xs.iter().enumerate().take(xb[sx + 1]).skip(xb[sx]) {
                            self.pixels[y * self.width + x] =
                                composite(&sources, origin, px, py, fallback);
                        }
                    } else {
                        let color = match source {
                            Some(Source::Color(c)) => *c,
                            _ => fallback,
                        };
                        self.pixels[y * self.width + xb[sx]..y * self.width + xb[sx + 1]]
                            .fill(color);
                    }
                }
            }
        }
        if panorama {
            // The VM retains side provenance through the same saves/restores as
            // the central stage. Never reconstruct it from the current draw list.
            let (canvas_left, canvas_right) = wide.map_or((0, self.width), |w| {
                (
                    (canvas[1] - 0.5).ceil().clamp(0., self.width as f64) as usize,
                    (canvas[1] + w.width as f64 * canvas[0] - 0.5)
                        .ceil()
                        .clamp(0., self.width as f64) as usize,
                )
            });
            for (&(sy, sx), &origin) in &player.margin_origins {
                if sy < 0 || sy >= sh as i32 {
                    continue;
                }
                let left = ((ox + sx as f64 * scale - 0.5)
                    .ceil()
                    .clamp(0., self.width as f64) as usize)
                    .max(canvas_left);
                let right = ((ox + (sx + 1) as f64 * scale - 0.5)
                    .ceil()
                    .clamp(0., self.width as f64) as usize)
                    .min(canvas_right);
                if left >= right {
                    continue;
                }
                for y in yb[sy as usize]..yb[sy as usize + 1] {
                    let py = (y as f64 + 0.5 - oy) / scale;
                    for (x, &px) in xs.iter().enumerate().take(right).skip(left) {
                        let p = y * self.width + x;
                        self.pixels[p] = composite(&sources, origin, px, py, self.base[p]);
                    }
                }
            }
        }
        Ok(())
    }
}
// Iterative front-to-back composition avoids recursion even for persistent
// captures containing many translucent draws. Source IDs form an acyclic graph.
fn leaf(source: Option<&Source<'_>>, x: f64, y: f64, fallback: u32) -> u32 {
    match source {
        Some(Source::Texture((image, x0, y0, iw, ih, fx, fy, _))) => {
            sample_at(image, (x - x0) * iw, (y - y0) * ih, *fx, *fy)
        }
        Some(Source::Color(c)) => *c,
        _ => fallback,
    }
}
fn composite(sources: &[Source<'_>], mut id: u32, x: f64, y: f64, fallback: u32) -> u32 {
    let mut remaining = 255u32;
    let mut sum = [0u32; 3];
    loop {
        let source = id.checked_sub(1).and_then(|i| sources.get(i as usize));
        let (color, next) = match source {
            Some(Source::Layer { top, under }) => (
                leaf(sources.get(*top as usize - 1), x, y, fallback),
                Some(*under),
            ),
            _ => {
                let color = leaf(source, x, y, fallback);
                (blend(color, fallback, color >> 24), None)
            }
        };
        let weight = (remaining * (color >> 24) + 127) / 255;
        for (j, shift) in [16, 8, 0].into_iter().enumerate() {
            sum[j] += ((color >> shift) & 255) * weight;
        }
        remaining -= weight;
        if remaining == 0 {
            break;
        }
        if let Some(next) = next {
            id = next;
        } else {
            for (j, shift) in [16, 8, 0].into_iter().enumerate() {
                sum[j] += ((fallback >> shift) & 255) * remaining;
            }
            break;
        }
    }
    0xff000000
        | (((sum[0] + 127) / 255) << 16)
        | (((sum[1] + 127) / 255) << 8)
        | ((sum[2] + 127) / 255)
}
fn sample_at(image: &RgbaImage, u: f64, v: f64, flip_x: bool, flip_y: bool) -> u32 {
    // Source-pixel coordinates keep the sampling phase independent of canvas
    // padding. Flip the selected texel, including exact integer boundaries.
    let mut x = (u as usize).min(image.width - 1);
    let mut y = (v as usize).min(image.height - 1);
    if flip_x {
        x = image.width - 1 - x;
    }
    if flip_y {
        y = image.height - 1 - y;
    }
    image.pixels[y * image.width + x]
}
fn blend(src: u32, dst: u32, a: u32) -> u32 {
    if a == 255 {
        return src;
    }
    let mut out = 0xff000000;
    for shift in [0, 8, 16] {
        out |= ((((src >> shift) & 255) * a + ((dst >> shift) & 255) * (255 - a) + 127) / 255)
            << shift;
    }
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        animation::{Clip, Command, Frame},
        resources::Image,
    };
    use std::collections::BTreeMap;
    fn op(opcode: u16, args: &[i16], text: &str) -> Command {
        Command {
            opcode,
            args: args.to_vec(),
            text: text.into(),
        }
    }
    fn fixture() -> (AssetStore, RemasterAssets) {
        let mut palette = [[0; 3]; 256];
        palette[1] = [0, 0, 255];
        palette[2] = [255, 0, 0];
        palette[3] = [0, 255, 0];
        let original = AssetStore {
            palette,
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
            raw: BTreeMap::new(),
        };
        let art = RemasterAssets {
            remade_frames: 0,
            widescreen: BTreeMap::new(),
            backgrounds: BTreeMap::from([(
                "TEST.SCR".into(),
                RgbaImage {
                    width: 1,
                    height: 1,
                    pixels: vec![0xff0000ff],
                },
            )]),
            sprites: BTreeMap::from([(
                "ACTOR.BMP".into(),
                vec![RgbaImage {
                    width: 3,
                    height: 1,
                    pixels: vec![0xffff0000, 0, 0xff00ff00],
                }],
            )]),
        };
        (original, art)
    }
    fn render(commands: Vec<Command>, width: usize, height: usize) -> (Player, HdPresentation) {
        let (original, art) = fixture();
        let clip = Clip {
            name: "test".into(),
            labels: BTreeMap::new(),
            frames: vec![Frame {
                commands,
                delay_ticks: 1,
            }],
        };
        let mut player = Player::new();
        player.step(&clip, &original);
        let mut renderer = HdPresentation::new(width, height, &original.palette);
        renderer
            .update(&player, &original, &art, true, ColorMode::Classic)
            .unwrap();
        (player, renderer)
    }
    fn setup() -> Vec<Command> {
        vec![op(0xf010, &[], "TEST.SCR"), op(0xf020, &[], "ACTOR.BMP")]
    }
    fn custom_fixture() -> (AssetStore, RemasterAssets) {
        let (original, mut art) = fixture();
        art.sprites.get_mut("ACTOR.BMP").unwrap()[0] = RgbaImage {
            width: 12,
            height: 4,
            // Translucent red, new opaque yellow coverage, removed green pixel.
            pixels: (0..48)
                .map(|i| [0x80ff0000, 0xffffff00, 0][(i % 12) / 4])
                .collect(),
        };
        (original, art)
    }
    #[test]
    fn custom_alpha_uses_underlay_and_changes_coverage_without_changing_retro() {
        let (original, art) = custom_fixture();
        let mut ops = setup();
        ops.extend([
            op(0x2000, &[3, 3], ""),
            op(0xa000, &[10, 10], ""),
            op(0xa500, &[10, 10, 0, 0], ""),
        ]);
        let clip = Clip {
            name: "alpha".into(),
            labels: BTreeMap::new(),
            frames: vec![Frame {
                commands: ops,
                delay_ticks: 1,
            }],
        };
        let mut custom = Player::new();
        custom.set_sprite_coverage(art.sprite_coverage());
        // Scene changes must keep the selected pack's coverage rules.
        custom.reset();
        let mut classic = Player::new();
        custom.step(&clip, &original);
        classic.step(&clip, &original);
        assert_eq!(custom.pixels, classic.pixels);
        let mut renderer = HdPresentation::new(2560, 1400, &original.palette);
        renderer
            .update(&custom, &original, &art, true, ColorMode::Classic)
            .unwrap();
        for y in 40..44 {
            assert_eq!(
                &renderer.pixels[y * 2560 + 40..y * 2560 + 52],
                &[
                    0xff807f00, 0xff807f00, 0xff807f00, 0xff807f00, 0xffffff00, 0xffffff00,
                    0xffffff00, 0xffffff00, 0xff0000ff, 0xff0000ff, 0xff0000ff, 0xff0000ff,
                ]
            );
        }
    }
    #[test]
    fn custom_coverage_survives_opaque_stored_buffer_restore() {
        let (mut original, art) = custom_fixture();
        original
            .backgrounds
            .get_mut("TEST.SCR")
            .unwrap()
            .pixels
            .fill(0);
        let mut art = art;
        art.backgrounds
            .get_mut("TEST.SCR")
            .unwrap()
            .pixels
            .fill(0xff000000);
        let mut ops = setup();
        ops.extend([
            op(0xa500, &[10, 10, 0, 0], ""),
            op(0x4200, &[10, 10, 3, 1], ""),
            op(0x2000, &[3, 3], ""),
            op(0xa100, &[10, 10, 3, 1], ""),
            op(0xb600, &[10, 10, 3, 1, 2, 1], ""),
            op(0x0020, &[], ""),
        ]);
        let clip = Clip {
            name: "save".into(),
            labels: BTreeMap::new(),
            frames: vec![
                Frame {
                    commands: ops,
                    delay_ticks: 1,
                },
                Frame {
                    commands: vec![],
                    delay_ticks: 1,
                },
            ],
        };
        let mut player = Player::new();
        player.set_sprite_coverage(art.sprite_coverage());
        player.step(&clip, &original);
        player.step(&clip, &original);
        assert!(player.draws.is_empty());
        let mut renderer = HdPresentation::new(640, 350, &original.palette);
        renderer
            .update(&player, &original, &art, true, ColorMode::Classic)
            .unwrap();
        assert_eq!(
            &renderer.pixels[6410..6413],
            &[0xff800000, 0xffffff00, 0xff000000]
        );
    }
    #[test]
    fn repainting_a_saved_translucent_pose_does_not_change_its_opacity() {
        let (original, art) = custom_fixture();
        let mut first = setup();
        first.extend([
            op(0xa500, &[10, 10, 0, 0], ""),
            op(0x4200, &[10, 10, 3, 1], ""),
        ]);
        let clip = Clip {
            name: "repaint".into(),
            labels: BTreeMap::new(),
            frames: vec![
                Frame {
                    commands: first,
                    delay_ticks: 1,
                },
                Frame {
                    commands: vec![op(0xa500, &[10, 10, 0, 0], "")],
                    delay_ticks: 1,
                },
            ],
        };
        let mut player = Player::new();
        player.set_sprite_coverage(art.sprite_coverage());
        let mut renderer = HdPresentation::new(640, 350, &original.palette);
        player.step(&clip, &original);
        renderer
            .update(&player, &original, &art, true, ColorMode::Classic)
            .unwrap();
        let first = renderer.pixels.clone();
        let indexed = player.pixels.clone();
        player.step(&clip, &original);
        renderer
            .update(&player, &original, &art, true, ColorMode::Classic)
            .unwrap();
        assert_eq!(player.pixels, indexed);
        assert_eq!(
            renderer.pixels, first,
            "holding the same pose must not accumulate alpha"
        );
    }
    #[test]
    fn changing_frame_padding_cannot_shift_high_resolution_sampling() {
        let (mut original, mut art) = fixture();
        original.sprites.insert(
            "ACTOR.BMP".into(),
            [24, 40]
                .into_iter()
                .map(|width| Image {
                    width,
                    height: 1,
                    pixels: (0..width).map(|x| if x < 8 { 2 } else { 0 }).collect(),
                })
                .collect(),
        );
        art.sprites.insert(
            "ACTOR.BMP".into(),
            [24, 40]
                .into_iter()
                .map(|width| RgbaImage {
                    width: width * 4,
                    height: 4,
                    pixels: (0..width * 16)
                        .map(|i| {
                            let x = i % (width * 4);
                            if x < 32 {
                                if x % 2 == 0 {
                                    0xff000000
                                } else {
                                    0xffffffff
                                }
                            } else {
                                0
                            }
                        })
                        .collect(),
                })
                .collect(),
        );
        let mut first = setup();
        first.push(op(0xa500, &[10, 10, 0, 0], ""));
        let clip = Clip {
            name: "padding".into(),
            labels: BTreeMap::new(),
            frames: vec![
                Frame {
                    commands: first,
                    delay_ticks: 1,
                },
                Frame {
                    commands: vec![op(0xa500, &[10, 10, 1, 0], "")],
                    delay_ticks: 1,
                },
            ],
        };
        let mut p = Player::new();
        p.set_sprite_coverage(art.sprite_coverage());
        let mut renderer = HdPresentation::new(1280, 700, &original.palette);
        p.step(&clip, &original);
        renderer
            .update(&p, &original, &art, true, ColorMode::Classic)
            .unwrap();
        let first = renderer.pixels.clone();
        p.step(&clip, &original);
        renderer
            .update(&p, &original, &art, true, ColorMode::Classic)
            .unwrap();
        assert!(
            renderer.pixels == first,
            "frame width must not change the sampling phase"
        );
    }
    #[test]
    fn mirrored_sampling_matches_a_physically_flipped_rgba_frame() {
        let (mut original, mut art) = custom_fixture();
        let image = &mut art.sprites.get_mut("ACTOR.BMP").unwrap()[0];
        image.pixels = (0u32..48)
            .map(|i| 0xff000000 | (((i % 12) * 17) << 16) | (((i / 12) * 60) << 8) | (i % 12))
            .collect();
        let mut commands = setup();
        commands.push(op(0xa530, &[10, 10, 0, 0], ""));
        let mut clip = Clip {
            name: "mirror".into(),
            labels: BTreeMap::new(),
            frames: vec![Frame {
                commands,
                delay_ticks: 1,
            }],
        };
        let mut p = Player::new();
        p.set_sprite_coverage(art.sprite_coverage());
        p.step(&clip, &original);
        let mut renderer = HdPresentation::new(1280, 700, &original.palette);
        renderer
            .update(&p, &original, &art, true, ColorMode::Classic)
            .unwrap();
        let mirrored = renderer.pixels.clone();
        original.sprites.get_mut("ACTOR.BMP").unwrap()[0]
            .pixels
            .reverse();
        art.sprites.get_mut("ACTOR.BMP").unwrap()[0]
            .pixels
            .reverse();
        clip.frames[0].commands.last_mut().unwrap().opcode = 0xa500;
        let mut p = Player::new();
        p.set_sprite_coverage(art.sprite_coverage());
        p.step(&clip, &original);
        renderer
            .update(&p, &original, &art, true, ColorMode::Classic)
            .unwrap();
        assert!(
            renderer.pixels == mirrored,
            "mirroring must preserve the chosen high-resolution sample"
        );
    }
    #[test]
    fn duplicate_pose_resources_repaint_without_alpha_or_shading_pulses() {
        let (mut original, mut art) = custom_fixture();
        original
            .sprites
            .insert("COPY.BMP".into(), original.sprites["ACTOR.BMP"].clone());
        let mut copy = art.sprites["ACTOR.BMP"].clone();
        // Independent remaster processing may slightly shade duplicate frames
        // differently. They still represent one held source pose.
        copy[0].pixels[0] = 0x81fe0000;
        art.sprites.insert("COPY.BMP".into(), copy);
        let mut ops = setup();
        ops.extend([op(0xa500, &[10, 10, 0, 0], ""), op(0x0020, &[], "")]);
        let clip = Clip {
            name: "copied pose".into(),
            labels: BTreeMap::new(),
            frames: vec![
                Frame {
                    commands: ops,
                    delay_ticks: 1,
                },
                Frame {
                    commands: vec![op(0xf020, &[], "COPY.BMP"), op(0xa500, &[10, 10, 0, 0], "")],
                    delay_ticks: 1,
                },
            ],
        };
        let mut p = Player::new();
        p.set_sprite_coverage(art.sprite_coverage());
        let mut renderer = HdPresentation::new(2560, 1400, &original.palette);
        p.step(&clip, &original);
        renderer
            .update(&p, &original, &art, true, ColorMode::Classic)
            .unwrap();
        let first = renderer.pixels.clone();
        p.step(&clip, &original);
        renderer
            .update(&p, &original, &art, true, ColorMode::Classic)
            .unwrap();
        assert_eq!(renderer.pixels, first);
    }
    #[test]
    fn repainting_overlapping_saved_parts_preserves_color_and_painter_order() {
        let (original, mut art) = custom_fixture();
        art.sprites.get_mut("ACTOR.BMP").unwrap()[0]
            .pixels
            .fill(0x80ff0000);
        let draws = vec![
            op(0xa500, &[10, 10, 0, 0], ""),
            op(0xa500, &[11, 10, 0, 0], ""),
        ];
        let mut first = setup();
        first.extend(draws.clone());
        first.push(op(0x4200, &[10, 10, 4, 1], ""));
        let clip = Clip {
            name: "overlapping parts".into(),
            labels: BTreeMap::new(),
            frames: vec![
                Frame {
                    commands: first,
                    delay_ticks: 1,
                },
                Frame {
                    commands: draws,
                    delay_ticks: 1,
                },
            ],
        };
        let mut p = Player::new();
        p.set_sprite_coverage(art.sprite_coverage());
        let mut renderer = HdPresentation::new(640, 350, &original.palette);
        p.step(&clip, &original);
        renderer
            .update(&p, &original, &art, true, ColorMode::Classic)
            .unwrap();
        let first = renderer.pixels.clone();
        assert_eq!(first[6411], 0xffc0003f);
        p.step(&clip, &original);
        renderer
            .update(&p, &original, &art, true, ColorMode::Classic)
            .unwrap();
        assert!(
            renderer.pixels == first,
            "overlapping held pieces must retain two layers, not four"
        );
    }
    #[test]
    fn real_bathing_scene_holds_its_remastered_pose_exactly() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let original = AssetStore::load(root.join("original")).unwrap();
        // Use generated fallback RGBA with partial edges, so this regression
        // does not depend on any externally supplied remaster pack.
        let mut art = RemasterAssets::load_with_painted_art(
            &root.join("assets/remastered"),
            &original,
            false,
        )
        .unwrap();
        for name in ["MJBATH.BMP", "GJGULL2.BMP"] {
            for image in art.sprites.get_mut(name).unwrap() {
                for color in &mut image.pixels {
                    if *color >> 24 != 0 {
                        *color = (*color & 0x00ffffff) | 0x80000000;
                    }
                }
            }
        }
        let raw: Vec<_> = original
            .raw
            .iter()
            .filter(|(n, _)| n.ends_with(".TTM"))
            .map(|(n, b)| Clip::parse(n, b).unwrap())
            .collect();
        let clips = crate::scheduler::build_scene_clips(&original, &raw).unwrap();
        let clip = clips.iter().find(|c| c.name == "MJBATH.TTM").unwrap();
        let mut player = Player::new();
        player.set_sprite_coverage(art.sprite_coverage());
        let mut renderer = HdPresentation::new(640, 350, &original.palette);
        player.step(clip, &original);
        renderer
            .update(&player, &original, &art, true, ColorMode::Classic)
            .unwrap();
        let indexed = player.pixels.clone();
        let first = renderer.pixels.clone();
        let draws = player.draws.clone();
        player.step(clip, &original);
        renderer
            .update(&player, &original, &art, true, ColorMode::Classic)
            .unwrap();
        assert_eq!(player.pixels, indexed);
        assert_eq!(player.draws, draws);
        assert_eq!(renderer.pixels, first);
    }
    #[test]
    fn custom_translucent_overlap_obeys_order_flip_and_clip() {
        let (original, art) = custom_fixture();
        let mut ops = setup();
        ops.extend([
            op(0xa500, &[10, 10, 0, 0], ""),
            op(0x4000, &[11, 10, 12, 10], ""),
            op(0xa520, &[9, 10, 0, 0], ""),
        ]);
        let clip = Clip {
            name: "overlap".into(),
            labels: BTreeMap::new(),
            frames: vec![Frame {
                commands: ops,
                delay_ticks: 1,
            }],
        };
        let mut player = Player::new();
        player.set_sprite_coverage(art.sprite_coverage());
        player.step(&clip, &original);
        let mut renderer = HdPresentation::new(640, 350, &original.palette);
        renderer
            .update(&player, &original, &art, true, ColorMode::Classic)
            .unwrap();
        assert_eq!(
            &renderer.pixels[6410..6413],
            &[0xff80007f, 0xffff7f00, 0xff0000ff]
        );
    }
    #[test]
    fn native_restore_and_primitives_obey_order_even_over_sprite_bounds() {
        let mut ops = setup();
        ops.extend([
            op(0x4210, &[10, 10, 3, 1], ""),
            op(0xa500, &[10, 10, 0, 0], ""),
            op(0x2000, &[0, 0], ""),
            op(0xa000, &[10, 10], ""),
        ]);
        let (_, renderer) = render(ops.clone(), 640, 350);
        assert_eq!(
            &renderer.pixels[6410..6413],
            &[0xff000000, 0xff0000ff, 0xff00ff00]
        );
        ops.push(op(0xa600, &[0], ""));
        let (_, renderer) = render(ops.clone(), 640, 350);
        assert_eq!(
            &renderer.pixels[6410..6413],
            &[0xff000000, 0xff0000ff, 0xff00ff00],
            "the native legacy get/put stub must not erase the actor"
        );
        ops.push(op(0xb600, &[10, 10, 3, 1, 2, 1], ""));
        let (_, renderer) = render(ops, 640, 350);
        assert_eq!(&renderer.pixels[6410..6413], &[0xff0000ff; 3]);
    }
    #[test]
    fn captured_actor_retains_hd_identity_after_draw_list_is_empty() {
        let (original, art) = fixture();
        let mut ops = setup();
        ops.extend([op(0xa500, &[10, 10, 0, 0], ""), op(0x0020, &[], "")]);
        let clip = Clip {
            name: "store".into(),
            labels: BTreeMap::new(),
            frames: vec![
                Frame {
                    commands: ops,
                    delay_ticks: 1,
                },
                Frame {
                    commands: vec![],
                    delay_ticks: 1,
                },
            ],
        };
        let mut player = Player::new();
        player.step(&clip, &original);
        player.step(&clip, &original);
        assert!(player.draws.is_empty());
        let mut renderer = HdPresentation::new(640, 350, &original.palette);
        renderer
            .update(&player, &original, &art, true, ColorMode::Classic)
            .unwrap();
        assert_eq!(
            &renderer.pixels[6410..6413],
            &[0xffff0000, 0xff0000ff, 0xff00ff00]
        );
    }
    #[test]
    fn flip_clip_and_transparent_pixels_match_indexed_renderer() {
        let mut ops = setup();
        ops.extend([
            op(0x4000, &[10, 10, 11, 10], ""),
            op(0xa520, &[10, 10, 0, 0], ""),
        ]);
        let (player, renderer) = render(ops, 640, 350);
        assert_eq!(&player.pixels[6410..6413], &[3, 1, 1]);
        assert_eq!(
            &renderer.pixels[6410..6413],
            &[0xff00ff00, 0xff0000ff, 0xff0000ff]
        );
    }
    #[test]
    fn stage_anchor_is_uniform_at_widescreen_resolutions() {
        for (w, h) in [(1920, 1080), (3440, 1440), (3840, 2160)] {
            let mut ops = setup();
            ops.push(op(0xa500, &[320, 200, 0, 0], ""));
            let (_, r) = render(ops, w, h);
            let s = (w as f64 / 640.).min(h as f64 / 350.);
            let x = (w as f64 / 2. + s / 2.) as usize;
            let y = ((h as f64 - 350. * s) / 2. + 200.5 * s) as usize;
            assert_eq!(r.pixels[y * w + x], 0xffff0000, "{w}x{h}");
        }
    }
    #[test]
    fn original_aspect_letterboxes_instead_of_repeating_scenery() {
        let (mut original, mut art) = fixture();
        let bg = original.backgrounds.remove("TEST.SCR").unwrap();
        original.backgrounds.insert("ISLETEMP.SCR".into(), bg);
        let bg = art.backgrounds.remove("TEST.SCR").unwrap();
        art.backgrounds.insert("ISLETEMP.SCR".into(), bg);
        let clip = Clip {
            name: "aspect".into(),
            labels: BTreeMap::new(),
            frames: vec![Frame {
                delay_ticks: 1,
                commands: vec![op(0xf010, &[], "ISLETEMP.SCR")],
            }],
        };
        let mut player = Player::new();
        player.step(&clip, &original);
        let mut renderer = HdPresentation::new(1290, 540, &original.palette);
        renderer
            .update(&player, &original, &art, true, ColorMode::Classic)
            .unwrap();
        assert_eq!(renderer.pixels[270 * 1290], 0xff0000ff);
        renderer.set_aspect(AspectMode::Original);
        renderer
            .update(&player, &original, &art, true, ColorMode::Classic)
            .unwrap();
        assert_eq!(renderer.pixels[270 * 1290], 0xff000000);
        assert_eq!(renderer.pixels[270 * 1290 + 645], 0xff0000ff);
    }
    #[test]
    fn panoramic_sprites_cross_both_stage_edges_and_survive_stored_holds() {
        let (mut original, mut art) = custom_fixture();
        original.backgrounds.insert(
            "ISLETEMP.SCR".into(),
            original.backgrounds["TEST.SCR"].clone(),
        );
        art.backgrounds
            .insert("ISLETEMP.SCR".into(), art.backgrounds["TEST.SCR"].clone());
        let mut ops = setup();
        ops[0].text = "ISLETEMP.SCR".into();
        ops.extend([
            op(0xa500, &[-2, 10, 0, 0], ""),
            op(0xa500, &[639, 10, 0, 0], ""),
            op(0x4200, &[0, 10, 640, 1], ""),
        ]);
        let clip = Clip {
            name: "side-hold".into(),
            labels: BTreeMap::new(),
            frames: vec![
                Frame {
                    commands: ops,
                    delay_ticks: 1,
                },
                Frame {
                    commands: vec![],
                    delay_ticks: 1,
                },
            ],
        };
        let mut player = Player::new();
        player.set_sprite_coverage(art.sprite_coverage());
        let mut renderer = HdPresentation::new(1280, 350, &original.palette);
        for tick in 0..2 {
            player.step(&clip, &original);
            renderer
                .update(&player, &original, &art, true, ColorMode::Classic)
                .unwrap();
            assert_eq!(renderer.stage_transform(), [1., 320., 0.]);
            assert_eq!(
                renderer.pixels[10 * 1280 + 318],
                0xff80007f,
                "left alpha at tick {tick}"
            );
            assert_eq!(
                renderer.pixels[10 * 1280 + 319],
                0xffffff00,
                "new coverage over left margin"
            );
            assert_eq!(renderer.pixels[10 * 1280 + 960], 0xffffff00, "right margin");
            assert_eq!(
                renderer.pixels[10 * 1280 + 961],
                0xff0000ff,
                "removed coverage reveals panorama"
            );
        }
        assert!(player.draws.is_empty());
        renderer.set_aspect(AspectMode::Original);
        renderer
            .update(&player, &original, &art, true, ColorMode::Classic)
            .unwrap();
        assert_eq!(renderer.pixels[10 * 1280 + 319], 0xff000000);
        renderer.set_aspect(AspectMode::Auto);
        renderer
            .update(&player, &original, &art, false, ColorMode::Classic)
            .unwrap();
        assert_eq!(
            renderer.pixels[10 * 1280 + 960],
            0xff000000,
            "retro keeps stage framing"
        );
    }

    #[test]
    fn side_pixels_obey_authored_interior_masks_and_flip() {
        let (mut original, mut art) = fixture();
        original.backgrounds.insert(
            "ISLETEMP.SCR".into(),
            original.backgrounds["TEST.SCR"].clone(),
        );
        art.backgrounds
            .insert("ISLETEMP.SCR".into(), art.backgrounds["TEST.SCR"].clone());
        let mut ops = setup();
        ops[0].text = "ISLETEMP.SCR".into();
        ops.extend([
            op(0xa520, &[-2, 10, 0, 0], ""),
            op(0x4000, &[1, 0, 638, 349], ""),
            op(0xa500, &[-2, 20, 0, 0], ""),
            op(0xa500, &[639, 20, 0, 0], ""),
        ]);
        let clip = Clip {
            name: "side-mask".into(),
            labels: BTreeMap::new(),
            frames: vec![
                Frame {
                    commands: ops,
                    delay_ticks: 1,
                },
                Frame {
                    commands: vec![],
                    delay_ticks: 1,
                },
            ],
        };
        let mut player = Player::new();
        player.step(&clip, &original);
        let mut renderer = HdPresentation::new(1280, 350, &original.palette);
        renderer
            .update(&player, &original, &art, true, ColorMode::Classic)
            .unwrap();
        assert_eq!(
            renderer.pixels[10 * 1280 + 318],
            0xff00ff00,
            "flipped sprite across left edge"
        );
        assert_eq!(
            renderer.pixels[20 * 1280 + 318],
            0xff0000ff,
            "interior left clip remains authored"
        );
        assert_eq!(
            renderer.pixels[20 * 1280 + 961],
            0xff0000ff,
            "interior right clip remains authored"
        );
        player.step(&clip, &original);
        assert!(
            player.margin_origins.is_empty(),
            "moving side sprites must not leave trails"
        );
        renderer
            .update(&player, &original, &art, true, ColorMode::Classic)
            .unwrap();
        assert_eq!(renderer.pixels[10 * 1280 + 318], 0xff0000ff);
    }

    #[test]
    fn panoramic_capture_restore_keeps_saved_side_layers_and_removes_new_ones() {
        let (mut original, mut art) = custom_fixture();
        original.backgrounds.insert(
            "ISLETEMP.SCR".into(),
            original.backgrounds["TEST.SCR"].clone(),
        );
        art.backgrounds
            .insert("ISLETEMP.SCR".into(), art.backgrounds["TEST.SCR"].clone());
        let mut ops = setup();
        ops[0].text = "ISLETEMP.SCR".into();
        ops.extend([
            op(0xa500, &[-2, 10, 0, 0], ""),
            op(0xa050, &[0, 10, 640, 1], ""),
            op(0xa500, &[640, 10, 0, 0], ""),
            op(0x2000, &[3, 3], ""),
            op(0xa000, &[-2, 10], ""),
            op(0xa060, &[42, 42, 1, 1], ""),
            op(0x0020, &[], ""),
        ]);
        let clip = Clip {
            name: "side-capture".into(),
            labels: BTreeMap::new(),
            frames: vec![
                Frame {
                    commands: ops,
                    delay_ticks: 1,
                },
                Frame {
                    commands: vec![],
                    delay_ticks: 1,
                },
            ],
        };
        let mut player = Player::new();
        player.set_sprite_coverage(art.sprite_coverage());
        player.step(&clip, &original);
        player.step(&clip, &original);
        let mut renderer = HdPresentation::new(1280, 350, &original.palette);
        renderer
            .update(&player, &original, &art, true, ColorMode::Classic)
            .unwrap();
        assert_eq!(
            renderer.pixels[10 * 1280 + 318],
            0xff80007f,
            "capture retains translucent left actor"
        );
        assert_eq!(
            renderer.pixels[10 * 1280 + 960],
            0xff0000ff,
            "restore clears later right actor"
        );
    }

    #[test]
    fn core_background_cache_changes_with_the_selected_art_pack() {
        let (original, mut art) = fixture();
        let clip = Clip {
            name: "core-style".into(),
            labels: BTreeMap::new(),
            frames: vec![Frame {
                commands: vec![op(0xf010, &[], "TEST.SCR")],
                delay_ticks: 1,
            }],
        };
        let mut player = Player::new();
        player.step(&clip, &original);
        let mut renderer = HdPresentation::new(640, 350, &original.palette);
        renderer.set_aspect(AspectMode::Original);
        renderer
            .update(&player, &original, &art, true, ColorMode::Classic)
            .unwrap();
        assert_eq!(renderer.pixels[0], 0xff0000ff);
        art.backgrounds
            .get_mut("TEST.SCR")
            .unwrap()
            .pixels
            .fill(0xffff0000);
        renderer.invalidate_art();
        renderer
            .update(&player, &original, &art, true, ColorMode::Classic)
            .unwrap();
        assert_eq!(
            renderer.pixels[0], 0xffff0000,
            "core aspect must use the newly selected pack"
        );
    }

    #[test]
    fn panorama_registration_controls_sprite_clipping_resize_and_aspect_toggle() {
        use crate::remaster::WideBackground;
        let (mut original, mut art) = fixture();
        original.backgrounds.insert(
            "TEST.SCR".into(),
            Image {
                width: 3,
                height: 1,
                pixels: vec![1; 3],
            },
        );
        let path = std::env::temp_dir().join(format!(
            "antics-wide-{}-{}.rgba",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut pixels = vec![0xff112233u32; 21 * 9];
        for y in 2..6 {
            pixels[y * 21 + 4..y * 21 + 16].fill(0xff0000ff);
        }
        let mut bytes = b"SAHD".to_vec();
        bytes.extend_from_slice(&21u32.to_le_bytes());
        bytes.extend_from_slice(&9u32.to_le_bytes());
        for p in pixels {
            bytes.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, p as u8, (p >> 24) as u8]);
        }
        std::fs::write(&path, bytes).unwrap();
        art.widescreen.insert(
            "TEST.SCR".into(),
            vec![WideBackground {
                aspect: AspectMode::Ratio21_9,
                width: 21,
                height: 9,
                stage: [4, 2, 12, 4],
                path: path.clone(),
            }],
        );
        let mut ops = setup();
        ops.extend([op(0x4000, &[0, 0, 1, 0], ""), op(0xa500, &[0, 0, 0, 0], "")]);
        let clip = Clip {
            name: "registered".into(),
            labels: BTreeMap::new(),
            frames: vec![Frame {
                commands: ops,
                delay_ticks: 1,
            }],
        };
        let mut player = Player::new();
        player.step(&clip, &original);
        let mut renderer = HdPresentation::new(210, 90, &original.palette);
        renderer
            .update(&player, &original, &art, true, ColorMode::Classic)
            .unwrap();
        assert_eq!(renderer.stage_transform(), [40., 40., 20.]);
        assert_eq!(renderer.pixels[0], 0xff112233);
        assert_eq!(renderer.pixels[40 * 210 + 60], 0xffff0000);
        assert_eq!(renderer.pixels[40 * 210 + 100], 0xff0000ff);
        assert_eq!(
            renderer.pixels[40 * 210 + 140],
            0xff0000ff,
            "clipped green must not appear"
        );
        renderer.resize(420, 180);
        renderer
            .update(&player, &original, &art, true, ColorMode::Classic)
            .unwrap();
        assert_eq!(renderer.stage_transform(), [80., 80., 40.]);
        assert_eq!(renderer.pixels[80 * 420 + 120], 0xffff0000);
        renderer.set_aspect(AspectMode::Original);
        renderer
            .update(&player, &original, &art, true, ColorMode::Classic)
            .unwrap();
        assert!(renderer.loaded_wide.is_none());
        assert_eq!(renderer.pixels[0], 0xff000000);
        renderer.set_aspect(AspectMode::Ratio21_9);
        renderer
            .update(&player, &original, &art, false, ColorMode::Classic)
            .unwrap();
        assert_eq!(
            renderer.pixels[0], 0xff000000,
            "retro keeps its original framing"
        );
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn all_original_frames_retain_exact_composition_through_hd_provenance() {
        let original =
            AssetStore::load(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("original"))
                .unwrap();
        let to_rgba = |im: &Image| RgbaImage {
            width: im.width as usize,
            height: im.height as usize,
            pixels: im
                .pixels
                .iter()
                .map(|&p| {
                    let [r, g, b] = original.palette[p as usize];
                    0xff000000 | ((r as u32) << 16) | ((g as u32) << 8) | b as u32
                })
                .collect(),
        };
        // Identity artwork isolates composition from optional color treatment.
        let art = RemasterAssets {
            remade_frames: 0,
            widescreen: BTreeMap::new(),
            backgrounds: original
                .backgrounds
                .iter()
                .map(|(n, i)| (n.clone(), to_rgba(i)))
                .collect(),
            sprites: original
                .sprites
                .iter()
                .map(|(n, f)| (n.clone(), f.iter().map(to_rgba).collect()))
                .collect(),
        };
        let palette: [u32; 256] = std::array::from_fn(|i| {
            let [r, g, b] = original.palette[i];
            0xff000000 | ((r as u32) << 16) | ((g as u32) << 8) | b as u32
        });
        let mut count = 0;
        for (name, bytes) in original.raw.iter().filter(|(n, _)| n.ends_with(".TTM")) {
            let clip = Clip::parse(name, bytes).unwrap();
            let mut player = Player::new();
            let mut renderer = HdPresentation::new(640, 350, &original.palette);
            while player.step(&clip, &original).is_some() {
                if renderer.width != player.width as usize
                    || renderer.height != player.height as usize
                {
                    renderer.resize(player.width as usize, player.height as usize);
                }
                renderer
                    .update(&player, &original, &art, true, ColorMode::Classic)
                    .unwrap();
                for y in 0..renderer.height {
                    for x in 0..renderer.width {
                        assert_eq!(
                            renderer.pixels[y * renderer.width + x],
                            palette[player.pixels[y * 640 + x] as usize],
                            "{name} frame {} at {x},{y}",
                            player.frame - 1
                        );
                    }
                }
                count += 1;
            }
        }
        assert_eq!(count, 6631);
    }
}
