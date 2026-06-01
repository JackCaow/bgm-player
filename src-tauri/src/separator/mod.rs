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

/// Separate `input` into vocals + no_vocals (BGM) using the bundled htdemucs ONNX.
/// `on_progress(progress_0_to_100, status)` mirrors the existing extraction-progress events.
pub fn separate_htdemucs_2track(
    _app_resource_dir: &Path,
    _input: &Path,
    _output_dir: &Path,
    _ffmpeg_path: &str,
    _on_progress: &dyn Fn(f32, &str),
) -> Result<OutputPaths, SeparatorError> {
    Err(SeparatorError::Inference("not implemented".into()))
}
