//! New semantic HD-2D artwork. There is deliberately no legacy bitmap fallback.
use crate::remaster::{load_image, RgbaImage};
use std::{
    collections::BTreeMap,
    fs,
    path::{Component, Path},
};
pub struct Frame {
    pub image: RgbaImage,
    pub anchor: [f32; 2],
    pub flip_x: bool,
    pub contacts: BTreeMap<String, [f32; 2]>,
}
pub struct Clip {
    pub frames: Vec<Frame>,
    pub ppu: f32,
    pub fps: f32,
}
pub struct Art {
    pub clips: BTreeMap<String, Clip>,
    pub backgrounds: BTreeMap<String, Vec<Background>>,
    pub ending_clean: RgbaImage,
}
pub struct Background {
    pub mode: String,
    pub width: usize,
    pub height: usize,
    pub stage: [usize; 4],
    pub path: std::path::PathBuf,
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn large_translucent_draw_matches_scalar_blending_and_keeps_bounds() {
        let colors = vec![0x00a03040, 0xfcc09060, 0x80d0a0f0, 0xff405060];
        let art = Art {
            clips: BTreeMap::from([(
                "test".into(),
                Clip {
                    ppu: 1.,
                    fps: 1.,
                    frames: vec![Frame {
                        image: RgbaImage {
                            width: 2,
                            height: 2,
                            pixels: colors.clone(),
                        },
                        anchor: [0., 0.],
                        flip_x: false,
                        contacts: BTreeMap::new(),
                    }],
                },
            )]),
            backgrounds: BTreeMap::new(),
            ending_clean: RgbaImage {
                width: 1,
                height: 1,
                pixels: vec![0],
            },
        };
        for (flip, tint, opacity) in [(false, None, 1.), (true, Some(0x918273), 0.4)] {
            let mut pixels = vec![0xff204060; 521 * 519];
            art.draw_transformed(
                "test",
                0,
                &mut pixels,
                [521, 519],
                [256., 4., 2.],
                [if flip { 2. } else { 0. }, 0.],
                [1., 1.],
                flip,
                tint,
                opacity,
            )
            .unwrap();
            for y in 0..519 {
                for x in 0..521 {
                    let mut expected = 0xff204060;
                    if (4..516).contains(&x) && (2..514).contains(&y) {
                        let sx = (x - 4) / 256;
                        let c = colors[(y - 2) / 256 * 2 + if flip { 1 - sx } else { sx }];
                        let alpha = ((c >> 24) as f32 * opacity) as u32;
                        let rgb = tint.unwrap_or(c);
                        expected = 0xff000000;
                        for shift in [0, 8, 16] {
                            expected |= ((((rgb >> shift) & 255) * alpha
                                + ((0xff204060u32 >> shift) & 255) * (255 - alpha)
                                + 127)
                                / 255)
                                << shift;
                        }
                    }
                    assert_eq!(pixels[y * 521 + x], expected, "pixel {x},{y}");
                }
            }
        }
    }
}
impl Art {
    pub fn load(root: &Path) -> Result<Self, String> {
        let manifest = root.join("scene-art.tsv");
        let text =
            fs::read_to_string(&manifest).map_err(|e| format!("{}: {e}", manifest.display()))?;
        let mut clips: BTreeMap<String, Clip> = BTreeMap::new();
        for (line, row) in text.lines().enumerate().skip(1) {
            let f: Vec<_> = row.split('\t').collect();
            let error = || {
                format!(
                    "{}:{}: invalid HD-2D clip entry",
                    manifest.display(),
                    line + 1
                )
            };
            if f.len() != 8 {
                return Err(error());
            }
            let index = f[1].parse::<usize>().map_err(|_| error())?;
            let fps = f[2].parse::<f32>().map_err(|_| error())?;
            let ppu = f[3].parse::<f32>().map_err(|_| error())?;
            let anchor = [
                f[4].parse::<f32>().map_err(|_| error())?,
                f[5].parse::<f32>().map_err(|_| error())?,
            ];
            if !fps.is_finite()
                || fps <= 0.
                || !ppu.is_finite()
                || ppu <= 0.
                || anchor.iter().any(|v| !v.is_finite())
                || Path::new(f[7])
                    .components()
                    .any(|c| !matches!(c, Component::Normal(_)))
            {
                return Err(error());
            }
            let clip = clips.entry(f[0].into()).or_insert_with(|| Clip {
                frames: vec![],
                ppu,
                fps,
            });
            if index != clip.frames.len() {
                return Err(error());
            }
            let image = load_image(root.join(f[7]))?;
            clip.frames.push(Frame {
                image,
                anchor,
                flip_x: f[6] == "1",
                contacts: BTreeMap::new(),
            });
        }
        if clips.is_empty() {
            return Err("HD-2D semantic art catalog is empty".into());
        }
        let contacts = root.join("scene-contacts.tsv");
        if contacts.exists() {
            for row in fs::read_to_string(&contacts)
                .map_err(|e| e.to_string())?
                .lines()
                .skip(1)
            {
                let f: Vec<_> = row.split('\t').collect();
                if f.len() != 5 {
                    return Err("Invalid HD-2D contact entry".into());
                }
                let index = f[1].parse::<usize>().map_err(|e| e.to_string())?;
                let p = [
                    f[3].parse::<f32>().map_err(|e| e.to_string())?,
                    f[4].parse::<f32>().map_err(|e| e.to_string())?,
                ];
                if p.iter().any(|v| !v.is_finite()) {
                    return Err("Invalid HD-2D contact coordinates".into());
                }
                clips
                    .get_mut(f[0])
                    .and_then(|c| c.frames.get_mut(index))
                    .ok_or("Contact references missing frame")?
                    .contacts
                    .insert(f[2].into(), p);
            }
        }
        let mut backgrounds: BTreeMap<String, Vec<Background>> = BTreeMap::new();
        let path = root.join("scene-backgrounds.tsv");
        let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        for row in text.lines().skip(1) {
            let f: Vec<_> = row.split('\t').collect();
            if f.len() != 9 {
                return Err("Invalid HD-2D scene background entry".into());
            }
            let n: Vec<usize> = f[2..8]
                .iter()
                .map(|s| {
                    s.parse()
                        .map_err(|_| "Invalid scene dimensions".to_string())
                })
                .collect::<Result<_, _>>()?;
            if n[0] == 0 || n[1] == 0 || n[2] + n[4] > n[0] || n[3] + n[5] > n[1] || n[4] != 2560 {
                return Err(format!("Invalid HD-2D background geometry for {}", f[0]));
            }
            backgrounds
                .entry(f[0].into())
                .or_default()
                .push(Background {
                    mode: f[1].into(),
                    width: n[0],
                    height: n[1],
                    stage: [n[2], n[3], n[4], n[5]],
                    path: root.join(f[8]),
                });
        }
        Ok(Self {
            clips,
            backgrounds,
            ending_clean: load_image(root.join("backdrops/ending-clean.rgba"))?,
        })
    }
    pub fn frame(&self, name: &str, phase: usize) -> Result<(&Frame, f32), String> {
        let clip = self
            .clips
            .get(name)
            .ok_or_else(|| format!("Missing new HD-2D clip: {name}"))?;
        Ok((&clip.frames[phase % clip.frames.len()], clip.ppu))
    }
    pub fn contact(
        &self,
        name: &str,
        phase: usize,
        contact: &str,
        point: [f32; 2],
        scale: f32,
        flip: bool,
    ) -> Option<[f32; 2]> {
        let (frame, ppu) = self.frame(name, phase).ok()?;
        let at = frame.contacts.get(contact)?;
        let direction = if flip ^ frame.flip_x { -1. } else { 1. };
        Some([
            point[0] + (at[0] - frame.anchor[0]) * scale / ppu * direction,
            point[1] + (at[1] - frame.anchor[1]) * scale / ppu,
        ])
    }
    pub fn draw(
        &self,
        name: &str,
        phase: usize,
        target: &mut [u32],
        size: [usize; 2],
        stage: [f64; 3],
        point: [f32; 2],
        scale: f32,
        flip: bool,
    ) -> Result<[f32; 4], String> {
        self.draw_transformed(
            name,
            phase,
            target,
            size,
            stage,
            point,
            [scale, scale],
            flip,
            None,
            1.,
        )
    }
    pub fn draw_transformed(
        &self,
        name: &str,
        phase: usize,
        target: &mut [u32],
        size: [usize; 2],
        stage: [f64; 3],
        point: [f32; 2],
        scale: [f32; 2],
        flip: bool,
        tint: Option<u32>,
        opacity: f32,
    ) -> Result<[f32; 4], String> {
        let (frame, ppu) = self.frame(name, phase)?;
        let image = &frame.image;
        let flip = flip ^ frame.flip_x;
        let factor = scale[0].max(0.001) / ppu;
        let factor_y = scale[1].max(0.001) / ppu;
        let anchor_x = if flip {
            image.width as f32 - frame.anchor[0]
        } else {
            frame.anchor[0]
        };
        let left = point[0] - anchor_x * factor;
        let top = point[1] - frame.anchor[1] * factor_y;
        let width = image.width as f32 * factor;
        let height = image.height as f32 * factor_y;
        let [s, ox, oy] = stage;
        let x0 = (ox + left as f64 * s - 0.5)
            .ceil()
            .clamp(0., size[0] as f64) as usize;
        let x1 = (ox + (left + width) as f64 * s - 0.5)
            .ceil()
            .clamp(0., size[0] as f64) as usize;
        let y0 = (oy + top as f64 * s - 0.5).ceil().clamp(0., size[1] as f64) as usize;
        let y1 = (oy + (top + height) as f64 * s - 0.5)
            .ceil()
            .clamp(0., size[1] as f64) as usize;
        let columns: Vec<_> = (x0..x1)
            .map(|x| {
                let sx = ((((x as f64 + 0.5 - ox) / s - left as f64) / factor as f64) as usize)
                    .min(image.width - 1);
                if flip {
                    image.width - 1 - sx
                } else {
                    sx
                }
            })
            .collect();
        let alpha: [u32; 256] = std::array::from_fn(|a| (a as f32 * opacity.clamp(0., 1.)) as u32);
        let draw_rows = |rows: &mut [u32], first_y: usize| {
            for (row, target) in rows.chunks_mut(size[0]).enumerate() {
                let y = first_y + row;
                let sy = ((((y as f64 + 0.5 - oy) / s - top as f64) / factor_y as f64) as usize)
                    .min(image.height - 1);
                for (offset, &sx) in columns.iter().enumerate() {
                    let original = image.pixels[sy * image.width + sx];
                    let c = tint.unwrap_or(original);
                    let a = alpha[(original >> 24) as usize];
                    if a == 0 {
                        continue;
                    }
                    let at = x0 + offset;
                    if a == 255 {
                        target[at] = 0xff000000 | (c & 0x00ffffff);
                        continue;
                    }
                    let base = target[at];
                    let mut out = 0xff000000;
                    for shift in [0, 8, 16] {
                        out |= (((((c >> shift) & 255) * a
                            + ((base >> shift) & 255) * (255 - a)
                            + 127)
                            / 255)
                            & 255)
                            << shift;
                    }
                    target[at] = out;
                }
            }
        };
        if (x1 - x0) * (y1 - y0) >= 262_144 {
            // Foreground ships can cover the complete ultrawide surface. Keep
            // their partial alpha intact while blending disjoint rows in parallel.
            let workers = std::thread::available_parallelism()
                .map_or(1, usize::from)
                .min(4);
            let rows = (y1 - y0).div_ceil(workers);
            std::thread::scope(|scope| {
                for (part, target) in target[y0 * size[0]..y1 * size[0]]
                    .chunks_mut(rows * size[0])
                    .enumerate()
                {
                    let draw_rows = &draw_rows;
                    scope.spawn(move || draw_rows(target, y0 + part * rows));
                }
            });
        } else {
            draw_rows(&mut target[y0 * size[0]..y1 * size[0]], y0);
        }
        Ok([left, top, width, height])
    }
}
