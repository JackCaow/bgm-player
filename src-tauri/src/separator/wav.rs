use std::path::Path;

/// Write two f32 channels (clamped to [-1,1]) as a 16-bit PCM stereo WAV.
pub fn write_stereo_wav(
    path: &Path,
    left: &[f32],
    right: &[f32],
    sample_rate: u32,
) -> Result<(), String> {
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut w = hound::WavWriter::create(path, spec).map_err(|e| e.to_string())?;
    let n = left.len().min(right.len());
    for i in 0..n {
        for s in [left[i], right[i]] {
            let v = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
            w.write_sample(v).map_err(|e| e.to_string())?;
        }
    }
    w.finalize().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip_f32_to_i16_wav() {
        let dir = std::env::temp_dir().join("bgm_wav_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("t.wav");
        // 2ch, 4 frames
        let left = vec![0.0_f32, 0.5, -0.5, 1.0];
        let right = vec![0.0_f32, -0.5, 0.5, -1.0];
        write_stereo_wav(&path, &left, &right, 44100).unwrap();
        let mut reader = hound::WavReader::open(&path).unwrap();
        assert_eq!(reader.spec().channels, 2);
        assert_eq!(reader.spec().sample_rate, 44100);
        let samples: Vec<i16> = reader.samples::<i16>().map(|s| s.unwrap()).collect();
        assert_eq!(samples.len(), 8); // 4 frames * 2ch interleaved
        assert_eq!(samples[0], 0);
        assert_eq!(samples[2], (0.5 * 32767.0) as i16);
    }
}
