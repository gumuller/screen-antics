//! True-color presentation of the VM's final pixel provenance.
//! Saves/restores and painter order are resolved by the VM, not guessed from
//! sprite bounding boxes. This also preserves opaque palette-zero primitives.
use crate::{
    animation::{PixelSource, Player},
    presentation::{ColorMode, Presentation},
    remaster::{RemasterAssets, RgbaImage},
    resources::AssetStore,
};

pub struct HdPresentation {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u32>,
    base: Vec<u32>,
    cache_key: String,
    retro: Presentation,
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
        }
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
    ) {
        let sw = player.width.clamp(1, 640) as usize;
        let sh = player.height.clamp(1, 480) as usize;
        let scale = (self.width as f64 / sw as f64).min(self.height as f64 / sh as f64);
        let ox = (self.width as f64 - sw as f64 * scale) / 2.;
        let oy = (self.height as f64 - sh as f64 * scale) / 2.;
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
            return;
        }
        let name = player.current_background.as_deref().unwrap_or("");
        let key = format!("{name}:{sw}:{sh}:{}:{}", self.width, self.height);
        if key != self.cache_key {
            let panorama = matches!(
                name,
                "ISLETEMP.SCR"
                    | "ISLAND2.SCR"
                    | "OCEAN00.SCR"
                    | "OCEAN01.SCR"
                    | "OCEAN02.SCR"
                    | "NIGHT.SCR"
            );
            let background = art.backgrounds.get(name);
            for y in 0..self.height {
                for x in 0..self.width {
                    let sx = (x as f64 + 0.5 - ox) / scale;
                    let sy = (y as f64 + 0.5 - oy) / scale;
                    self.base[y * self.width + x] = if let Some(bg) = background {
                        let inside = sx >= 0. && sy >= 0. && sx < sw as f64 && sy < sh as f64;
                        if inside {
                            sample(bg, sx / sw as f64, sy / sh as f64)
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
                            sample(
                                bg,
                                tx / sw as f64,
                                sy.clamp(0., sh as f64 - 0.001) / sh as f64,
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
        // Resolve each stable source once. The per-output-pixel loop only uses
        // indexed lookups; a sprite can survive any number of stored frames.
        let sources: Vec<_> = player
            .sources
            .iter()
            .map(|source| match source {
                PixelSource::Background(n) => art.backgrounds.get(n).map(|image| {
                    let (w, h) = original
                        .backgrounds
                        .get(n)
                        .map(|b| (b.width, b.height))
                        .unwrap_or((sw as u32, sh as u32));
                    (
                        image,
                        0.,
                        0.,
                        1. / w as f64,
                        1. / h as f64,
                        false,
                        false,
                        n == name,
                    )
                }),
                PixelSource::Sprite(draw) => art
                    .sprites
                    .get(&draw.name)
                    .and_then(|frames| frames.get(draw.frame))
                    .map(|image| {
                        (
                            image,
                            draw.x as f64,
                            draw.y as f64,
                            1. / draw.width.max(1) as f64,
                            1. / draw.height.max(1) as f64,
                            draw.flip_x,
                            draw.flip_y,
                            false,
                        )
                    }),
            })
            .collect();
        let xs: Vec<_> = (0..self.width)
            .map(|x| (x as f64 + 0.5 - ox) / scale)
            .collect();
        let palette: [u32; 256] = std::array::from_fn(|i| {
            let [r, g, b] = original.palette[i];
            0xff000000 | ((r as u32) << 16) | ((g as u32) << 8) | b as u32
        });
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
                    sources.get(origin as usize - 1).and_then(Option::as_ref)
                };
                if let Some((_, _, _, _, _, _, _, true)) = source {
                    continue;
                }
                for y in yb[sy]..yb[sy + 1] {
                    if let Some((image, x0, y0, iw, ih, fx, fy, _)) = source {
                        let mut v = ((y as f64 + 0.5 - oy) / scale - y0) * ih;
                        if *fy {
                            v = 1. - v;
                        }
                        for (x, &px) in xs.iter().enumerate().take(xb[sx + 1]).skip(xb[sx]) {
                            let mut u = (px - x0) * iw;
                            if *fx {
                                u = 1. - u;
                            }
                            let c = sample(image, u, v);
                            // Original coverage is authoritative: smoothing may
                            // not punch holes in bodies, props or occluders.
                            self.pixels[y * self.width + x] = blend(c, fallback, c >> 24);
                        }
                    } else {
                        self.pixels[y * self.width + xb[sx]..y * self.width + xb[sx + 1]]
                            .fill(fallback);
                    }
                }
            }
        }
    }
}
fn sample(image: &RgbaImage, u: f64, v: f64) -> u32 {
    let x = ((u * image.width as f64) as usize).min(image.width - 1);
    let y = ((v * image.height as f64) as usize).min(image.height - 1);
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
        renderer.update(&player, &original, &art, true, ColorMode::Classic);
        (player, renderer)
    }
    fn setup() -> Vec<Command> {
        vec![op(0xf010, &[], "TEST.SCR"), op(0xf020, &[], "ACTOR.BMP")]
    }
    #[test]
    fn primitives_and_restore_obey_order_even_over_sprite_bounds() {
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
        renderer.update(&player, &original, &art, true, ColorMode::Classic);
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
                renderer.update(&player, &original, &art, true, ColorMode::Classic);
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
