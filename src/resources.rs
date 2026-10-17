//! Original DGDS artwork reader. File records and chunk sizes are little endian;
//! bit 31 of a chunk size marks a container. Image pixels are packed high nibble first.
use std::{collections::BTreeMap, fs, path::Path};

#[derive(Clone, Debug)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

pub struct AssetStore {
    pub palette: [[u8; 3]; 256],
    pub backgrounds: BTreeMap<String, Image>,
    pub sprites: BTreeMap<String, Vec<Image>>,
    pub raw: BTreeMap<String, Vec<u8>>,
}

fn u16le(data: &[u8], p: usize) -> Result<usize, String> {
    let b = data.get(p..p + 2).ok_or("Truncated 16-bit value")?;
    Ok(u16::from_le_bytes([b[0], b[1]]) as usize)
}
fn u32le(data: &[u8], p: usize) -> Result<usize, String> {
    let b = data.get(p..p + 4).ok_or("Truncated 32-bit value")?;
    Ok(u32::from_le_bytes(b.try_into().unwrap()) as usize)
}

pub fn chunks(data: &[u8]) -> Result<Vec<([u8; 4], &[u8])>, String> {
    let mut result = Vec::new();
    let mut p = 0;
    while p < data.len() {
        let tag: [u8; 4] = data
            .get(p..p + 4)
            .ok_or("Truncated chunk tag")?
            .try_into()
            .unwrap();
        let size = u32le(data, p + 4)?;
        p += 8;
        let end = p
            .checked_add(size & 0x7fff_ffff)
            .ok_or("Chunk size overflow")?;
        let body = data.get(p..end).ok_or("Chunk extends beyond resource")?;
        if size & 0x8000_0000 != 0 {
            result.extend(chunks(body)?);
        } else {
            result.push((tag, body));
        }
        p = end;
    }
    Ok(result)
}

/// Decode the DGDS five-byte compression header followed by raw, RLE or LZW data.
pub fn decompress(data: &[u8]) -> Result<Vec<u8>, String> {
    let method = *data.first().ok_or("Missing compression header")?;
    let size = u32le(data, 1)?;
    if size > 64 * 1024 * 1024 {
        return Err("Unreasonable decompressed size".into());
    }
    let input = data.get(5..).ok_or("Truncated compression header")?;
    let mut output = Vec::with_capacity(size);
    match method {
        0 => output.extend_from_slice(input.get(..size).ok_or("Truncated raw data")?),
        1 => {
            let mut p = 0;
            while output.len() < size {
                let command = *input.get(p).ok_or("Truncated RLE command")?;
                p += 1;
                let count = (command & 127) as usize;
                if command < 128 {
                    let literal = input.get(p..p + count).ok_or("Truncated RLE literal")?;
                    output.extend_from_slice(&literal[..count.min(size - output.len())]);
                    p += count;
                } else if count != 0 {
                    let value = *input.get(p).ok_or("Truncated RLE run")?;
                    p += 1;
                    output.resize((output.len() + count).min(size), value);
                }
            }
        }
        2 => {
            let mut bits = Bits {
                data: input,
                position: 0,
            };
            let mut dictionary: Vec<Vec<u8>> = (0..256).map(|i| vec![i as u8]).collect();
            dictionary.push(Vec::new()); // 256 is the dictionary reset instruction.
            let mut previous: Vec<u8> = Vec::new();
            let mut width = 9;
            let mut group_bits = 0;
            while output.len() < size {
                let code = bits.read(width)?;
                group_bits = (group_bits + width) % (width * 8);
                if code == 256 {
                    if group_bits != 0 {
                        bits.read(width * 8 - group_bits)?;
                    }
                    // Reset records are padded to the end of a byte/code group.
                    dictionary.truncate(257);
                    previous.clear();
                    width = 9;
                    group_bits = 0;
                    continue;
                }
                let entry = if code < dictionary.len() {
                    dictionary[code].clone()
                } else if code == dictionary.len() && !previous.is_empty() {
                    let mut next = previous.clone();
                    next.push(previous[0]);
                    next
                } else {
                    return Err(format!("Invalid LZW code {code} at bit {}", bits.position));
                };
                if entry.is_empty() {
                    return Err("Empty LZW entry".into());
                }
                output.extend_from_slice(&entry[..entry.len().min(size - output.len())]);
                if !previous.is_empty() && dictionary.len() < 4096 {
                    previous.push(entry[0]);
                    dictionary.push(previous);
                    group_bits = 0;
                    if dictionary.len() == 1 << width && width < 12 {
                        width += 1;
                    }
                }
                previous = entry;
            }
        }
        _ => return Err(format!("Unsupported DGDS compression {method}")),
    }
    Ok(output)
}
struct Bits<'a> {
    data: &'a [u8],
    position: usize,
}
impl Bits<'_> {
    fn read(&mut self, count: usize) -> Result<usize, String> {
        if self.position + count > self.data.len() * 8 {
            return Err("Truncated LZW stream".into());
        }
        let mut value = 0;
        // Padding skips may exceed the machine word; only code values need accumulation.
        for n in 0..count {
            let p = self.position + n;
            if n < usize::BITS as usize {
                value |= (((self.data[p / 8] >> (p % 8)) & 1) as usize) << n;
            }
        }
        self.position += count;
        Ok(value)
    }
}

fn images(data: &[u8], screen: bool) -> Result<Vec<Image>, String> {
    let mut frames = Vec::new();
    for (tag, body) in chunks(data)? {
        match &tag {
            b"DIM:" if screen => {
                let (w, h) = (u16le(body, 0)?, u16le(body, 2)?);
                frames.push(Image {
                    width: w as u32,
                    height: h as u32,
                    pixels: vec![0; w * h],
                });
            }
            b"INF:" if !screen => {
                let count = u16le(body, 0)?;
                if count > 1024 {
                    return Err("Unreasonable frame count".into());
                }
                for i in 0..count {
                    let (w, h) = (u16le(body, 2 + i * 2)?, u16le(body, 2 + count * 2 + i * 2)?);
                    if w * h > 4_000_000 {
                        return Err("Unreasonable image size".into());
                    }
                    frames.push(Image {
                        width: w as u32,
                        height: h as u32,
                        pixels: vec![0; w * h],
                    });
                }
            }
            b"BIN:" | b"VGA:" => {
                let unpacked = decompress(body)?;
                let mut offset = 0;
                for image in &mut frames {
                    let bytes = image.pixels.len().div_ceil(2);
                    let packed = unpacked
                        .get(offset..offset + bytes)
                        .ok_or("Not enough image pixels")?;
                    for (i, pixel) in image.pixels.iter_mut().enumerate() {
                        let nibble = if i % 2 == 0 {
                            packed[i / 2] >> 4
                        } else {
                            packed[i / 2] & 15
                        };
                        *pixel |= if &tag == b"VGA:" { nibble << 4 } else { nibble };
                    }
                    offset += bytes;
                }
                if offset != unpacked.len() {
                    return Err(format!(
                        "Image data has {} unused bytes",
                        unpacked.len() - offset
                    ));
                }
            }
            _ => {
                return Err(format!(
                    "Unsupported image chunk {}",
                    String::from_utf8_lossy(&tag)
                ))
            }
        }
    }
    if frames.is_empty() {
        return Err("No image dimensions".into());
    }
    Ok(frames)
}

impl AssetStore {
    pub fn load(dir: impl AsRef<Path>) -> Result<Self, String> {
        let archive = fs::read(dir.as_ref().join("RESOURCE.001"))
            .map_err(|e| format!("Cannot read original RESOURCE.001: {e}"))?;
        let mut raw = BTreeMap::new();
        let mut p = 0;
        while p < archive.len() {
            let name = archive.get(p..p + 13).ok_or("Truncated resource name")?;
            let name =
                String::from_utf8_lossy(&name[..name.iter().position(|&b| b == 0).unwrap_or(13)])
                    .to_uppercase();
            let size = u32le(&archive, p + 13)?;
            p += 17;
            let body = archive
                .get(p..p + size)
                .ok_or("Truncated resource record")?;
            raw.insert(name, body.to_vec());
            p += size;
        }
        let mut result = Self {
            palette: [[0; 3]; 256],
            backgrounds: BTreeMap::new(),
            sprites: BTreeMap::new(),
            raw,
        };
        let pal = result
            .raw
            .get("JOHNCAST.PAL")
            .ok_or("Missing JOHNCAST.PAL")?;
        for (tag, body) in chunks(pal)? {
            if &tag != b"VGA:" || body.len() != 768 {
                return Err("Unsupported palette".into());
            }
            for (i, rgb) in body.chunks_exact(3).enumerate() {
                for (j, &channel) in rgb.iter().enumerate() {
                    result.palette[i][j] = (channel << 2) | (channel >> 4);
                }
            }
        }
        for (name, body) in &result.raw {
            if name.ends_with(".BMP") {
                result.sprites.insert(
                    name.clone(),
                    images(body, false).map_err(|e| format!("{name}: {e}"))?,
                );
            } else if name.ends_with(".SCR") {
                result.backgrounds.insert(
                    name.clone(),
                    images(body, true)
                        .map_err(|e| format!("{name}: {e}"))?
                        .remove(0),
                );
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rle_runs_and_literals() {
        assert_eq!(
            decompress(&[1, 6, 0, 0, 0, 3, 1, 2, 3, 0x83, 9]).unwrap(),
            [1, 2, 3, 9, 9, 9]
        );
    }
    #[test]
    fn original_archive_decodes() {
        let assets =
            AssetStore::load(Path::new(env!("CARGO_MANIFEST_DIR")).join("original")).unwrap();
        assert_eq!(assets.backgrounds.len(), 10);
        assert_eq!(assets.sprites.len(), 116);
        assert_eq!(assets.backgrounds["ISLETEMP.SCR"].width, 640);
        assert_eq!(assets.backgrounds["ISLETEMP.SCR"].height, 350);
        assert!(assets.sprites["JOHNWALK.BMP"].len() > 20);
    }
}
