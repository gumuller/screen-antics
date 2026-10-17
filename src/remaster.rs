//! Optional true-color replacement art, with contour-preserving classic fallbacks.
//! SAHD files: magic, little-endian u32 width/height, then straight RGBA pixels.
use crate::resources::{AssetStore, Image};
use std::{collections::BTreeMap, fs, path::Path};
#[derive(Clone, Debug)]
pub struct RgbaImage {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u32>,
}
pub struct RemasterAssets {
    pub sprites: BTreeMap<String, Vec<RgbaImage>>,
    pub backgrounds: BTreeMap<String, RgbaImage>,
    /// Number of frames that use newly drawn artwork, rather than enhanced originals.
    pub remade_frames: usize,
}
pub fn load_image(path: impl AsRef<Path>) -> Result<RgbaImage, String> {
    let path = path.as_ref();
    let bytes = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if bytes.len() < 12 || &bytes[..4] != b"SAHD" {
        return Err(format!("{}: invalid SAHD image", path.display()));
    }
    let width = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
    let height = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    if width == 0
        || height == 0
        || width > 16384
        || height > 16384
        || bytes.len() != 12 + width * height * 4
    {
        return Err(format!("{}: invalid image dimensions/data", path.display()));
    }
    let pixels = bytes[12..]
        .chunks_exact(4)
        .map(|c| ((c[3] as u32) << 24) | ((c[0] as u32) << 16) | ((c[1] as u32) << 8) | c[2] as u32)
        .collect();
    Ok(RgbaImage {
        width,
        height,
        pixels,
    })
}
fn scale2(image: &RgbaImage) -> RgbaImage {
    let (w, h) = (image.width, image.height);
    let mut out = RgbaImage {
        width: w * 2,
        height: h * 2,
        pixels: vec![0; w * h * 4],
    };
    for y in 0..h {
        for x in 0..w {
            let c = image.pixels[y * w + x];
            let b = image.pixels[y.saturating_sub(1) * w + x];
            let d = image.pixels[y * w + x.saturating_sub(1)];
            let f = image.pixels[y * w + (x + 1).min(w - 1)];
            let h1 = image.pixels[(y + 1).min(h - 1) * w + x];
            let q = if b != h1 && d != f {
                [
                    if d == b { d } else { c },
                    if b == f { f } else { c },
                    if d == h1 { d } else { c },
                    if h1 == f { f } else { c },
                ]
            } else {
                [c; 4]
            };
            let p = y * 2 * w * 2 + x * 2;
            out.pixels[p] = q[0];
            out.pixels[p + 1] = q[1];
            out.pixels[p + w * 2] = q[2];
            out.pixels[p + w * 2 + 1] = q[3];
        }
    }
    out
}
/// Four-times edge-aware enlargement. This retains original artwork and is not a redraw.
pub fn enhance_original(image: &Image, palette: &[[u8; 3]; 256], transparent: bool) -> RgbaImage {
    let base = RgbaImage {
        width: image.width as usize,
        height: image.height as usize,
        pixels: image
            .pixels
            .iter()
            .map(|&p| {
                if p == 0 && transparent {
                    0
                } else {
                    let c = palette[p as usize];
                    0xff000000 | ((c[0] as u32) << 16) | ((c[1] as u32) << 8) | c[2] as u32
                }
            })
            .collect(),
    };
    let mut enlarged = scale2(&scale2(&base));
    // A local material filter, independent of the frame's bounding box. The old
    // normalized x/y lighting changed a character's colors as its frame size
    // changed. Identical neighborhoods now always receive identical colors.
    let material = enlarged.pixels.clone();
    let w = enlarged.width;
    let h = enlarged.height;
    for (i, pixel) in enlarged.pixels.iter_mut().enumerate() {
        if *pixel >> 24 == 0 {
            continue;
        }
        let rgb = [
            ((*pixel >> 16) & 255) as f32,
            ((*pixel >> 8) & 255) as f32,
            (*pixel & 255) as f32,
        ];
        if rgb.iter().copied().fold(0.0f32, f32::max) < 64.0 {
            continue;
        }
        let (x, y) = (i % w, i / w);
        let mut sum = [0.; 3];
        let mut weight = 0.;
        for dy in -2isize..=2 {
            for dx in -2isize..=2 {
                let xx = x as isize + dx;
                let yy = y as isize + dy;
                if xx < 0 || yy < 0 || xx >= w as isize || yy >= h as isize {
                    continue;
                }
                let c = material[yy as usize * w + xx as usize];
                let channels = [
                    ((c >> 16) & 255) as f32,
                    ((c >> 8) & 255) as f32,
                    (c & 255) as f32,
                ];
                // Never blur ink, transparency or detached parts into the surface.
                if c >> 24 == 0 || channels.iter().copied().fold(0., f32::max) < 64. {
                    continue;
                }
                let k = ((3 - dx.abs()) * (3 - dy.abs())) as f32;
                for j in 0..3 {
                    sum[j] += channels[j] * k;
                }
                weight += k;
            }
        }
        let c: [f32; 3] =
            std::array::from_fn(|j| (rgb[j] * 0.72 + sum[j] / weight.max(1.) * 0.28) / 255.);
        let blue = ((c[2] - c[0]).max(0.) * (c[2] - c[1]).max(0.)).sqrt();
        let cyan = (c[1].min(c[2]) - c[0]).max(0.);
        let yellow = (c[0].min(c[1]) - c[2]).max(0.);
        let green = (c[1] - c[0].max(c[2])).max(0.);
        let toned = [
            c[0] + 0.045 * blue + 0.06 * green,
            c[1] + 0.19 * blue - 0.10 * cyan - 0.06 * yellow,
            c[2] - 0.08 * blue - 0.025 * cyan + 0.22 * yellow + 0.12 * green,
        ];
        let out = toned.map(|v| (v.clamp(0., 1.) * 255.).round() as u32);
        *pixel = 0xff000000 | (out[0] << 16) | (out[1] << 8) | out[2];
    }
    enlarged
}
impl RemasterAssets {
    /// The generated pack is preserved for comparison. Its poses and scenery
    /// geometry failed the complete sprite review, so it is explicitly opt-in.
    pub fn load_with_painted_art(
        asset_dir: &Path,
        original: &AssetStore,
        painted: bool,
    ) -> Result<Self, String> {
        let mut sprites = BTreeMap::new();
        let mut remade_frames = 0;
        for (name, frames) in &original.sprites {
            let mut output = Vec::with_capacity(frames.len());
            for (index, frame) in frames.iter().enumerate() {
                let path = asset_dir
                    .join("sprites")
                    .join(name)
                    .join(format!("{index:03}.rgba"));
                let img = if painted && path.exists() {
                    let img = load_image(&path)?;
                    if img.width != frame.width as usize * 4
                        || img.height != frame.height as usize * 4
                    {
                        return Err(format!(
                            "{}: replacement must be four times source dimensions",
                            path.display()
                        ));
                    }
                    remade_frames += 1;
                    img
                } else {
                    enhance_original(frame, &original.palette, true)
                };
                output.push(img);
            }
            sprites.insert(name.clone(), output);
        }
        let mut backgrounds = BTreeMap::new();
        for (name, image) in &original.backgrounds {
            let path = asset_dir.join("backgrounds").join(format!("{name}.rgba"));
            backgrounds.insert(
                name.clone(),
                if painted && path.exists() {
                    load_image(path)?
                } else {
                    enhance_original(image, &original.palette, false)
                },
            );
        }
        for name in original.backgrounds.keys() {
            let path = asset_dir
                .join("backgrounds")
                .join(format!("{name}.wide.rgba"));
            if painted && path.exists() {
                backgrounds.insert(format!("{name}.wide"), load_image(path)?);
            }
        }
        Ok(Self {
            sprites,
            backgrounds,
            remade_frames,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn changing_frame_bounds_does_not_change_material_colors() {
        let mut palette = [[0; 3]; 256];
        palette[1] = [220, 120, 60];
        let small = Image {
            width: 12,
            height: 12,
            pixels: (0..144)
                .map(|i| {
                    let (x, y) = (i % 12, i / 12);
                    if (3..9).contains(&x) && (3..9).contains(&y) {
                        1
                    } else {
                        0
                    }
                })
                .collect(),
        };
        let mut padded = Image {
            width: 24,
            height: 24,
            pixels: vec![0; 24 * 24],
        };
        for y in 0..12 {
            for x in 0..12 {
                padded.pixels[(y + 6) * 24 + x + 6] = small.pixels[y * 12 + x];
            }
        }
        let a = enhance_original(&small, &palette, true);
        let b = enhance_original(&padded, &palette, true);
        for y in 0..48 {
            for x in 0..48 {
                assert_eq!(a.pixels[y * 48 + x], b.pixels[(y + 24) * 96 + x + 24]);
            }
        }
    }
    #[test]
    fn default_hd_preserves_all_source_poses_and_disconnected_parts() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let original = AssetStore::load(root.join("original")).unwrap();
        let art = RemasterAssets::load_with_painted_art(
            &root.join("assets/remastered"),
            &original,
            false,
        )
        .unwrap();
        assert_eq!(art.remade_frames, 0);
        assert_eq!(art.sprites.len(), 116);
        let colors: std::collections::HashSet<_> = art.backgrounds["ISLETEMP.SCR"]
            .pixels
            .iter()
            .copied()
            .collect();
        assert!(
            colors.len() > 256,
            "HD scenery must retain unrestricted RGB variation"
        );
        let mut total = 0;
        for (name, frames) in &original.sprites {
            for (i, frame) in frames.iter().enumerate() {
                let hd = &art.sprites[name][i];
                assert_eq!(
                    (hd.width, hd.height),
                    (frame.width as usize * 4, frame.height as usize * 4)
                );
                // A recovered pixel cannot lose all of its subpixels. This
                // includes 8x1 marks, detached hats and multi-part splashes.
                for y in 0..frame.height as usize {
                    for x in 0..frame.width as usize {
                        if frame.pixels[y * frame.width as usize + x] != 0 {
                            assert!(
                                (0..4).any(|dy| (0..4)
                                    .any(|dx| hd.pixels[(y * 4 + dy) * hd.width + x * 4 + dx]
                                        >> 24
                                        != 0)),
                                "{name} #{i} {x},{y}"
                            );
                        }
                    }
                }
                total += 1;
            }
        }
        assert_eq!(total, 2391);
    }
    #[test]
    fn preserve_registration_and_alpha() {
        let image = Image {
            width: 2,
            height: 2,
            pixels: vec![0, 1, 1, 0],
        };
        let mut palette = [[0; 3]; 256];
        palette[1] = [12, 34, 56];
        let rgba = enhance_original(&image, &palette, true);
        assert_eq!((rgba.width, rgba.height), (8, 8));
        assert_eq!(rgba.pixels[0], 0);
        assert_eq!(rgba.pixels[7], 0xff0c2238);
    }
    #[test]
    fn full_color_shading_preserves_ink_and_silhouette() {
        let image = Image {
            width: 32,
            height: 32,
            pixels: (0..1024)
                .map(|i| {
                    if i % 32 == 0 {
                        0
                    } else if i % 32 == 1 {
                        2
                    } else {
                        1 + (((i % 32) * (i % 32) + 13 * (i / 32) + (i % 32) * (i / 32)) % 6) as u8
                    }
                })
                .collect(),
        };
        let mut palette = [[0; 3]; 256];
        palette[1] = [220, 120, 60];
        palette[2] = [20, 20, 20];
        palette[3] = [230, 190, 80];
        palette[4] = [100, 210, 130];
        palette[5] = [190, 120, 150];
        palette[6] = [190, 210, 230];
        let enhanced = enhance_original(&image, &palette, true);
        let colors: std::collections::BTreeSet<_> = enhanced
            .pixels
            .iter()
            .copied()
            .filter(|p| p >> 24 != 0)
            .collect();
        assert!(
            colors.len() > 16,
            "Material filtering should interpolate the six input colors: {}",
            colors.len()
        );
        assert_eq!(
            enhanced.pixels[0], 0,
            "Transparent silhouette must remain transparent"
        );
        assert_eq!(
            enhanced.pixels[4], 0xff141414,
            "Contour ink must keep its exact original color"
        );
        assert!(enhanced
            .pixels
            .iter()
            .all(|p| p >> 24 == 0 || p >> 24 == 255));
    }
}
