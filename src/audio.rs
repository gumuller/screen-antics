//! Original Win16 NE sound resource decoding. Samples are unsigned 8-bit mono,
//! 11025 Hz, in ascending numeric resource ID order (index zero is ID 1).

pub const SAMPLE_RATE: i32 = 11_025;
// The executable's RT_NAMETABLE gives this exact mapping; names are
// lexicographically ordered rather than numerically ordered.
pub const WAVE_SFX_NUMBERS: [u8; 23] = [
    1, 10, 12, 14, 15, 16, 17, 18, 19, 2, 20, 21, 22, 23, 24, 25, 3, 4, 5, 6, 7, 8, 9,
];

pub fn index_for_sfx(number: u8) -> Option<usize> {
    WAVE_SFX_NUMBERS.iter().position(|&value| value == number)
}

fn word(bytes: &[u8], at: usize) -> Result<usize, String> {
    let b = bytes.get(at..at + 2).ok_or("Truncated NE word")?;
    Ok(u16::from_le_bytes([b[0], b[1]]) as usize)
}

fn dword(bytes: &[u8], at: usize) -> Result<usize, String> {
    let b = bytes.get(at..at + 4).ok_or("Truncated RIFF/NE dword")?;
    Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
}

fn pcm(wave: &[u8]) -> Result<Vec<u8>, String> {
    if wave.get(..4) != Some(b"RIFF") || wave.get(8..12) != Some(b"WAVE") {
        return Err("Invalid WAVE resource".into());
    }
    let end = dword(wave, 4)?.checked_add(8).ok_or("RIFF size overflow")?;
    let wave = wave.get(..end).ok_or("Truncated WAVE resource")?;
    let mut at = 12;
    let mut format_ok = false;
    let mut samples = None;
    while at + 8 <= wave.len() {
        let size = dword(wave, at + 4)?;
        let start = at + 8;
        let end = start.checked_add(size).ok_or("WAVE chunk size overflow")?;
        let data = wave.get(start..end).ok_or("Truncated WAVE chunk")?;
        match &wave[at..at + 4] {
            b"fmt " => {
                format_ok = word(data, 0)? == 1
                    && word(data, 2)? == 1
                    && dword(data, 4)? == SAMPLE_RATE as usize
                    && word(data, 12)? == 1
                    && word(data, 14)? == 8;
            }
            b"data" => samples = Some(data.to_vec()),
            _ => {}
        }
        at = end.checked_add(size & 1).ok_or("WAVE alignment overflow")?;
    }
    if !format_ok {
        return Err("Expected original mono 8-bit 11025 Hz PCM".into());
    }
    samples.ok_or_else(|| "Missing WAVE samples".into())
}

pub fn waves(exe: &[u8]) -> Result<Vec<Vec<u8>>, String> {
    if exe.get(..2) != Some(b"MZ") {
        return Err("Expected original MZ/NE executable".into());
    }
    let ne = dword(exe, 0x3c)?;
    if exe.get(ne..ne + 2) != Some(b"NE") {
        return Err("Missing NE executable header".into());
    }
    let table = ne
        .checked_add(word(exe, ne + 0x24)?)
        .ok_or("NE table overflow")?;
    let shift = word(exe, table)? as u32;
    if shift > 20 {
        return Err("Invalid NE resource alignment".into());
    }
    let mut at = table + 2;
    let mut found = Vec::new();
    loop {
        let kind = word(exe, at)?;
        if kind == 0 {
            break;
        }
        let count = word(exe, at + 2)?;
        at += 8;
        for _ in 0..count {
            let start = word(exe, at)?
                .checked_shl(shift)
                .ok_or("Resource offset overflow")?;
            let length = word(exe, at + 2)?
                .checked_shl(shift)
                .ok_or("Resource size overflow")?;
            let id = word(exe, at + 6)?;
            let end = start.checked_add(length).ok_or("Resource end overflow")?;
            let data = exe.get(start..end).ok_or("Truncated NE resource")?;
            if kind == 0xf005 && data.get(..4) == Some(b"RIFF") {
                found.push((id & 0x7fff, pcm(data)?));
            }
            at += 12;
        }
    }
    found.sort_by_key(|entry| entry.0);
    if found.is_empty() {
        return Err("No original WAVE resources found".into());
    }
    Ok(found.into_iter().map(|entry| entry.1).collect())
}
