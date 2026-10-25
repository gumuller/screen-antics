//! Background/camera and foreground geometry for the new HD-2D scene engine.
use crate::{
    hd2d_art::{Art, Background},
    hd2d_layout::{Layout, Point},
    remaster::{load_image, AspectMode, RgbaImage},
};
pub struct Canvas {
    pub pixels: Vec<u32>,
    pub base: Vec<u32>,
    pub width: usize,
    pub height: usize,
    pub stage: [f64; 3],
    key: String,
    loaded: Option<(std::path::PathBuf, RgbaImage)>,
    overlay_key: Option<(usize, usize, [u64; 3], usize, usize, usize)>,
    overlay_pixels: Vec<u32>,
    overlay_bounds: [usize; 4],
    frame_spans: Vec<std::ops::Range<usize>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cached_frame_mask_keeps_only_intentional_black_pixels() {
        let mut canvas = Canvas::new(4, 2);
        canvas.base = vec![
            0xff000000, 0xff000000, 0xff123456, 0xff111111, 0xff222222, 0xff000000, 0xff333333,
            0xff000000,
        ];
        canvas.cache_frame_spans();
        canvas.pixels.fill(0xffffffff);
        canvas.preserve_frame();
        for (pixel, base) in canvas.pixels.iter().zip(&canvas.base) {
            assert_eq!(
                *pixel,
                if base & 0xffffff == 0 {
                    *base
                } else {
                    0xffffffff
                }
            );
        }
        canvas.base.fill(0xff123456);
        canvas.cache_frame_spans();
        canvas.pixels.fill(0xffffffff);
        canvas.preserve_frame();
        assert!(canvas.pixels.iter().all(|&p| p == 0xffffffff));
    }
    #[test]
    fn cached_stage_overlay_tracks_resize_camera_and_image() {
        let red = RgbaImage {
            width: 16,
            height: 16,
            pixels: vec![0xffff0000; 256],
        };
        let green = RgbaImage {
            width: 16,
            height: 16,
            pixels: vec![0xff00ff00; 256],
        };
        let mut canvas = Canvas::new(4, 4);
        canvas.stage_overlay(&red, 1.);
        assert!(canvas.pixels.iter().all(|&c| c == 0xffff0000));
        canvas.resize(8, 4);
        canvas.stage = [1., 2., 0.];
        canvas.pixels.fill(0xff000000);
        canvas.stage_overlay(&red, 1.);
        assert_eq!(
            &canvas.pixels[..8],
            &[
                0xff000000, 0xff000000, 0xffff0000, 0xffff0000, 0xffff0000, 0xffff0000, 0xff000000,
                0xff000000
            ]
        );
        canvas.stage = [1., 1., 0.];
        canvas.pixels.fill(0xff000000);
        canvas.stage_overlay(&green, 0.5);
        assert_eq!(
            &canvas.pixels[..8],
            &[
                0xff000000, 0xff007f00, 0xff007f00, 0xff007f00, 0xff007f00, 0xff000000, 0xff000000,
                0xff000000
            ]
        );
        let held = canvas.pixels.clone();
        canvas.stage_overlay(&red, 0.);
        assert_eq!(canvas.pixels, held);
    }
}
impl Canvas {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            pixels: vec![0; width * height],
            base: vec![0; width * height],
            width,
            height,
            stage: [1., 0., 0.],
            key: String::new(),
            loaded: None,
            overlay_key: None,
            overlay_pixels: vec![],
            overlay_bounds: [0; 4],
            frame_spans: vec![],
        }
    }
    pub fn resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
        self.key.clear();
        self.pixels.resize(width * height, 0);
        self.base.resize(width * height, 0);
    }
    fn select<'a>(
        &self,
        art: &'a Art,
        name: &str,
        aspect: AspectMode,
    ) -> Result<&'a Background, String> {
        let entries = art
            .backgrounds
            .get(name)
            .ok_or_else(|| format!("No new HD-2D background for {name}"))?;
        let mode = match aspect {
            AspectMode::Original => "core",
            AspectMode::Ratio16_9 => "16_9",
            AspectMode::Ratio21_9 => "21_9",
            AspectMode::Ratio43_18 => "43_18",
            AspectMode::Auto => {
                let ratio = self.width as f64 / self.height as f64;
                let core = entries
                    .iter()
                    .find(|v| v.mode == "core")
                    .ok_or("Missing new core background")?;
                if ratio <= core.width as f64 / core.height as f64 * 1.025 {
                    "core"
                } else {
                    return entries
                        .iter()
                        .filter(|v| v.mode != "core")
                        .min_by(|a, b| {
                            let delta = |v: &Background| {
                                (v.width as f64 / v.height as f64 / ratio).ln().abs()
                            };
                            delta(a).total_cmp(&delta(b))
                        })
                        .ok_or_else(|| "Missing HD-2D panorama".into());
                }
            }
        };
        entries
            .iter()
            .find(|v| v.mode == mode)
            .ok_or_else(|| format!("Missing HD-2D {mode} background"))
    }
    pub fn background(&mut self, art: &Art, name: &str, aspect: AspectMode) -> Result<(), String> {
        let bg = self.select(art, name, aspect)?;
        let key = format!(
            "{}:{}:{}x{}",
            bg.path.display(),
            bg.mode,
            self.width,
            self.height
        );
        if key != self.key {
            if self.loaded.as_ref().map(|(p, _)| p) != Some(&bg.path) {
                self.loaded = Some((bg.path.clone(), load_image(&bg.path)?));
            }
            let image = &self.loaded.as_ref().unwrap().1;
            if image.width != bg.width || image.height != bg.height {
                return Err("Scene background metadata dimensions disagree".into());
            }
            let scale =
                (self.width as f64 / bg.width as f64).min(self.height as f64 / bg.height as f64);
            let ox = (self.width as f64 - bg.width as f64 * scale) / 2.;
            let oy = (self.height as f64 - bg.height as f64 * scale) / 2.;
            self.stage = [
                scale * 4.,
                ox + bg.stage[0] as f64 * scale,
                oy + bg.stage[1] as f64 * scale,
            ];
            for y in 0..self.height {
                for x in 0..self.width {
                    let px = (x as f64 + 0.5 - ox) / scale;
                    let py = (y as f64 + 0.5 - oy) / scale;
                    self.base[y * self.width + x] =
                        if px >= 0. && py >= 0. && px < bg.width as f64 && py < bg.height as f64 {
                            image.pixels[py as usize * bg.width + px as usize]
                        } else {
                            0xff000000
                        };
                }
            }
            self.cache_frame_spans();
            self.key = key;
        }
        self.pixels.copy_from_slice(&self.base);
        Ok(())
    }
    pub fn shadow(&mut self, point: Point, radius: f32, alpha: f32) {
        let [s, ox, oy] = self.stage;
        let r = radius as f64 * s;
        let cx = ox + point.x as f64 * s;
        let cy = oy + point.y as f64 * s;
        let x0 = (cx - r).max(0.) as usize;
        let x1 = (cx + r).ceil().min(self.width as f64) as usize;
        let y0 = (cy - r * 0.2).max(0.) as usize;
        let y1 = (cy + r * 0.2).ceil().min(self.height as f64) as usize;
        for y in y0..y1 {
            for x in x0..x1 {
                let d = ((x as f64 + 0.5 - cx) / r).powi(2)
                    + ((y as f64 + 0.5 - cy) / (r * 0.2)).powi(2);
                if d >= 1. {
                    continue;
                }
                let attenuation = (1. - alpha as f64 * (1. - d)) as f32;
                let c = self.pixels[y * self.width + x];
                let mut out = 0xff000000;
                for shift in [0, 8, 16] {
                    out |= ((((c >> shift) & 255) as f32 * attenuation) as u32) << shift;
                }
                self.pixels[y * self.width + x] = out;
            }
        }
    }
    pub fn stage_overlay(&mut self, image: &RgbaImage, opacity: f32) {
        if opacity <= 0. {
            return;
        }
        let key = (
            self.width,
            self.height,
            self.stage.map(f64::to_bits),
            image.pixels.as_ptr() as usize,
            image.width,
            image.height,
        );
        if self.overlay_key != Some(key) {
            let [s, ox, oy] = self.stage;
            let left = (ox - 0.5).ceil().clamp(0., self.width as f64) as usize;
            let top = (oy - 0.5).ceil().clamp(0., self.height as f64) as usize;
            let right = (ox + image.width as f64 * s / 4. - 0.5)
                .ceil()
                .clamp(0., self.width as f64) as usize;
            let bottom = (oy + image.height as f64 * s / 4. - 0.5)
                .ceil()
                .clamp(0., self.height as f64) as usize;
            self.overlay_bounds = [left, top, right, bottom];
            self.overlay_pixels.clear();
            self.overlay_pixels.resize(self.pixels.len(), 0);
            for y in 0..self.height {
                let sy = ((y as f64 + 0.5 - oy) / s * 4.).floor() as i64;
                if sy < 0 || sy >= image.height as i64 {
                    continue;
                }
                for x in 0..self.width {
                    let sx = ((x as f64 + 0.5 - ox) / s * 4.).floor() as i64;
                    if sx < 0 || sx >= image.width as i64 {
                        continue;
                    }
                    self.overlay_pixels[y * self.width + x] =
                        image.pixels[sy as usize * image.width + sx as usize] | 0xff000000;
                }
            }
            self.overlay_key = Some(key);
        }
        let a = (opacity.clamp(0., 1.) * 255.) as u32;
        if a == 255 {
            let [left, top, right, bottom] = self.overlay_bounds;
            for y in top..bottom {
                let row = y * self.width + left..y * self.width + right;
                self.pixels[row.clone()].copy_from_slice(&self.overlay_pixels[row]);
            }
            return;
        }
        let [left, top, right, bottom] = self.overlay_bounds;
        let width = self.width;
        let blend = |target: &mut [u32], source: &[u32]| {
            for (target, source) in target.chunks_mut(width).zip(source.chunks(width)) {
                for (out, &color) in target[left..right].iter_mut().zip(&source[left..right]) {
                    let base = *out;
                    *out = 0xff000000;
                    for shift in [0, 8, 16] {
                        *out |= ((((color >> shift) & 255) * a
                            + ((base >> shift) & 255) * (255 - a)
                            + 127)
                            / 255)
                            << shift;
                    }
                }
            }
        };
        let target = &mut self.pixels[top * width..bottom * width];
        let source = &self.overlay_pixels[top * width..bottom * width];
        if (right - left) * (bottom - top) >= 262_144 {
            let workers = std::thread::available_parallelism()
                .map_or(1, usize::from)
                .min(4);
            let rows = (bottom - top).div_ceil(workers);
            std::thread::scope(|scope| {
                for (target, source) in target
                    .chunks_mut(rows * width)
                    .zip(source.chunks(rows * width))
                {
                    let blend = &blend;
                    scope.spawn(move || blend(target, source));
                }
            });
        } else {
            blend(target, source);
        }
    }
    fn overlay(&mut self, x: usize, y: usize, color: u32, alpha: f32) {
        if alpha <= 0. || x >= self.width || y >= self.height {
            return;
        }
        let at = y * self.width + x;
        if self.base[at] & 0xffffff == 0 {
            return;
        }
        let a = (alpha.clamp(0., 1.) * 255.) as u32;
        let old = self.pixels[at];
        let mut out = 0xff000000;
        for shift in [0, 8, 16] {
            out |= ((((color >> shift) & 255) * a + ((old >> shift) & 255) * (255 - a) + 127)
                / 255)
                << shift;
        }
        self.pixels[at] = out;
    }
    pub fn ripple(&mut self, point: Point, radius: f32, phase: f32) {
        let [s, ox, oy] = self.stage;
        let rx = radius as f64 * s;
        let ry = (radius * 0.12).max(1.) as f64 * s;
        let cx = ox + point.x as f64 * s;
        let cy = oy + point.y as f64 * s;
        let x0 = (cx - rx - 2.).max(0.) as usize;
        let x1 = (cx + rx + 2.).ceil().min(self.width as f64) as usize;
        let y0 = (cy - ry - 2.).max(0.) as usize;
        let y1 = (cy + ry + 2.).ceil().min(self.height as f64) as usize;
        for y in y0..y1 {
            for x in x0..x1 {
                let dx = (x as f64 + 0.5 - cx) / rx;
                let dy = (y as f64 + 0.5 - cy) / ry;
                let d = (dx * dx + dy * dy).sqrt();
                let line = (1. - (d - 1.).abs() * ry).clamp(0., 1.) as f32;
                let wave = (dx as f32 * 8. + phase).sin() * 0.25 + 0.75;
                self.overlay(x, y, 0xffb8edeb, line * wave * 0.42);
            }
        }
    }
    pub fn fishing_line(&mut self, tip: Point, float: Point, taut: bool, time: f64) {
        let end = Point::new(float.x, float.y + (time as f32 * 2.).sin() * 0.6);
        let [s, ox, oy] = self.stage;
        let steps = ((tip.x - end.x).hypot(tip.y - end.y) as f64 * s * 1.5)
            .ceil()
            .max(1.) as usize;
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let x = tip.x + (end.x - tip.x) * t;
            let y = tip.y + (end.y - tip.y) * t + if taut { 0. } else { 8. * t * (1. - t) };
            let px = (ox + x as f64 * s).round() as isize;
            let py = (oy + y as f64 * s).round() as isize;
            if px >= 0 && py >= 0 && px < self.width as isize && py < self.height as isize {
                self.overlay(px as usize, py as usize, 0xe8e1c4, 0.7);
            }
        }
    }
    pub fn projectile(&mut self, point: Point) {
        let [s, ox, oy] = self.stage;
        let cx = ox + point.x as f64 * s;
        let cy = oy + point.y as f64 * s;
        let r = 1.5 * s;
        for y in (cy - r).max(0.) as usize..(cy + r).ceil().min(self.height as f64) as usize {
            for x in (cx - r).max(0.) as usize..(cx + r).ceil().min(self.width as f64) as usize {
                if (x as f64 + 0.5 - cx).powi(2) + (y as f64 + 0.5 - cy).powi(2) <= r * r {
                    self.overlay(x, y, 0x292d35, 1.);
                }
            }
        }
    }
    pub fn scorecard(&mut self, point: Point, text: &str) {
        let glyphs = [
            0b111101101101111u16,
            0b010110010010111,
            0b111001111100111,
            0b111001111001111,
            0b101101111001001,
            0b111100111001111,
            0b111100111101111,
            0b111001010010010,
            0b111101111101111,
            0b111101111001111,
        ];
        let [s, ox, oy] = self.stage;
        let width = (text.len() * 4 + 4) as f64;
        let left = point.x as f64 - width / 2.;
        let top = point.y as f64 - 9.;
        let x0 = (ox + left * s).floor().max(0.) as usize;
        let x1 = (ox + (left + width) * s).ceil().min(self.width as f64) as usize;
        let y0 = (oy + top * s).floor().max(0.) as usize;
        let y1 = (oy + (top + 9.) * s).ceil().min(self.height as f64) as usize;
        for y in y0..y1 {
            for x in x0..x1 {
                let px = ((x as f64 + 0.5 - ox) / s - left) as usize;
                let py = ((y as f64 + 0.5 - oy) / s - top) as usize;
                let mut ink = px == 0 || px + 1 >= width as usize || py == 0 || py == 8;
                if px >= 2 && (2..7).contains(&py) {
                    let index = (px - 2) / 4;
                    let col = (px - 2) % 4;
                    if let Some(ch) = text.as_bytes().get(index) {
                        if ch.is_ascii_digit() && col < 3 {
                            ink |= glyphs[(ch - b'0') as usize]
                                & (1 << (14 - ((py - 2) * 3 + col)))
                                != 0;
                        }
                        if *ch == b'.' {
                            ink |= py == 6 && col == 1;
                        }
                    }
                }
                self.overlay(x, y, if ink { 0x343039 } else { 0xffeccb }, 1.);
            }
        }
    }
    /// Ink only, centered inside a scorecard already painted in the judge pose.
    pub fn score_digits(&mut self, point: Point, text: &str) {
        let glyphs = [
            0b111101101101111u16,
            0b010110010010111,
            0b111001111100111,
            0b111001111001111,
            0b101101111001001,
            0b111100111001111,
            0b111100111101111,
            0b111001010010010,
            0b111101111101111,
            0b111101111001111,
        ];
        let width = (text.len() * 4).saturating_sub(1) as f64;
        let [s, ox, oy] = self.stage;
        for (i, ch) in text.bytes().enumerate() {
            for row in 0..5 {
                for col in 0..3 {
                    let ink = if ch.is_ascii_digit() {
                        glyphs[(ch - b'0') as usize] & (1 << (14 - row * 3 - col)) != 0
                    } else if ch == b'.' {
                        row == 4 && col == 1
                    } else {
                        ch == b'-' && row == 2
                    };
                    if !ink {
                        continue;
                    }
                    let x = ox + (point.x as f64 - width * 0.5 + (i * 4 + col) as f64) * s;
                    let y = oy + (point.y as f64 - 2.5 + row as f64) * s;
                    for py in (y - 0.5).ceil().max(0.) as usize
                        ..(y + s - 0.5).ceil().min(self.height as f64) as usize
                    {
                        for px in (x - 0.5).ceil().max(0.) as usize
                            ..(x + s - 0.5).ceil().min(self.width as f64) as usize
                        {
                            self.overlay(px, py, 0x343039, 1.);
                        }
                    }
                }
            }
        }
    }
    pub fn lightning(&mut self, from: Point, to: Point) {
        let [s, ox, oy] = self.stage;
        let points = [
            (0., 0.),
            (0.22, 7.),
            (0.43, -5.),
            (0.62, 6.),
            (0.80, -3.),
            (1., 0.),
        ];
        for pair in points.windows(2) {
            let pos = |(t, dx): (f32, f32)| {
                (
                    ox + (from.x + (to.x - from.x) * t + dx) as f64 * s,
                    oy + (from.y + (to.y - from.y) * t) as f64 * s,
                )
            };
            let (ax, ay) = pos(pair[0]);
            let (bx, by) = pos(pair[1]);
            let steps = ((bx - ax).abs().max((by - ay).abs()).ceil() as usize).max(1);
            let radius = (s * 2.).ceil() as i32;
            for i in 0..=steps {
                let t = i as f64 / steps as f64;
                let x = ax + (bx - ax) * t;
                let y = ay + (by - ay) * t;
                for dy in -radius..=radius {
                    for dx in -radius..=radius {
                        let px = x.round() as i32 + dx;
                        let py = y.round() as i32 + dy;
                        if px < 0 || py < 0 {
                            continue;
                        }
                        let d = ((dx * dx + dy * dy) as f64).sqrt() / s;
                        self.overlay(
                            px as usize,
                            py as usize,
                            if d < 0.7 { 0xfffce8 } else { 0xfacb65 },
                            if d < 0.7 {
                                1.
                            } else {
                                (0.22 * (1. - d / 2.).max(0.)) as f32
                            },
                        );
                    }
                }
            }
        }
    }
    pub fn smoke(&mut self, point: Point, time: f64) {
        let [s, ox, oy] = self.stage;
        for i in 0..4 {
            let age = (time as f32 * 0.32 + i as f32 * 0.25).fract();
            let r = (2. + age * 4.) as f64 * s;
            let cx = ox + (point.x + (age * 7. + i as f32).sin() * 2.) as f64 * s;
            let cy = oy + (point.y - 5. - age * 28.) as f64 * s;
            let x0 = (cx - r).max(0.) as usize;
            let x1 = (cx + r).ceil().min(self.width as f64) as usize;
            let y0 = (cy - r).max(0.) as usize;
            let y1 = (cy + r).ceil().min(self.height as f64) as usize;
            for y in y0..y1 {
                for x in x0..x1 {
                    let d = (((x as f64 - cx) / r).powi(2) + ((y as f64 - cy) / r).powi(2)) as f32;
                    self.overlay(
                        x,
                        y,
                        0xff697582,
                        (1. - d).max(0.).powi(2) * (1. - age) * 0.12,
                    );
                }
            }
        }
    }
    pub fn sleep_text(&mut self, point: Point, time: f64) {
        let [s, ox, oy] = self.stage;
        for i in 0..3 {
            let age = (time as f32 * 0.3 + i as f32 / 3.).fract();
            let x = point.x + 25. + age * 8.;
            let y = point.y - 20. - age * 23.;
            let size = 3. + age * 2.;
            let alpha = 1. - age;
            for step in 0..24 {
                let t = step as f32 / 23.;
                for (px, py) in [
                    (x + t * size, y),
                    (x + (1. - t) * size, y + t * size),
                    (x + t * size, y + size),
                ] {
                    let px = (ox + px as f64 * s).round() as i32;
                    let py = (oy + py as f64 * s).round() as i32;
                    if px >= 0 && py >= 0 {
                        for dy in 0..(s.round() as usize).max(1) {
                            self.overlay(px as usize, py as usize + dy, 0xffe7e5d5, alpha);
                        }
                    }
                }
            }
        }
    }
    pub fn water_cover(&mut self, rect: [f32; 4], waterline: f32) {
        let [s, ox, oy] = self.stage;
        let [left, top, w, h] = rect;
        let x0 = (ox + left as f64 * s).floor().clamp(0., self.width as f64) as usize;
        let x1 = (ox + (left + w) as f64 * s)
            .ceil()
            .clamp(0., self.width as f64) as usize;
        let y0 = (oy + waterline.max(top) as f64 * s)
            .floor()
            .clamp(0., self.height as f64) as usize;
        let y1 = (oy + (top + h) as f64 * s)
            .ceil()
            .clamp(0., self.height as f64) as usize;
        for y in y0..y1 {
            let depth = ((y as f64 + 0.5 - oy) / s - waterline as f64).max(0.);
            let alpha = (110. * (1. - depth / 30.).clamp(0., 1.)) as u32;
            for x in x0..x1 {
                let at = y * self.width + x;
                let c = self.pixels[at];
                let water = self.base[at];
                let mut pixel = 0xff000000;
                for shift in [0, 8, 16] {
                    pixel |= ((((c >> shift) & 255) * alpha
                        + ((water >> shift) & 255) * (255 - alpha))
                        / 255)
                        << shift;
                }
                self.pixels[at] = pixel;
            }
        }
    }
    pub fn preserve_frame(&mut self) {
        // Only outside the painted canvas is black framing. Do not let overscan
        // actors leak into intentional office/title borders or letterboxing.
        for span in &self.frame_spans {
            self.pixels[span.clone()].copy_from_slice(&self.base[span.clone()]);
        }
    }
    fn cache_frame_spans(&mut self) {
        self.frame_spans.clear();
        let mut start = None;
        for (i, &pixel) in self.base.iter().enumerate() {
            if pixel & 0xffffff == 0 {
                start.get_or_insert(i);
            } else if let Some(first) = start.take() {
                self.frame_spans.push(first..i);
            }
        }
        if let Some(first) = start {
            self.frame_spans.push(first..self.base.len());
        }
    }
    /// The painted tree is the only palm. This material/geometry mask can put it
    /// back in front of an actor without importing the old red trunk sprite.
    pub fn palm_foreground(&mut self, layout: Layout, rect: [f32; 4], include_crown: bool) {
        let [scale, ox, oy] = self.stage;
        let [left, top, w, h] = rect;
        let x0 = (ox + left as f64 * scale)
            .floor()
            .clamp(0., self.width as f64) as usize;
        let x1 = (ox + (left + w) as f64 * scale)
            .ceil()
            .clamp(0., self.width as f64) as usize;
        let y0 = (oy + top as f64 * scale)
            .floor()
            .clamp(0., self.height as f64) as usize;
        let y1 = (oy + (top + h) as f64 * scale)
            .ceil()
            .clamp(0., self.height as f64) as usize;
        for y in y0..y1 {
            let sy = ((y as f64 + 0.5 - oy) / scale) as f32;
            for x in x0..x1 {
                let sx = ((x as f64 + 0.5 - ox) / scale) as f32;
                let t = ((layout.palm_root.y - sy) / (layout.palm_root.y - layout.palm_crown.y))
                    .clamp(0., 1.);
                let trunk = layout.palm_at(t);
                let in_trunk = sy >= layout.palm_crown.y
                    && sy <= layout.palm_root.y
                    && (sx - trunk.x).abs() < 7. - t * 2.;
                let in_crown =
                    include_crown && ((sx - 469.) / 74.).powi(2) + ((sy - 61.) / 58.).powi(2) < 1.;
                if !in_trunk && !in_crown {
                    continue;
                }
                let at = y * self.width + x;
                let c = self.base[at];
                let r = (c >> 16) & 255;
                let g = (c >> 8) & 255;
                let b = c & 255;
                let green = g * 10 > r * 8 && g * 10 > b * 12;
                let bark = r * 10 > g * 10 && g * 10 > b * 12 && r < 175;
                if green || bark {
                    self.pixels[at] = c;
                }
            }
        }
    }
}
