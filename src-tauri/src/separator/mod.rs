mod chunking;
mod decode;
mod model;
mod wav;

use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct OutputPaths {
    pub vocals: PathBuf,
    pub no_vocals: PathBuf,
}

#[derive(Debug)]
pub enum SeparatorError {
    ModelMissing,
    Decode(String),
    Inference(String),
    Io(String),
}

impl std::fmt::Display for SeparatorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SeparatorError::ModelMissing => write!(f, "onnx model missing"),
            SeparatorError::Decode(e) => write!(f, "decode failed: {e}"),
            SeparatorError::Inference(e) => write!(f, "inference failed: {e}"),
            SeparatorError::Io(e) => write!(f, "io failed: {e}"),
        }
    }
}

/// Compute the scalar normalization stats used by the Python reference:
/// `ref[i] = (left[i] + right[i]) / 2`, then return `(ref.mean(), ref.std())`
/// where std is the population std (÷N). If std < 1e-8, std is replaced by 1.0
/// so normalization is a no-op divide. f64 is used internally for stability.
fn compute_ref_mean_std(left: &[f32], right: &[f32]) -> (f32, f32) {
    let n = left.len().min(right.len());
    if n == 0 {
        return (0.0, 1.0);
    }
    let mut sum = 0.0_f64;
    for i in 0..n {
        sum += (left[i] as f64 + right[i] as f64) / 2.0;
    }
    let mean = sum / n as f64;
    let mut var_sum = 0.0_f64;
    for i in 0..n {
        let r = (left[i] as f64 + right[i] as f64) / 2.0;
        let d = r - mean;
        var_sum += d * d;
    }
    let std = (var_sum / n as f64).sqrt();
    let std = if std < 1e-8 { 1.0 } else { std };
    (mean as f32, std as f32)
}

/// Separate `input` into vocals + no_vocals (BGM) using the bundled htdemucs ONNX.
/// `on_progress(progress_0_to_100, status)` mirrors the existing extraction-progress events.
pub fn separate_htdemucs_2track(
    app_resource_dir: &Path,
    input: &Path,
    output_dir: &Path,
    ffmpeg_path: &str,
    on_progress: &dyn Fn(f32, &str),
) -> Result<OutputPaths, SeparatorError> {
    on_progress(5.0, "正在启动音频分离引擎...");
    let model_path = model::resolve_model_path(app_resource_dir).ok_or(SeparatorError::ModelMissing)?;
    let mut m = model::HtdemucsModel::load(&model_path)?;

    let (mut left, mut right) =
        decode::decode_stereo_f32(ffmpeg_path, input).map_err(SeparatorError::Decode)?;
    let n = left.len();
    if n == 0 {
        return Err(SeparatorError::Decode("empty audio".into()));
    }

    // Normalize: scalar mean/std over ref=(L+R)/2, applied to BOTH channels.
    let (mean, std) = compute_ref_mean_std(&left, &right);
    for x in left.iter_mut() {
        *x = (*x - mean) / std;
    }
    for x in right.iter_mut() {
        *x = (*x - mean) / std;
    }

    let seg = m.segment_len();
    // demucs overlap = 0.25 -> stride = seg * 0.75.
    let stride = ((seg as f32) * 0.75).round() as usize;
    let plan = chunking::SegmentPlan::new(n, seg, stride);

    // One OverlapAdder per (source, channel).
    let mut adders: Vec<[chunking::OverlapAdder; 2]> = (0..model::SOURCES)
        .map(|_| {
            [
                chunking::OverlapAdder::new(n, seg),
                chunking::OverlapAdder::new(n, seg),
            ]
        })
        .collect();

    let total = plan.segments.len().max(1);
    for (k, s) in plan.segments.iter().enumerate() {
        let lc = plan.extract(&left, s);
        let rc = plan.extract(&right, s);
        let stems = m.run_chunk(&lc, &rc)?; // [SOURCES][2][seg]
        for (src, adder) in adders.iter_mut().enumerate() {
            adder[0].add(s.start, &stems[src][0]);
            adder[1].add(s.start, &stems[src][1]);
        }
        on_progress(
            5.0 + 90.0 * (k as f32 + 1.0) / total as f32,
            "正在分离音轨...",
        );
    }

    // Finish overlap-add, then denormalize every source/channel.
    let mut srcs: Vec<[Vec<f32>; 2]> = adders
        .into_iter()
        .map(|[a, b]| [a.finish(), b.finish()])
        .collect();
    for s in srcs.iter_mut() {
        for ch in s.iter_mut() {
            for x in ch.iter_mut() {
                *x = *x * std + mean;
            }
        }
    }

    // 2-track: vocals = sources[VOCALS_IDX]; BGM = sum of the other 3 sources.
    let voc_l = srcs[model::VOCALS_IDX][0].clone();
    let voc_r = srcs[model::VOCALS_IDX][1].clone();
    let mut bgm_l = vec![0.0_f32; n];
    let mut bgm_r = vec![0.0_f32; n];
    for (src, s) in srcs.iter().enumerate() {
        if src == model::VOCALS_IDX {
            continue;
        }
        for i in 0..n {
            bgm_l[i] += s[0][i];
            bgm_r[i] += s[1][i];
        }
    }

    on_progress(96.0, "正在保存文件...");
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let dir = output_dir.join("htdemucs").join(stem);
    std::fs::create_dir_all(&dir).map_err(|e| SeparatorError::Io(e.to_string()))?;
    let vocals = dir.join("vocals.wav");
    let no_vocals = dir.join("no_vocals.wav");
    wav::write_stereo_wav(&vocals, &voc_l, &voc_r, decode::SAMPLE_RATE).map_err(SeparatorError::Io)?;
    wav::write_stereo_wav(&no_vocals, &bgm_l, &bgm_r, decode::SAMPLE_RATE)
        .map_err(SeparatorError::Io)?;
    on_progress(100.0, "处理完成");
    Ok(OutputPaths { vocals, no_vocals })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    /// End-to-end smoke test: full pipeline on a short stereo sample.
    /// Requires both the bundled ONNX model and ffmpeg to be present;
    /// skips (does not fail) if either is missing.
    #[test]
    fn separates_short_sample_into_two_wavs() {
        let cwd = std::env::current_dir().unwrap();
        if model::resolve_model_path(&cwd).is_none() {
            eprintln!("model absent; skipping end-to-end smoke test");
            return;
        }

        // Prepare the input sample (~6s stereo). Use the cached one if present,
        // otherwise generate it with ffmpeg.
        let sample_dir = std::path::Path::new("/tmp/bgm-test");
        let sample = sample_dir.join("sample.wav");
        if !sample.exists() {
            if std::fs::create_dir_all(sample_dir).is_err() {
                eprintln!("cannot create sample dir; skipping");
                return;
            }
            let gen = Command::new("ffmpeg")
                .args([
                    "-y",
                    "-v",
                    "error",
                    "-f",
                    "lavfi",
                    "-i",
                    "sine=frequency=440:duration=6",
                    "-ac",
                    "2",
                    "-ar",
                    "44100",
                ])
                .arg(&sample)
                .status();
            if gen.map(|s| !s.success()).unwrap_or(true) {
                eprintln!("ffmpeg unavailable; skipping end-to-end smoke test");
                return;
            }
        }

        let out_dir = std::env::temp_dir().join("bgm_separator_e2e_out");
        let _ = std::fs::remove_dir_all(&out_dir);
        std::fs::create_dir_all(&out_dir).unwrap();

        let t = std::time::Instant::now();
        let result = separate_htdemucs_2track(&cwd, &sample, &out_dir, "ffmpeg", &|p, msg| {
            eprintln!("progress {p:.1}%: {msg}");
        })
        .expect("separation should succeed");
        eprintln!("separation finished in {:?}", t.elapsed());

        let vocals_meta = std::fs::metadata(&result.vocals).expect("vocals.wav must exist");
        let no_vocals_meta =
            std::fs::metadata(&result.no_vocals).expect("no_vocals.wav must exist");
        eprintln!(
            "vocals.wav: {} bytes ({})",
            vocals_meta.len(),
            result.vocals.display()
        );
        eprintln!(
            "no_vocals.wav: {} bytes ({})",
            no_vocals_meta.len(),
            result.no_vocals.display()
        );
        assert!(vocals_meta.len() > 0, "vocals.wav must be non-empty");
        assert!(no_vocals_meta.len() > 0, "no_vocals.wav must be non-empty");
    }

    #[test]
    fn ref_mean_std_constant_signal_guards_zero_std() {
        // Constant signal -> std == 0 -> guarded to 1.0; mean is the constant.
        let l = vec![0.5_f32; 100];
        let r = vec![0.5_f32; 100];
        let (mean, std) = compute_ref_mean_std(&l, &r);
        assert!((mean - 0.5).abs() < 1e-6, "mean {mean}");
        assert!((std - 1.0).abs() < 1e-6, "std {std}");
    }

    #[test]
    fn ref_mean_std_matches_population_formula() {
        // ref = (L+R)/2 = [0,1,2,3]; mean=1.5; popvar = mean((x-1.5)^2)=1.25; std=sqrt(1.25)
        let l = vec![0.0_f32, 1.0, 2.0, 3.0];
        let r = vec![0.0_f32, 1.0, 2.0, 3.0];
        let (mean, std) = compute_ref_mean_std(&l, &r);
        assert!((mean - 1.5).abs() < 1e-5, "mean {mean}");
        assert!((std - 1.25_f32.sqrt()).abs() < 1e-5, "std {std}");
    }
}
