//! ONNX Runtime wrapper around the bundled htdemucs model.
//!
//! Built against **ort 2.0.0-rc.12** and **ndarray 0.17**. The relevant API
//! shape (verified against the resolved crate source) is:
//!   - `ort::session::Session::builder()? -> SessionBuilder`
//!   - `SessionBuilder::with_optimization_level(GraphOptimizationLevel::Level3)?`
//!   - `SessionBuilder::with_intra_threads(n)?`
//!   - `SessionBuilder::commit_from_file(path)? -> Session`
//!   - input tensor: `ort::value::Tensor::<f32>::from_array((shape, data))?`
//!     where `shape` is `Vec<i64>` and `data` is row-major `Vec<f32>`.
//!   - run: `session.run(ort::inputs![tensor])? -> SessionOutputs`
//!     (a single positional input is bound to the model's sole input).
//!   - extract: `outputs.get("output")?.try_extract_tensor::<f32>()? -> (&Shape, &[f32])`
//!     where `Shape` derefs to `[i64]`. We index the flat slice manually for the
//!     `[1, 4, 2, seg]` row-major layout.
//!
//! Execution provider: CPU only (the default when no EP is registered; the
//! prebuilt CPU binary is supplied by the `download-binaries` feature). CoreML
//! is intentionally NOT enabled.

use std::path::{Path, PathBuf};

use ort::session::{builder::GraphOptimizationLevel, Session};
use ort::value::Tensor;

use crate::separator::SeparatorError;

/// Number of stems htdemucs produces: `[drums, bass, other, vocals]`.
pub const SOURCES: usize = 4;
/// Index of the vocals stem within the source dimension.
pub const VOCALS_IDX: usize = 3;
/// Static segment length (samples per channel) the exported model expects.
pub const SEGMENT_LEN: usize = 343980;

/// Model output tensor name (static in the exported graph).
const OUTPUT_NAME: &str = "output";

/// Resolve the bundled model file.
///
/// Production: `<resource_dir>/models/htdemucs.onnx`.
/// Dev fallbacks (relative to the current working directory):
/// `resources/models/htdemucs.onnx` and `src-tauri/resources/models/htdemucs.onnx`.
pub fn resolve_model_path(resource_dir: &Path) -> Option<PathBuf> {
    let prod = resource_dir.join("models").join("htdemucs.onnx");
    if prod.exists() {
        return Some(prod);
    }
    let cwd = std::env::current_dir().ok()?;
    for c in [
        cwd.join("resources/models/htdemucs.onnx"),
        cwd.join("src-tauri/resources/models/htdemucs.onnx"),
    ] {
        if c.exists() {
            return Some(c);
        }
    }
    None
}

/// Loaded htdemucs ONNX session.
pub struct HtdemucsModel {
    session: Session,
    seg_len: usize,
}

impl HtdemucsModel {
    /// Load the model from `model_path`, building a CPU-only session.
    pub fn load(model_path: &Path) -> Result<Self, SeparatorError> {
        let session = Session::builder()
            .map_err(|e| SeparatorError::Inference(format!("session builder: {e}")))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| SeparatorError::Inference(format!("optimization level: {e}")))?
            // htdemucs on CPU is compute-bound; use all logical cores for intra-op.
            .with_intra_threads(num_intra_threads())
            .map_err(|e| SeparatorError::Inference(format!("intra threads: {e}")))?
            .commit_from_file(model_path)
            .map_err(|e| SeparatorError::Inference(format!("commit_from_file: {e}")))?;
        Ok(HtdemucsModel {
            session,
            seg_len: SEGMENT_LEN,
        })
    }

    /// Segment length (samples per channel) this model operates on.
    pub fn segment_len(&self) -> usize {
        self.seg_len
    }

    /// Run one `[1, 2, seg]` stereo chunk through the model.
    ///
    /// Returns `SOURCES` entries, each a `[left, right]` pair of `seg` samples.
    /// `left`/`right` must each be exactly `segment_len()` long.
    pub fn run_chunk(
        &mut self,
        left: &[f32],
        right: &[f32],
    ) -> Result<Vec<[Vec<f32>; 2]>, SeparatorError> {
        let seg = self.seg_len;
        if left.len() != seg || right.len() != seg {
            return Err(SeparatorError::Inference(format!(
                "chunk length mismatch: left={}, right={}, expected={}",
                left.len(),
                right.len(),
                seg
            )));
        }

        // Build the row-major [1, 2, seg] input: channel 0 = left, channel 1 = right.
        let mut data = Vec::with_capacity(2 * seg);
        data.extend_from_slice(left);
        data.extend_from_slice(right);
        let shape: Vec<i64> = vec![1, 2, seg as i64];
        let input = Tensor::from_array((shape, data))
            .map_err(|e| SeparatorError::Inference(format!("build input tensor: {e}")))?;

        // A single positional input is bound to the model's sole input ("input").
        let outputs = self
            .session
            .run(ort::inputs![input])
            .map_err(|e| SeparatorError::Inference(format!("session run: {e}")))?;

        let out_val = outputs
            .get(OUTPUT_NAME)
            .ok_or_else(|| SeparatorError::Inference(format!("missing output '{OUTPUT_NAME}'")))?;
        let (out_shape, flat) = out_val
            .try_extract_tensor::<f32>()
            .map_err(|e| SeparatorError::Inference(format!("extract output tensor: {e}")))?;

        // Expected shape: [1, SOURCES, 2, seg] (batch, sources, channels, samples).
        let dims: &[i64] = out_shape;
        if dims.len() != 4
            || dims[0] != 1
            || dims[1] != SOURCES as i64
            || dims[2] != 2
            || dims[3] != seg as i64
        {
            return Err(SeparatorError::Inference(format!(
                "unexpected output shape {dims:?}, expected [1, {SOURCES}, 2, {seg}]"
            )));
        }
        let expected = SOURCES * 2 * seg;
        if flat.len() != expected {
            return Err(SeparatorError::Inference(format!(
                "output element count {} != expected {}",
                flat.len(),
                expected
            )));
        }

        // Reshape flat row-major [1, 4, 2, seg] -> Vec<[left, right]>.
        // index = ((src * 2) + ch) * seg + i
        let mut result: Vec<[Vec<f32>; 2]> = Vec::with_capacity(SOURCES);
        for src in 0..SOURCES {
            let l_off = (src * 2) * seg;
            let r_off = ((src * 2) + 1) * seg;
            let l = flat[l_off..l_off + seg].to_vec();
            let r = flat[r_off..r_off + seg].to_vec();
            result.push([l, r]);
        }
        Ok(result)
    }
}

/// Pick a reasonable intra-op thread count (all logical cores, min 1).
fn num_intra_threads() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_and_runs_one_chunk_when_model_present() {
        let cwd = std::env::current_dir().unwrap();
        let Some(p) = resolve_model_path(&cwd) else {
            eprintln!("model absent; skip");
            return;
        };
        eprintln!("loading model from {}", p.display());
        let t_load = std::time::Instant::now();
        let mut m = HtdemucsModel::load(&p).unwrap();
        eprintln!("model loaded in {:?}", t_load.elapsed());

        let seg = m.segment_len();
        let silence = vec![0.0_f32; seg];
        let t_run = std::time::Instant::now();
        let out = m.run_chunk(&silence, &silence).unwrap();
        eprintln!("ran one chunk ({seg} samples) in {:?}", t_run.elapsed());

        assert_eq!(out.len(), SOURCES);
        assert_eq!(out[0][0].len(), seg);
        assert_eq!(out[0][1].len(), seg);
        assert_eq!(out[VOCALS_IDX][0].len(), seg);
        assert_eq!(out[VOCALS_IDX][1].len(), seg);
    }
}
