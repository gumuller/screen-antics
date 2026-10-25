//! HD-2D lighting in the existing native SDL compositor. Low-resolution radiance,
//! scrolling normal tiles and separable bloom feed a parallel, linear-light ACES
//! pass. The source framebuffer is never modified or fed back into the effects.
use crate::{
    animation::{Draw, PixelSource, Player},
    remaster::RemasterAssets,
};
use std::collections::BTreeMap;

const CELL: usize = 8;
const NORMAL: usize = 128;
#[derive(Clone, Copy, Default)]
struct Field {
    gain: [u16; 3],
    glow: [u16; 3],
    tables: [u32; 3],
}
#[derive(Clone, Copy)]
struct Fire {
    x: f32,
    y: f32,
}
#[derive(Clone, Copy)]
pub struct WorldLighting {
    pub horizon: f32,
    pub reflection_x: f32,
    pub night: bool,
    pub outdoors: bool,
}
pub struct Hd2dEffects {
    pub pixels: Vec<u32>,
    world: Option<WorldLighting>,
    field: Vec<Field>,
    emission: Vec<[f32; 3]>,
    scratch: Vec<[f32; 3]>,
    water: Vec<bool>,
    normals: Vec<[f32; 2]>,
    linear: [u16; 256],
    aces: Vec<u8>,
    fire_centers: BTreeMap<usize, Option<(f32, f32)>>,
    table_index: Vec<u32>,
    tables: Vec<[u8; 256]>,
}
impl Hd2dEffects {
    pub fn new() -> Self {
        let linear = std::array::from_fn(|i| ((i as f32 / 255.).powf(2.2) * 1024.).round() as u16);
        let aces = (0..8192)
            .map(|i| {
                let x = i as f32 / 1024.;
                let film = (x * (2.51 * x + 0.03) / (x * (2.43 * x + 0.59) + 0.14)).clamp(0., 1.);
                (film.powf(1. / 2.2) * 255.).round() as u8
            })
            .collect();
        let normals = (0..NORMAL * NORMAL)
            .map(|i| {
                let x = (i % NORMAL) as f32 * std::f32::consts::TAU / NORMAL as f32;
                let y = (i / NORMAL) as f32 * std::f32::consts::TAU / NORMAL as f32;
                [
                    (x * 3. + (y * 2.).sin()).sin() * 0.65 + (y * 5. + x).cos() * 0.35,
                    (y * 3. + (x * 2.).cos()).cos() * 0.65 + (x * 5. - y).sin() * 0.35,
                ]
            })
            .collect();
        Self {
            pixels: vec![],
            world: None,
            field: vec![],
            emission: vec![],
            scratch: vec![],
            water: vec![],
            normals,
            linear,
            aces,
            fire_centers: BTreeMap::new(),
            table_index: vec![u32::MAX; 1025 * 256],
            tables: Vec::new(),
        }
    }
    pub fn set_world(&mut self, world: WorldLighting) {
        self.world = Some(world);
    }
    fn normal(&self, x: f32, y: f32) -> [f32; 2] {
        let at = |v: f32| (v.floor() as i32).rem_euclid(NORMAL as i32) as usize;
        self.normals[at(y) * NORMAL + at(x)]
    }
    fn fire_center(&mut self, draw: &Draw, art: &RemasterAssets) -> Option<Fire> {
        let center = *self.fire_centers.entry(draw.frame).or_insert_with(|| {
            let image = art.sprites.get("FIRE1.BMP")?.get(draw.frame)?;
            let mut sums = [0f32; 3];
            // Only bright warm flame pixels emit; unlit wood/actor poses do not.
            for y in (0..image.height).step_by(2) {
                for x in (0..image.width).step_by(2) {
                    let c = image.pixels[y * image.width + x];
                    let r = ((c >> 16) & 255) as f32;
                    let g = ((c >> 8) & 255) as f32;
                    let b = (c & 255) as f32;
                    if c >> 24 > 128 && r > 215. && g > 90. && r > g * 1.08 && g > b * 1.4 {
                        let weight = (r - 215.) / 40.;
                        sums[0] += (x as f32 + 0.5) / image.width as f32 * weight;
                        sums[1] += (y as f32 + 0.5) / image.height as f32 * weight;
                        sums[2] += weight;
                    }
                }
            }
            (sums[2] > 1.).then(|| (sums[0] / sums[2], sums[1] / sums[2]))
        });
        center.map(|(mut x, mut y)| {
            if draw.flip_x {
                x = 1. - x;
            }
            if draw.flip_y {
                y = 1. - y;
            }
            Fire {
                x: draw.x as f32 + x * draw.width as f32,
                y: draw.y as f32 + y * draw.height as f32,
            }
        })
    }
    fn visible_fires(&mut self, player: &Player, art: &RemasterAssets) -> Vec<Fire> {
        if !player
            .sources
            .iter()
            .any(|s| matches!(s,PixelSource::Sprite(d) if d.name=="FIRE1.BMP"))
        {
            return vec![];
        }
        let mut visible = vec![false; player.sources.len() + 1];
        for &id in player.origins.iter().chain(player.margin_origins.values()) {
            visible[id as usize] = true;
        }
        for (i, s) in player.sources.iter().enumerate().rev() {
            if visible[i + 1] {
                if let PixelSource::Layer { top, under } = s {
                    visible[*top as usize] = true;
                    visible[*under as usize] = true;
                }
            }
        }
        let mut fires = vec![];
        for (i, s) in player.sources.iter().enumerate().rev() {
            if visible[i + 1] {
                if let PixelSource::Sprite(d) = s {
                    if d.name == "FIRE1.BMP" {
                        if let Some(fire) = self.fire_center(d, art) {
                            if !fires.iter().any(|f: &Fire| {
                                (f.x - fire.x).abs() < 20. && (f.y - fire.y).abs() < 20.
                            }) {
                                fires.push(fire);
                            }
                            if fires.len() == 4 {
                                break;
                            }
                        }
                    }
                }
            }
        }
        fires
    }
    pub fn render(
        &mut self,
        input: &[u32],
        width: usize,
        height: usize,
        stage: [f64; 3],
        player: &Player,
        art: &RemasterAssets,
        time: f64,
    ) -> &[u32] {
        let time = time as f32;
        let [scale, ox, oy] = stage.map(|v| v as f32);
        let (gw, gh) = (width.div_ceil(CELL), height.div_ceil(CELL));
        self.field.resize(gw * gh, Field::default());
        self.emission.resize(gw * gh, [0.; 3]);
        self.scratch.resize(gw * gh, [0.; 3]);
        self.water.resize(gw * gh, true);
        self.water.fill(true);
        let bg = player.current_background.as_deref().unwrap_or("");
        let night = self.world.map_or(bg == "NIGHT.SCR", |w| w.night);
        let ocean = bg.starts_with("OCEAN");
        let legacy_outdoors =
            night || ocean || matches!(bg, "ISLAND2.SCR" | "ISLETEMP.SCR" | "SUZBEACH.SCR");
        let outdoors = self.world.map_or(legacy_outdoors, |w| w.outdoors);
        let reflection_x = self.world.map_or(165.5, |w| w.reflection_x);
        let legacy_horizon = if night {
            205.
        } else if ocean {
            160.
        } else {
            185.
        };
        let horizon = self.world.map_or(legacy_horizon, |w| w.horizon);
        let fires = if outdoors {
            self.visible_fires(player, art)
        } else {
            vec![]
        };
        // Conservative water mask: every covered logical actor pixel blocks its
        // lighting tile. Caustics cannot crawl across a character's edges.
        let mut water_sources = vec![false; player.sources.len() + 1];
        for (i, s) in player.sources.iter().enumerate() {
            water_sources[i + 1] = match s {
                PixelSource::Background(_) => true,
                PixelSource::Sprite(d) => d.name == "BACKGRND.BMP",
                PixelSource::Layer { top, under } => {
                    water_sources[*top as usize] && water_sources[*under as usize]
                }
                _ => false,
            };
        }
        let grid_x = |x: i32| {
            (((ox + x as f32 * scale) / CELL as f32).floor() as i32).clamp(0, gw as i32 - 1)
                as usize
        };
        let grid_y = |y: i32| {
            (((oy + y as f32 * scale) / CELL as f32).floor() as i32).clamp(0, gh as i32 - 1)
                as usize
        };
        let xs: Vec<_> = (0..=640).map(grid_x).collect();
        let ys: Vec<_> = (0..=480).map(grid_y).collect();
        if outdoors {
            for y in horizon as usize..player.height.min(480) as usize {
                for x in 0..640 {
                    if !water_sources[player.origins[y * 640 + x] as usize] {
                        for gy in ys[y]..=ys[y + 1] {
                            self.water[gy * gw + xs[x]..=gy * gw + xs[x + 1]].fill(false);
                        }
                    }
                }
            }
            for (&(y, x), &id) in &player.margin_origins {
                if y >= horizon as i32 && y < 480 && !water_sources[id as usize] {
                    for gy in grid_y(y)..=grid_y(y + 1) {
                        self.water[gy * gw + grid_x(x)..=gy * gw + grid_x(x + 1)].fill(false);
                    }
                }
            }
        }
        let flicker = 1. + 0.15 * (noise(time * 2.1) + noise(time * 5.3 + 7.) * 0.35);
        for gy in 0..gh {
            let py = (gy * CELL + CELL / 2).min(height - 1);
            let sy = (py as f32 + 0.5 - oy) / scale;
            for gx in 0..gw {
                let px = (gx * CELL + CELL / 2).min(width - 1);
                let at = gy * gw + gx;
                let sx = (px as f32 + 0.5 - ox) / scale;
                let c = input[py * width + px];
                let rgb = [
                    ((c >> 16) & 255) as f32,
                    ((c >> 8) & 255) as f32,
                    (c & 255) as f32,
                ];
                let mut gain = [1f32; 3];
                let mut emission = [0f32; 3];
                if outdoors && sy >= 0. && sy < player.height as f32 && c & 0xffffff != 0 {
                    if !night && sy < horizon {
                        let ray = ((sx * 0.025 + sy * 0.012 + time * 0.025).sin() * 0.5 + 0.5)
                            .powi(8)
                            * 0.035;
                        gain[0] += ray;
                        gain[1] += ray * 0.85;
                        gain[2] += ray * 0.5;
                    }
                    let blue = rgb[2] > rgb[0] * 1.06 && rgb[1] > rgb[0] * 0.85;
                    if sy >= horizon && blue && self.water[at] {
                        let a = self.normal(sx * 1.3 + time * 7., sy * 2.3 + time * 3.);
                        let b = self.normal(sx * 1.9 - time * 5., sy * 1.6 - time * 4.);
                        let crest = (a[0] * b[1] + a[1] * b[0]).max(0.).powi(3);
                        let strength = if night { 0.045 } else { 0.095 };
                        gain[1] += crest * strength;
                        gain[2] += crest * strength * 0.85;
                        let column = ((sx - reflection_x + a[0] * 12.)
                            / (18. + (sy - horizon) * 0.1))
                            .powi(2);
                        let reflection = (-column).exp() * crest * if night { 0.10 } else { 0.12 };
                        emission = [
                            reflection * if night { 0.6 } else { 1. },
                            reflection * 0.8,
                            reflection * if night { 1.2 } else { 0.35 },
                        ];
                        for f in &fires {
                            if sy > f.y {
                                let col =
                                    ((sx - f.x + a[1] * 5.) / (7. + (sy - f.y) * 0.12)).powi(2);
                                let light = (-col).exp() * crest * 0.14 * flicker;
                                for k in 0..3 {
                                    emission[k] += light * [1., 0.72, 0.25][k];
                                }
                            }
                        }
                    }
                    for f in &fires {
                        let d = ((sx - f.x).powi(2) + (sy - f.y).powi(2)).sqrt();
                        let dist = d * 4.;
                        let attenuation =
                            (1. - d / 80.).max(0.) / (1. + 0.005 * dist + 0.00002 * dist * dist);
                        for k in 0..3 {
                            gain[k] += attenuation * flicker * [1., 0.72, 0.25][k] * 0.8;
                        }
                        // Translucent curling smoke, confined to the active fire.
                        let rise = f.y - sy;
                        if (2.0..55.).contains(&rise) {
                            let curl = (rise * 0.15 + time * 0.7).sin() * 3. + rise * 0.08;
                            let smoke = (1. - (sx - f.x - curl).abs() / (2. + rise * 0.07)).max(0.)
                                * (1. - rise / 55.)
                                * 0.035;
                            for k in 0..3 {
                                emission[k] += smoke * [0.55, 0.58, 0.63][k];
                            }
                        }
                    }
                    let luminous = rgb[0].max(rgb[1]).max(rgb[2]) / 255.;
                    let warm = rgb[0] > 210. && rgb[0] > rgb[2] * 1.3;
                    let celestial = sy < horizon && (night || warm);
                    let flame = fires
                        .iter()
                        .any(|f| (sx - f.x).abs() < 12. && (sy - f.y).abs() < 16.);
                    if luminous > 0.80 && (celestial || flame) {
                        for k in 0..3 {
                            emission[k] += (luminous - 0.80) * 0.7 * (rgb[k] / 255.).powi(2);
                        }
                    }
                }
                self.field[at].gain = gain.map(|g| (g * 256.).round() as u16);
                self.emission[at] = emission;
            }
        }
        // Radius ~12 output pixels: two separable five-tap Gaussian passes.
        blur(&self.emission, &mut self.scratch, gw, gh, true);
        blur(&self.scratch, &mut self.emission, gw, gh, false);
        for (field, emission) in self.field.iter_mut().zip(&self.emission) {
            field.glow = emission.map(|v| (v * 1024.).round() as u16);
            for k in 0..3 {
                let gain = (field.gain[k] as usize).min(1024);
                let glow = ((field.glow[k] as usize + 2) / 4).min(255);
                let key = gain * 256 + glow;
                if self.table_index[key] == u32::MAX {
                    let table = std::array::from_fn(|value| {
                        self.aces[(self.linear[value] as usize * gain / 256 + glow * 4).min(8191)]
                    });
                    self.table_index[key] = self.tables.len() as u32;
                    self.tables.push(table);
                }
                field.tables[k] = self.table_index[key];
            }
        }
        self.pixels.resize(input.len(), 0);
        let tables = &self.tables;
        let field = &self.field;
        let workers = std::thread::available_parallelism()
            .map_or(1, usize::from)
            .min(4);
        let rows = gh.div_ceil(workers) * CELL;
        std::thread::scope(|scope| {
            for (part, (target, source)) in self
                .pixels
                .chunks_mut(rows * width)
                .zip(input.chunks(rows * width))
                .enumerate()
            {
                scope.spawn(move || {
                    for (row, (out, src)) in target
                        .chunks_mut(width)
                        .zip(source.chunks(width))
                        .enumerate()
                    {
                        let gy = (part * rows + row) / CELL;
                        for (gx, (dst, base)) in
                            out.chunks_mut(CELL).zip(src.chunks(CELL)).enumerate()
                        {
                            let f = field[gy * gw + gx];
                            let red = &tables[f.tables[0] as usize];
                            let green = &tables[f.tables[1] as usize];
                            let blue = &tables[f.tables[2] as usize];
                            for (dst, &c) in dst.iter_mut().zip(base) {
                                if c & 0xffffff == 0 {
                                    *dst = c;
                                    continue;
                                }
                                *dst = 0xff000000
                                    | ((red[((c >> 16) & 255) as usize] as u32) << 16)
                                    | ((green[((c >> 8) & 255) as usize] as u32) << 8)
                                    | blue[(c & 255) as usize] as u32;
                            }
                        }
                    }
                });
            }
        });
        // Deterministic 24-ember emitter: no per-frame allocation or state drift.
        for fire in fires {
            for i in 0..24 {
                let seed = i as f32 * 0.618034;
                let age = (time * (0.22 + (i % 5) as f32 * 0.018) + seed).fract();
                let x = fire.x + (seed * 71. + age * 8.).sin() * (2. + age * 6.) + age * 4.;
                let y = fire.y - age * 48.;
                let energy = (1. - age).powi(2) * 0.8;
                let radius = (scale * 0.35).round().max(1.) as i32;
                let px = (ox + x * scale).round() as i32;
                let py = (oy + y * scale).round() as i32;
                for dy in -radius..=radius {
                    for dx in -radius..=radius {
                        let (x, y) = (px + dx, py + dy);
                        if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
                            continue;
                        }
                        let at = y as usize * width + x as usize;
                        if input[at] & 0xffffff == 0 {
                            continue;
                        }
                        let alpha = energy
                            * (1.
                                - (dx * dx + dy * dy) as f32
                                    / (2. * (radius * radius) as f32 + 1.));
                        if alpha <= 0. {
                            continue;
                        }
                        let old = self.pixels[at];
                        let mut out = 0xff000000;
                        for (k, shift) in [16, 8, 0].into_iter().enumerate() {
                            let c = (((old >> shift) & 255) as f32 * (1. - alpha)
                                + [255., 175., 55.][k] * alpha)
                                .round() as u32;
                            out |= c << shift;
                        }
                        self.pixels[at] = out;
                    }
                }
            }
        }
        &self.pixels
    }
}
fn blur(source: &[[f32; 3]], dest: &mut [[f32; 3]], w: usize, h: usize, horizontal: bool) {
    for y in 0..h {
        for x in 0..w {
            let mut sum = [0.; 3];
            for (offset, weight) in [1., 4., 6., 4., 1.].into_iter().enumerate() {
                let d = offset as i32 - 2;
                let (px, py) = if horizontal {
                    ((x as i32 + d).clamp(0, w as i32 - 1) as usize, y)
                } else {
                    (x, (y as i32 + d).clamp(0, h as i32 - 1) as usize)
                };
                for k in 0..3 {
                    sum[k] += source[py * w + px][k] * weight / 16.;
                }
            }
            dest[y * w + x] = sum;
        }
    }
}
fn noise(t: f32) -> f32 {
    let n = t.floor() as i32;
    let f = t - t.floor();
    let gradient = |i: i32| {
        if (i as u32).wrapping_mul(0x45d9f3b).count_ones() % 2 == 0 {
            1.
        } else {
            -1.
        }
    };
    let smooth = f * f * f * (f * (f * 6. - 15.) + 10.);
    (gradient(n) * f * (1. - smooth) + gradient(n + 1) * (f - 1.) * smooth) * 2.
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remaster::RgbaImage;
    fn fixture() -> (Player, RemasterAssets, Vec<u32>) {
        let mut p = Player::new();
        p.current_background = Some("ISLAND2.SCR".into());
        p.sources
            .push(PixelSource::Background("ISLAND2.SCR".into()));
        p.origins.fill(1);
        let art = RemasterAssets {
            sprites: BTreeMap::new(),
            backgrounds: BTreeMap::new(),
            widescreen: BTreeMap::new(),
            remade_frames: 0,
        };
        (p, art, vec![0xff206080; 640 * 350])
    }
    fn actor(name: &str) -> Draw {
        Draw {
            name: name.into(),
            frame: 0,
            x: 80,
            y: 250,
            width: 4,
            height: 4,
            flip_x: false,
            flip_y: false,
            clip: [0, 0, 640, 480],
            pose_signature: 1,
        }
    }
    #[test]
    fn effects_are_time_based_deterministic_and_do_not_touch_black_framing_or_input() {
        let (p, art, source) = fixture();
        let original = source.clone();
        let mut source = source;
        source[..640 * 8].fill(0xff000000);
        let mut fx = Hd2dEffects::new();
        let a = fx
            .render(&source, 640, 350, [1., 0., 0.], &p, &art, 1.)
            .to_vec();
        let b = fx
            .render(&source, 640, 350, [1., 0., 0.], &p, &art, 3.)
            .to_vec();
        assert!(a.iter().zip(&b).filter(|(a, b)| a != b).count() > 100);
        assert_eq!(fx.render(&source, 640, 350, [1., 0., 0.], &p, &art, 1.), a);
        assert!(a[..640 * 8].iter().all(|&c| c == 0xff000000));
        assert_eq!(&source[640 * 8..], &original[640 * 8..]);
    }
    #[test]
    fn water_caustics_do_not_animate_across_foreground_characters() {
        let (mut p, art, source) = fixture();
        p.sources.push(PixelSource::Sprite(actor("ACTOR.BMP")));
        for y in 250..254 {
            p.origins[y * 640 + 80..y * 640 + 84].fill(2);
        }
        let mut fx = Hd2dEffects::new();
        let a = fx
            .render(&source, 640, 350, [1., 0., 0.], &p, &art, 1.)
            .to_vec();
        let b = fx
            .render(&source, 640, 350, [1., 0., 0.], &p, &art, 2.)
            .to_vec();
        for y in 250..254 {
            assert_eq!(
                &a[y * 640 + 80..y * 640 + 84],
                &b[y * 640 + 80..y * 640 + 84]
            );
        }
    }
    #[test]
    fn active_flame_lights_nearby_surfaces_and_removing_it_leaves_no_light_or_particles() {
        let (mut p, mut art, source) = fixture();
        p.current_background = Some("NIGHT.SCR".into());
        p.sources.push(PixelSource::Sprite(actor("FIRE1.BMP")));
        art.sprites.insert(
            "FIRE1.BMP".into(),
            vec![RgbaImage {
                width: 4,
                height: 4,
                pixels: vec![0xffffa020; 16],
            }],
        );
        let mut fx = Hd2dEffects::new();
        let unlit = fx
            .render(&source, 640, 350, [1., 0., 0.], &p, &art, 1.)
            .to_vec();
        for y in 250..254 {
            p.origins[y * 640 + 80..y * 640 + 84].fill(2);
        }
        let lit = fx
            .render(&source, 640, 350, [1., 0., 0.], &p, &art, 1.)
            .to_vec();
        assert!(((lit[250 * 640 + 96] >> 16) & 255) > ((unlit[250 * 640 + 96] >> 16) & 255));
        p.origins.fill(1);
        assert_eq!(
            fx.render(&source, 640, 350, [1., 0., 0.], &p, &art, 1.),
            unlit
        );
    }
}
