//! Crisp indexed presentation, with an optional 24-bit color treatment.
//! Precompute color ramps: rendering uses integer lookups and reuses its buffer.
pub const WIDTH: usize = 640;
pub const HEIGHT: usize = 480;
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ColorMode {
    Classic,
    Vibrant,
}
impl ColorMode {
    pub fn toggle(&mut self) {
        *self = match self {
            Self::Classic => Self::Vibrant,
            Self::Vibrant => Self::Classic,
        };
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Classic => "256 colors",
            Self::Vibrant => "Vibrant",
        }
    }
}
pub struct Presentation {
    classic: [u32; 256],
    vibrant: Vec<u32>,
    pub pixels: Vec<u32>,
}
fn pack(c: [u8; 3]) -> u32 {
    0xff000000 | ((c[0] as u32) << 16) | ((c[1] as u32) << 8) | c[2] as u32
}
impl Presentation {
    pub fn new(palette: &[[u8; 3]; 256]) -> Self {
        let classic = std::array::from_fn(|i| pack(palette[i]));
        let mut vibrant = vec![0; HEIGHT * 256];
        for y in 0..HEIGHT {
            for (i, &rgb) in palette.iter().enumerate() {
                let mut c = rgb.map(|v| v as f32 / 255.);
                let luma = c[0] * 0.2126 + c[1] * 0.7152 + c[2] * 0.0722;
                let max = c.iter().copied().fold(0f32, f32::max);
                let min = c.iter().copied().fold(1f32, f32::min);
                // Gentle vibrance on muted colors; saturated VGA colors get tonal headroom.
                let saturation = 1.12 - 0.24 * (max - min);
                for channel in &mut c {
                    *channel = luma + (*channel - luma) * saturation;
                }
                // Give VGA's flat cyan/blue ramps a tropical hue, preserving dark outlines.
                let blue = ((c[2] - c[0]).max(0.) * (c[2] - c[1]).max(0.)).sqrt();
                let cyan = (c[1].min(c[2]) - c[0]).max(0.) * (1. - (c[1] - c[2]).abs());
                let yellow = (c[0].min(c[1]) - c[2]).max(0.);
                let green = (c[1] - c[0].max(c[2])).max(0.);
                c[0] += blue * 0.045 + green * 0.06;
                c[1] += blue * 0.19 - cyan * 0.10 - yellow * 0.06;
                c[2] += -blue * 0.08 - cyan * 0.025 + yellow * 0.22 + green * 0.12;
                let depth = y as f32 / (HEIGHT - 1) as f32;
                let light = 1.045 - depth * 0.065;
                for channel in &mut c {
                    *channel = (*channel * light + 0.018 * (*channel * (1. - *channel)).max(0.))
                        .clamp(0., 1.);
                }
                vibrant[y * 256 + i] = pack(c.map(|v| (v * 255.).round() as u8));
            }
        }
        Self {
            classic,
            vibrant,
            pixels: vec![0xff000000; WIDTH * HEIGHT],
        }
    }
    pub fn update(&mut self, indexed: &[u8], height: usize, mode: ColorMode) {
        let h = height.min(HEIGHT);
        let offset = (HEIGHT - h) / 2;
        self.pixels[..offset * WIDTH].fill(0xff000000);
        self.pixels[(offset + h) * WIDTH..].fill(0xff000000);
        for y in 0..h {
            let lut = match mode {
                ColorMode::Classic => &self.classic[..],
                ColorMode::Vibrant => &self.vibrant[y * 256..(y + 1) * 256],
            };
            for (dst, &src) in self.pixels[(y + offset) * WIDTH..(y + offset + 1) * WIDTH]
                .iter_mut()
                .zip(&indexed[y * WIDTH..(y + 1) * WIDTH])
            {
                *dst = lut[src as usize];
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn classic_is_exact_and_bars_are_black() {
        let p = std::array::from_fn(|i| [i as u8, 0, 255 - i as u8]);
        let mut r = Presentation::new(&p);
        r.update(&vec![42; WIDTH * HEIGHT], 350, ColorMode::Classic);
        assert_eq!(r.pixels[65 * WIDTH], pack(p[42]));
        assert_eq!(r.pixels[0], 0xff000000);
        assert_eq!(r.pixels[HEIGHT * WIDTH - 1], 0xff000000);
    }
    #[test]
    fn vibrant_expands_color_depth_without_blurring() {
        let p = std::array::from_fn(|i| [i as u8, 100, 180]);
        let mut r = Presentation::new(&p);
        let indexed: Vec<u8> = (0..WIDTH * HEIGHT).map(|i| (i % 256) as u8).collect();
        r.update(&indexed, 480, ColorMode::Vibrant);
        let colors: std::collections::HashSet<_> = r.pixels.iter().collect();
        assert!(colors.len() > 256);
        let dark = Presentation::new(&[[0; 3]; 256]);
        assert!(dark.vibrant.iter().all(|&c| c == 0xff000000));
    }
}
