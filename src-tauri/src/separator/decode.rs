use std::path::Path;
use std::process::Command;

pub const SAMPLE_RATE: u32 = 44100;

/// Decode any input to 2-channel f32 PCM at 44100 Hz via ffmpeg.
/// Returns (left, right) deinterleaved.
pub fn decode_stereo_f32(
    ffmpeg_path: &str,
    input: &Path,
) -> Result<(Vec<f32>, Vec<f32>), String> {
    let out = Command::new(ffmpeg_path)
        .args(["-v", "error", "-i"])
        .arg(input)
        .args(["-f", "f32le", "-acodec", "pcm_f32le", "-ac", "2", "-ar", "44100", "-"])
        .output()
        .map_err(|e| format!("spawn ffmpeg: {e}"))?;
    if !out.status.success() {
        return Err(format!("ffmpeg: {}", String::from_utf8_lossy(&out.stderr)));
    }
    let bytes = out.stdout;
    if bytes.len() < 8 || bytes.len() % 8 != 0 {
        return Err(format!("unexpected PCM length {}", bytes.len()));
    }
    let frames = bytes.len() / 8; // 2ch * 4 bytes
    let mut left = Vec::with_capacity(frames);
    let mut right = Vec::with_capacity(frames);
    for f in 0..frames {
        let o = f * 8;
        left.push(f32::from_le_bytes([bytes[o], bytes[o+1], bytes[o+2], bytes[o+3]]));
        right.push(f32::from_le_bytes([bytes[o+4], bytes[o+5], bytes[o+6], bytes[o+7]]));
    }
    Ok((left, right))
}

#[cfg(test)]
mod tests {
    use super::*;
    // Integration test: requires ffmpeg on PATH. Generates a 1s tone and decodes it.
    #[test]
    fn decodes_generated_tone() {
        let dir = std::env::temp_dir().join("bgm_decode_test");
        std::fs::create_dir_all(&dir).unwrap();
        let wav = dir.join("tone.wav");
        let gen = Command::new("ffmpeg")
            .args(["-y","-v","error","-f","lavfi","-i","sine=frequency=440:duration=1",
                   "-ac","2","-ar","44100"]).arg(&wav).status();
        if gen.map(|s| !s.success()).unwrap_or(true) { eprintln!("ffmpeg unavailable; skipping"); return; }
        let (l, r) = decode_stereo_f32("ffmpeg", &wav).unwrap();
        assert!((l.len() as i64 - 44100).abs() < 2000);
        assert_eq!(l.len(), r.len());
    }
}
