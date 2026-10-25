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

pub const OUTPUT_RATE: u32 = 44_100;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioMode {
    Remastered,
    Original,
}
impl AudioMode {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "remastered" => Ok(Self::Remastered),
            "original" => Ok(Self::Original),
            _ => Err("Audio must be remastered or original".into()),
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Remastered => "remastered",
            Self::Original => "original",
        }
    }
    pub fn toggle(&mut self) {
        *self = match self {
            Self::Remastered => Self::Original,
            Self::Original => Self::Remastered,
        };
    }
}

/// Both sources share the output device format, so toggling never changes speed
/// or drops/reopens the audio device. The SFX suffix mapping remains authoritative.
pub struct SoundBank {
    original: Vec<Vec<i16>>,
    remastered: Vec<Option<Vec<i16>>>,
}
impl SoundBank {
    pub fn load(exe: &[u8], pack: Option<&std::path::Path>) -> Result<Self, String> {
        let original = waves(exe)?
            .iter()
            .map(|p| {
                resample(
                    &p.iter()
                        .map(|&x| ((x as i16) - 128) << 8)
                        .collect::<Vec<_>>(),
                    SAMPLE_RATE as u32,
                )
            })
            .collect();
        let mut remastered = Vec::new();
        for n in WAVE_SFX_NUMBERS {
            let path = pack.map(|p| p.join("audio").join(format!("WAVESFX{n:02}.wav")));
            let pcm = if let Some(path) = path.filter(|p| p.exists()) {
                let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
                let (rate, pcm) =
                    decode_pcm16(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
                Some(resample(&pcm, rate))
            } else {
                None
            };
            remastered.push(pcm);
        }
        Ok(Self {
            original,
            remastered,
        })
    }
    pub fn replacement_count(&self) -> usize {
        self.remastered.iter().filter(|p| p.is_some()).count()
    }
    pub fn len(&self) -> usize {
        self.original.len()
    }
    pub fn sound(&self, number: u16, mode: AudioMode) -> Option<&[i16]> {
        let i = index_for_sfx(u8::try_from(number).ok()?)?;
        if mode == AudioMode::Remastered {
            if let Some(pcm) = self.remastered.get(i).and_then(|p| p.as_ref()) {
                return Some(pcm);
            }
        }
        self.original.get(i).map(Vec::as_slice)
    }
}

fn decode_pcm16(bytes: &[u8]) -> Result<(u32, Vec<i16>), String> {
    if bytes.get(..4) != Some(b"RIFF") || bytes.get(8..12) != Some(b"WAVE") {
        return Err("Expected RIFF WAVE".into());
    }
    let end = dword(bytes, 4)?
        .checked_add(8)
        .ok_or("RIFF size overflow")?;
    let bytes = bytes.get(..end).ok_or("Truncated WAVE")?;
    let mut at = 12;
    let mut rate = None;
    let mut data = None;
    while at + 8 <= bytes.len() {
        let size = dword(bytes, at + 4)?;
        let start = at + 8;
        let end = start.checked_add(size).ok_or("WAVE size overflow")?;
        let chunk = bytes.get(start..end).ok_or("Truncated WAVE chunk")?;
        match &bytes[at..at + 4] {
            b"fmt " => {
                let hz = dword(chunk, 4)? as u32;
                if word(chunk, 0)? != 1
                    || word(chunk, 2)? != 1
                    || word(chunk, 12)? != 2
                    || word(chunk, 14)? != 16
                    || !matches!(hz, 44_100 | 48_000)
                    || dword(chunk, 8)? != hz as usize * 2
                {
                    return Err("Expected mono 16-bit PCM at 44100 or 48000 Hz".into());
                }
                rate = Some(hz);
            }
            b"data" => {
                if chunk.is_empty() || chunk.len() % 2 != 0 {
                    return Err("Invalid PCM16 sample data".into());
                }
                data = Some(
                    chunk
                        .chunks_exact(2)
                        .map(|b| i16::from_le_bytes([b[0], b[1]]))
                        .collect(),
                );
            }
            _ => {}
        }
        at = end.checked_add(size % 2).ok_or("WAVE padding overflow")?;
    }
    Ok((
        rate.ok_or("Missing PCM format")?,
        data.ok_or("Missing PCM samples")?,
    ))
}

fn resample(pcm: &[i16], rate: u32) -> Vec<i16> {
    if rate == OUTPUT_RATE {
        return pcm.to_vec();
    }
    if pcm.is_empty() {
        return Vec::new();
    }
    let count = (pcm.len() as u64 * OUTPUT_RATE as u64 + rate as u64 / 2) / rate as u64;
    (0..count)
        .map(|i| {
            let n = i * rate as u64;
            let p = (n / OUTPUT_RATE as u64) as usize;
            let fraction = (n % OUTPUT_RATE as u64) as i64;
            let a = pcm[p.min(pcm.len() - 1)] as i64;
            let b = pcm[(p + 1).min(pcm.len() - 1)] as i64;
            (a + (b - a) * fraction / OUTPUT_RATE as i64) as i16
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn wav(samples: &[i16], rate: u32) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(36 + samples.len() as u32 * 2).to_le_bytes());
        out.extend_from_slice(b"WAVEfmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&rate.to_le_bytes());
        out.extend_from_slice(&(rate * 2).to_le_bytes());
        out.extend_from_slice(&2u16.to_le_bytes());
        out.extend_from_slice(&16u16.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&(samples.len() as u32 * 2).to_le_bytes());
        for s in samples {
            out.extend_from_slice(&s.to_le_bytes());
        }
        out
    }
    #[test]
    fn pcm16_preserves_signed_samples_and_rejects_truncated_waves() {
        let samples = [i16::MIN, -257, 0, 256, i16::MAX];
        let bytes = wav(&samples, OUTPUT_RATE);
        let (rate, decoded) = decode_pcm16(&bytes).unwrap();
        assert_eq!(rate, OUTPUT_RATE);
        assert_eq!(decoded, samples);
        assert_eq!(resample(&decoded, rate), samples);
        for len in [0, 4, 12, 30, bytes.len() - 1] {
            assert!(decode_pcm16(&bytes[..len]).is_err());
        }
    }
    #[test]
    fn conversion_preserves_duration_at_both_supported_input_rates() {
        for rate in [11_025, 48_000] {
            let input = vec![-12345; rate as usize];
            let output = resample(&input, rate);
            assert_eq!(output.len(), 44_100);
            assert!(output.iter().all(|&s| s == -12345));
        }
    }
    #[test]
    fn source_toggle_keeps_verified_sfx_suffix_mapping() {
        let original = (0..23).map(|i| vec![i]).collect();
        let remastered = (0..23).map(|i| Some(vec![100 + i])).collect();
        let bank = SoundBank {
            original,
            remastered,
        };
        assert_eq!(bank.sound(10, AudioMode::Original), Some([1i16].as_slice()));
        assert_eq!(
            bank.sound(10, AudioMode::Remastered),
            Some([101i16].as_slice())
        );
        assert_eq!(bank.sound(11, AudioMode::Remastered), None);
        assert_eq!(bank.sound(256, AudioMode::Original), None);
    }
}
