# ONNX + Rust htdemucs Separator — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the `python3` extraction call in `extract_bgm` with in-process Rust ONNX Runtime inference for the htdemucs default model in 2-track (vocals + BGM) mode, keeping the existing Python path as a fallback.

**Architecture:** A new `separator` Rust module decodes audio via ffmpeg → normalizes → splits into overlapping segments → runs a bundled self-contained htdemucs ONNX model (STFT baked into the graph) per segment via the `ort` crate → weighted overlap-add → denormalizes → writes `vocals.wav` + `no_vocals.wav`. `extract_bgm` forks to this path only for `htdemucs` + `2-track` when the model is present; any failure logs and falls through to the unchanged Python path. The ONNX path produces the same WAV files and re-enters the existing shared post-processing tail (format conversion, `ExtractResult` assembly), so the frontend is unchanged.

**Tech Stack:** Rust, Tauri 2, `ort` (ONNX Runtime), `hound` (WAV), `ndarray`; ffmpeg (existing dependency) for decode; Python + demucs PR #10 (dev/build-time only) for ONNX export.

**Spec:** `specs/2026-06-01-onnx-rust-separator-design.md`

**Reference implementations (consult, don't blind-copy):**
- demucs PR #10 export: https://github.com/adefossez/demucs/pull/10 (provides `stft.py`, `istft.py`, `scripts/convert-pth-to-onnx.py`)
- Rust ort + htdemucs inference: https://github.com/acolombier/stemgen (`src/demucs.rs` — windowing/overlap-add reference)
- demucs windowing source of truth: `demucs/apply.py` (`apply_model`, the `weight` tensor)
- `ort` crate docs/examples for the **pinned** version: https://github.com/pykeio/ort

---

## File Structure

| File | Responsibility | New/Modify |
|---|---|---|
| `script/export_onnx.py` | Dev/build-time: export htdemucs → `htdemucs.onnx`; print input/output shapes + source order | Create |
| `src-tauri/resources/models/htdemucs.onnx` | Bundled model weight (gitignored, generated) | Generated |
| `src-tauri/src/separator/mod.rs` | Orchestrator + public `separate_htdemucs_2track` | Create |
| `src-tauri/src/separator/decode.rs` | ffmpeg → interleaved f32 → `[2][N]` | Create |
| `src-tauri/src/separator/chunking.rs` | Segment split, weight window, overlap-add (pure) | Create |
| `src-tauri/src/separator/model.rs` | ONNX resource path resolution, `ort::Session`, run one chunk | Create |
| `src-tauri/src/separator/wav.rs` | Write 16-bit PCM WAV from f32 | Create |
| `src-tauri/Cargo.toml` | Add `ort`, `hound`, `ndarray` | Modify |
| `src-tauri/tauri.conf.json` | Add `bundle.resources` → model | Modify |
| `.gitignore` | Ignore `src-tauri/resources/models/*.onnx` | Modify |
| `src-tauri/src/lib.rs` | Fork in `extract_bgm` + fallback + escape hatch | Modify |
| `script/verify_parity.py` | Compare Rust-path vs Python-path stems (RMS/corr) | Create |

**Module boundary note:** `chunking.rs` and `wav.rs` are pure and fully unit-tested. `decode.rs` and `model.rs` are integration-tested (need ffmpeg / the model file present). `mod.rs` is validated by the parity script + manual run.

---

## Task 0: Export htdemucs to ONNX (de-risk first)

**Why first:** If PR #10 export fails, the whole approach changes — find out now (spec §9, §降级).

**Files:**
- Create: `script/export_onnx.py`
- Generated: `src-tauri/resources/models/htdemucs.onnx`
- Modify: `.gitignore`

- [ ] **Step 1: Add gitignore rule**

Append to `.gitignore`:
```
# Generated ONNX model weights (produced by script/export_onnx.py)
src-tauri/resources/models/*.onnx
```

- [ ] **Step 2: Write `script/export_onnx.py`**

It should: clone/checkout demucs PR #10 into a temp/build dir, install it into the existing `.venv`, run its `scripts/convert-pth-to-onnx.py` for `htdemucs`, and copy the result to `src-tauri/resources/models/htdemucs.onnx`. Print the ONNX input/output names and shapes.

```python
#!/usr/bin/env python3
"""Export htdemucs to a self-contained ONNX (STFT baked in) via demucs PR #10.
Run inside the project venv: .venv/bin/python script/export_onnx.py
"""
import subprocess, sys, shutil
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BUILD = ROOT / "build" / "demucs-onnx"
OUT = ROOT / "src-tauri" / "resources" / "models" / "htdemucs.onnx"
PR_REMOTE = "https://github.com/adefossez/demucs.git"
PR_REF = "pull/10/head"   # pin to a specific commit once verified

def run(cmd, **kw):
    print("+", " ".join(map(str, cmd)))
    subprocess.run(cmd, check=True, **kw)

def main():
    OUT.parent.mkdir(parents=True, exist_ok=True)
    if not BUILD.exists():
        run(["git", "clone", "--depth", "1", PR_REMOTE, str(BUILD)])
        run(["git", "-C", str(BUILD), "fetch", "origin", PR_REF])
        run(["git", "-C", str(BUILD), "checkout", "FETCH_HEAD"])
    run([sys.executable, "-m", "pip", "install", "-e", str(BUILD)])
    # PR #10 provides scripts/convert-pth-to-onnx.py; adapt args to its actual CLI.
    conv = BUILD / "scripts" / "convert-pth-to-onnx.py"
    run([sys.executable, str(conv), "--model", "htdemucs", "--out", str(OUT)])
    print(f"Exported -> {OUT} ({OUT.stat().st_size/1e6:.1f} MB)")

if __name__ == "__main__":
    main()
```

> The exact CLI of `convert-pth-to-onnx.py` must be confirmed against the PR; adjust `--model/--out` flags accordingly. If the PR script differs, follow its README.

- [ ] **Step 3: Run the export**

Run: `cd /Users/cver/workspace/bgm-player && .venv/bin/python script/export_onnx.py`
Expected: `Exported -> .../htdemucs.onnx (NN.N MB)` and exit 0.

- [ ] **Step 4: Inspect the model (one-time source-order assertion — spec §4.2.4)**

Run:
```bash
.venv/bin/python - <<'PY'
import onnxruntime as ort, numpy as np
s = ort.InferenceSession("src-tauri/resources/models/htdemucs.onnx",
                         providers=["CPUExecutionProvider"])
i = s.get_inputs()[0]; o = s.get_outputs()[0]
print("IN ", i.name, i.shape)
print("OUT", o.name, o.shape)
PY
```
Expected: input rank-3 `[1,2,T]`-like, output rank-4 `[1,4,2,T]`-like. Record the exact input/output **names** and the segment length `T` — they are needed in `model.rs`. (If `onnxruntime` isn't in the venv: `uv pip install --python .venv/bin/python onnxruntime`.)

- [ ] **Step 5: Commit (script only; model is gitignored)**

```bash
git add script/export_onnx.py .gitignore
git commit -m "feat(onnx): add htdemucs ONNX export script"
```

---

## Task 1: Rust deps + module skeleton

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/separator/mod.rs`
- Modify: `src-tauri/src/lib.rs` (add `mod separator;`)

- [ ] **Step 1: Add dependencies to `src-tauri/Cargo.toml`**

Under `[dependencies]` (pin `ort` to the version Stemgen uses / latest 2.x rc; confirm the exact version string against crates.io before building):
```toml
ort = { version = "=2.0.0-rc.10", default-features = false, features = ["ndarray", "download-binaries"] }
ndarray = "0.16"
hound = "3.5"
```
> `coreml` is intentionally NOT enabled yet (spec §3, CPU EP first). Add `"coreml"` to features in a later follow-up after benchmarking. Confirm the rc version and feature names against the pinned `ort` docs — they change between rc releases.

- [ ] **Step 2: Create `src-tauri/src/separator/mod.rs` skeleton**

```rust
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
```

- [ ] **Step 3: Register the module** — add `mod separator;` near the top of `src-tauri/src/lib.rs`. Create empty `decode.rs`, `chunking.rs`, `model.rs`, `wav.rs` so it compiles.

- [ ] **Step 4: Verify it builds**

Run: `cd src-tauri && source "$HOME/.cargo/env" && cargo build 2>&1 | tail -5`
Expected: compiles (ort downloads its prebuilt binary on first build). If ort fails to fetch, recheck version/features.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/separator src-tauri/src/lib.rs
git commit -m "feat(onnx): add separator module skeleton + ort/hound/ndarray deps"
```

---

## Task 2: WAV writer (TDD)

**Files:**
- Create: `src-tauri/src/separator/wav.rs`

- [ ] **Step 1: Write the failing test** (append to `wav.rs`)

```rust
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test --lib separator::wav 2>&1 | tail -15`
Expected: FAIL (`write_stereo_wav` not found).

- [ ] **Step 3: Implement `write_stereo_wav`** (top of `wav.rs`)

```rust
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
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test --lib separator::wav 2>&1 | tail -8`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/separator/wav.rs
git commit -m "feat(onnx): 16-bit WAV writer with test"
```

---

## Task 3: Chunking + weighted overlap-add (TDD) — core fidelity

**Files:**
- Create: `src-tauri/src/separator/chunking.rs`

This must match demucs `apply.py`. Constants: `segment_seconds ≈ 7.8`, `overlap = 0.25`, `transition_power = 1.0`. The weight window is triangular (ramps up to the center, down to the end). Confirm against `demucs/apply.py` and Stemgen `src/demucs.rs`.

- [ ] **Step 1: Write the failing tests** (append to `chunking.rs`)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn triangular_weight_is_symmetric_and_positive() {
        let w = triangular_weight(8);
        assert_eq!(w.len(), 8);
        assert!(w.iter().all(|&x| x > 0.0));
        assert!((w[0] - w[7]).abs() < 1e-6);
        assert!(w[3] > w[0]); // center heavier than edge
    }

    #[test]
    fn overlap_add_reconstructs_identity() {
        // With an identity "model" and weighted overlap-add, output ≈ input.
        let n = 5000usize;
        let input: Vec<f32> = (0..n).map(|i| (i as f32 * 0.01).sin()).collect();
        let seg = 1000usize;
        let stride = 750usize; // overlap 0.25
        let plan = SegmentPlan::new(n, seg, stride);
        let mut acc = OverlapAdder::new(n, seg);
        for s in &plan.segments {
            let chunk = plan.extract(&input, s); // len == seg (zero-padded at tail)
            acc.add(s.start, &chunk); // identity model: feed chunk straight back
        }
        let out = acc.finish();
        for i in 0..n {
            assert!((out[i] - input[i]).abs() < 1e-4, "i={i} {} vs {}", out[i], input[i]);
        }
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test --lib separator::chunking 2>&1 | tail -15`
Expected: FAIL (symbols not found).

- [ ] **Step 3: Implement `chunking.rs`**

```rust
/// Triangular center-weighting window (demucs transition_power=1.0).
pub fn triangular_weight(seg: usize) -> Vec<f32> {
    let half = seg / 2;
    let mut w = Vec::with_capacity(seg);
    for i in 1..=half { w.push(i as f32); }
    for i in (1..=(seg - half)).rev() { w.push(i as f32); }
    let max = w.iter().cloned().fold(0.0_f32, f32::max);
    w.iter().map(|x| x / max).collect()
}

#[derive(Clone, Copy)]
pub struct Segment { pub start: usize, pub len: usize }

pub struct SegmentPlan { pub seg: usize, pub segments: Vec<Segment> }

impl SegmentPlan {
    pub fn new(total: usize, seg: usize, stride: usize) -> Self {
        let mut segments = Vec::new();
        let mut start = 0;
        while start < total {
            let len = seg.min(total - start);
            segments.push(Segment { start, len });
            if start + seg >= total { break; }
            start += stride;
        }
        SegmentPlan { seg, segments }
    }
    /// Extract a zero-padded chunk of exactly `seg` samples starting at `s.start`.
    pub fn extract(&self, signal: &[f32], s: &Segment) -> Vec<f32> {
        let mut chunk = vec![0.0_f32; self.seg];
        chunk[..s.len].copy_from_slice(&signal[s.start..s.start + s.len]);
        chunk
    }
}

/// Accumulates weighted segment outputs and normalizes by summed weights.
pub struct OverlapAdder { out: Vec<f32>, wsum: Vec<f32>, weight: Vec<f32>, seg: usize }

impl OverlapAdder {
    pub fn new(total: usize, seg: usize) -> Self {
        OverlapAdder { out: vec![0.0; total], wsum: vec![0.0; total], weight: triangular_weight(seg), seg }
    }
    pub fn add(&mut self, start: usize, chunk: &[f32]) {
        let total = self.out.len();
        for j in 0..self.seg {
            let idx = start + j;
            if idx >= total { break; }
            self.out[idx] += chunk[j] * self.weight[j];
            self.wsum[idx] += self.weight[j];
        }
    }
    pub fn finish(mut self) -> Vec<f32> {
        for i in 0..self.out.len() {
            if self.wsum[i] > 1e-8 { self.out[i] /= self.wsum[i]; }
        }
        std::mem::take(&mut self.out)
    }
}
```

> The real pipeline runs the **model** on each chunk (4 sources × 2 channels) and overlap-adds each `(source, channel)` plane independently with this same window. The identity test above validates the windowing math; the orchestrator (Task 6) applies it per plane.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd src-tauri && cargo test --lib separator::chunking 2>&1 | tail -8`
Expected: PASS (both tests).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/separator/chunking.rs
git commit -m "feat(onnx): segment plan + weighted overlap-add with tests"
```

---

## Task 4: ffmpeg decode to f32 PCM

**Files:**
- Create: `src-tauri/src/separator/decode.rs`

- [ ] **Step 1: Implement `decode.rs`**

```rust
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
```

- [ ] **Step 2: Run test**

Run: `cd src-tauri && cargo test --lib separator::decode 2>&1 | tail -8`
Expected: PASS (or "skipping" line if ffmpeg missing — should be present here).

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/separator/decode.rs
git commit -m "feat(onnx): ffmpeg PCM decode with integration test"
```

---

## Task 5: ONNX model wrapper (ort)

**Files:**
- Create: `src-tauri/src/separator/model.rs`

> The `ort` 2.x API (Session::builder, inputs!/run, extracting tensors) is version-sensitive. Follow the pinned version's docs + Stemgen `src/demucs.rs`. Below is the intended shape — adapt API calls to the actual crate version.

- [ ] **Step 1: Implement `model.rs`**

```rust
use std::path::{Path, PathBuf};
use crate::separator::SeparatorError;

pub const SOURCES: usize = 4; // drums, bass, other, vocals (verify in Task 0 step 4)
pub const VOCALS_IDX: usize = 3;

/// Resolve the bundled model: prod = <resource_dir>/models/htdemucs.onnx;
/// dev = <cwd>/resources/models/htdemucs.onnx or <cwd>/../src-tauri/resources/models/...
pub fn resolve_model_path(resource_dir: &Path) -> Option<PathBuf> {
    let prod = resource_dir.join("models").join("htdemucs.onnx");
    if prod.exists() { return Some(prod); }
    let cwd = std::env::current_dir().ok()?;
    for c in [
        cwd.join("resources/models/htdemucs.onnx"),
        cwd.join("src-tauri/resources/models/htdemucs.onnx"),
    ] {
        if c.exists() { return Some(c); }
    }
    None
}

pub struct HtdemucsModel { session: ort::session::Session, seg_len: usize }

impl HtdemucsModel {
    pub fn load(model_path: &Path) -> Result<Self, SeparatorError> {
        let session = ort::session::Session::builder()
            .and_then(|b| b.commit_from_file(model_path))
            .map_err(|e| SeparatorError::Inference(format!("session init: {e}")))?;
        // seg_len: from model input shape (Task 0) or compute round(7.8*44100)=343980.
        Ok(Self { session, seg_len: 343980 })
    }

    pub fn segment_len(&self) -> usize { self.seg_len }

    /// Run one [1,2,seg] chunk → returns [SOURCES][2][seg].
    pub fn run_chunk(&mut self, left: &[f32], right: &[f32]) -> Result<Vec<[Vec<f32>;2]>, SeparatorError> {
        // Build ndarray [1,2,seg]; run; extract [1,4,2,seg]; reshape.
        // Use the input/output NAMES recorded in Task 0 step 4.
        // ... (adapt to pinned ort API: ort::inputs!, session.run, output.try_extract_tensor)
        unimplemented!("fill in with pinned ort API; see Stemgen src/demucs.rs")
    }
}
```

- [ ] **Step 2: Write an integration test** (append to `model.rs`) — gated on the model file existing so CI without the model still passes:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn loads_and_runs_one_chunk_when_model_present() {
        let cwd = std::env::current_dir().unwrap();
        let Some(p) = resolve_model_path(&cwd) else { eprintln!("model absent; skip"); return; };
        let mut m = HtdemucsModel::load(&p).unwrap();
        let seg = m.segment_len();
        let silence = vec![0.0_f32; seg];
        let out = m.run_chunk(&silence, &silence).unwrap();
        assert_eq!(out.len(), SOURCES);
        assert_eq!(out[0][0].len(), seg);
    }
}
```

- [ ] **Step 3: Run test**

Run: `cd src-tauri && cargo test --lib separator::model 2>&1 | tail -10`
Expected: PASS (runs the model if present; "skip" otherwise). Fix `run_chunk` until it passes with the model present.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/separator/model.rs
git commit -m "feat(onnx): ort htdemucs model wrapper"
```

---

## Task 6: Orchestrator (`separate_htdemucs_2track`)

**Files:**
- Modify: `src-tauri/src/separator/mod.rs`

- [ ] **Step 1: Implement the pipeline** (replace the stub body)

Pseudocode → Rust:
```rust
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

    let (mut left, mut right) = decode::decode_stereo_f32(ffmpeg_path, input)
        .map_err(SeparatorError::Decode)?;
    let n = left.len();

    // normalize: ref = mean over channels; (x - ref.mean()) / ref.std()
    let (mean, std) = compute_ref_mean_std(&left, &right);
    for x in left.iter_mut() { *x = (*x - mean) / std; }
    for x in right.iter_mut() { *x = (*x - mean) / std; }

    let seg = m.segment_len();
    let stride = ((seg as f32) * 0.75) as usize; // overlap 0.25
    let plan = chunking::SegmentPlan::new(n, seg, stride);

    // one OverlapAdder per (source, channel)
    let mut adders: Vec<[chunking::OverlapAdder;2]> = (0..model::SOURCES)
        .map(|_| [chunking::OverlapAdder::new(n, seg), chunking::OverlapAdder::new(n, seg)])
        .collect();

    let total = plan.segments.len().max(1);
    for (k, s) in plan.segments.iter().enumerate() {
        let lc = plan.extract(&left, s);
        let rc = plan.extract(&right, s);
        let stems = m.run_chunk(&lc, &rc)?; // [SOURCES][2][seg]
        for src in 0..model::SOURCES {
            adders[src][0].add(s.start, &stems[src][0]);
            adders[src][1].add(s.start, &stems[src][1]);
        }
        on_progress(5.0 + 90.0 * (k as f32 + 1.0) / total as f32, "正在分离音轨...");
    }

    // finish + denormalize
    let mut srcs: Vec<[Vec<f32>;2]> = adders.into_iter()
        .map(|[a,b]| [a.finish(), b.finish()]).collect();
    for s in srcs.iter_mut() { for ch in s.iter_mut() { for x in ch.iter_mut() { *x = *x * std + mean; } } }

    // 2-track: vocals = srcs[VOCALS_IDX]; no_vocals = sum(all) - vocals
    let (voc_l, voc_r) = (srcs[model::VOCALS_IDX][0].clone(), srcs[model::VOCALS_IDX][1].clone());
    let mut bgm_l = vec![0.0_f32; n]; let mut bgm_r = vec![0.0_f32; n];
    for src in 0..model::SOURCES {
        if src == model::VOCALS_IDX { continue; }
        for i in 0..n { bgm_l[i] += srcs[src][0][i]; bgm_r[i] += srcs[src][1][i]; }
    }

    on_progress(96.0, "正在保存文件...");
    let stem = input.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
    let dir = output_dir.join("htdemucs").join(stem);
    std::fs::create_dir_all(&dir).map_err(|e| SeparatorError::Io(e.to_string()))?;
    let vocals = dir.join("vocals.wav");
    let no_vocals = dir.join("no_vocals.wav");
    wav::write_stereo_wav(&vocals, &voc_l, &voc_r, decode::SAMPLE_RATE).map_err(SeparatorError::Io)?;
    wav::write_stereo_wav(&no_vocals, &bgm_l, &bgm_r, decode::SAMPLE_RATE).map_err(SeparatorError::Io)?;
    on_progress(100.0, "处理完成");
    Ok(OutputPaths { vocals, no_vocals })
}
```
Add `compute_ref_mean_std` matching the script: `ref = (left+right)/2` per-sample; `mean = ref.mean()`; `std = ref.std()` (population std; guard std==0 → 1.0).

> **Memory note:** this holds 4 sources × 2 channels × full-length `f32` buffers plus per-plane `OverlapAdder` accumulators at once — for a long song that's a large allocation (e.g. a 5-min stereo track ≈ 4×2×13.2M×4B ≈ 420MB for the sources alone, ~2× with accumulators). Acceptable for now; if it becomes a problem, stream/segment-write later (out of scope).

- [ ] **Step 2: Build**

Run: `cd src-tauri && cargo build 2>&1 | tail -8`
Expected: compiles.

- [ ] **Step 3: End-to-end smoke (model + ffmpeg present)** — add an ignored test or a tiny example bin that runs on `/tmp/bgm-test/sample.wav`; assert both WAVs exist and are non-empty.

Run: `cd src-tauri && cargo test --lib separator:: -- --nocapture 2>&1 | tail -15`
Expected: PASS; output WAVs created.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/separator/mod.rs
git commit -m "feat(onnx): htdemucs 2-track separation orchestrator"
```

---

## Task 7: Wire into `extract_bgm` + fallback + escape hatch

**Files:**
- Modify: `src-tauri/src/lib.rs` (the `extract_bgm` command, ~lines 104–298)

- [ ] **Step 1: Add the fork before the binary/python spawn**

Just before computing `binary_path` / spawning, insert:
```rust
let mode = separation_mode.as_deref().unwrap_or("2-track");
let use_onnx = model == "htdemucs"
    && mode == "2-track"
    && std::env::var("BGM_DISABLE_ONNX").is_err();
if use_onnx {
    let res_dir = app.path().resource_dir().unwrap_or_default();
    let w = window.clone();
    let progress = move |p: f32, s: &str| {
        // Reuse the existing "processing" stage (NOT a new "separating") so the
        // frontend, which may switch on `stage`, needs zero changes (reviewer note).
        let _ = w.emit("extraction-progress", ProgressPayload {
            progress: p, status: s.to_string(), stage: "processing".to_string(),
        });
    };
    match separator::separate_htdemucs_2track(&res_dir, Path::new(&input), &output_dir, &get_ffmpeg_path(&app), &progress) {
        Ok(paths) => {
            // Re-enter the shared post-processing tail (spec §4.1):
            // run the SAME format-conversion + ExtractResult assembly the python branch uses,
            // using paths.vocals / paths.no_vocals as the produced stems. Do NOT duplicate it —
            // refactor the tail into a helper if needed so both paths call it.
            return finalize_extraction(/* app, window, output_dir, model, mode, paths, ... */);
        }
        Err(e) => {
            eprintln!("[onnx] fallback to python: {e}");
            // fall through to existing path unchanged
        }
    }
}
```

- [ ] **Step 2: Extract the shared tail into `finalize_extraction`**

The current code after a successful extraction must be reachable by both paths. This tail is roughly **lines ~350–450**, and includes (a) the track-list building + per-track output-file discovery/existence check (`result_dir.join("{track}.wav")`, ~359–396), (b) format conversion (~398–422), and (c) `ExtractResult` assembly (~433–450). Refactor this whole span into a helper `finalize_extraction(...)` that takes the produced stem paths + model/mode and returns the same `Result` the command returns. The Python branch calls it too (behavior unchanged). Do NOT extract only the 398–450 part — that would skip the file-validation/track-discovery block the ONNX path also needs. Keep the refactor minimal and behavior-preserving.

- [ ] **Step 3: Build + run the app**

Run: `cd /Users/cver/workspace/bgm-player && bash script/dev.sh` (background), then extract a real song with htdemucs / 2-track via the UI.
Expected: app builds, extraction completes via the ONNX path (check logs — no `[onnx] fallback`), output + History correct. Test fallback: set `BGM_DISABLE_ONNX=1` and confirm it uses Python.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(onnx): route htdemucs 2-track through Rust ONNX with python fallback"
```

---

## Task 8: Parity verification

**Files:**
- Create: `script/verify_parity.py`

- [ ] **Step 1: Write `script/verify_parity.py`**

Runs the Python path (existing `bgm_extractor.py`) and reads the ONNX-path output (produced by the app or a small Rust example) for the same input; computes per-stem RMS difference and normalized cross-correlation between the two `no_vocals.wav` / `vocals.wav`. Prints a table and a PASS/FAIL against thresholds (e.g., corr > 0.99).

```python
#!/usr/bin/env python3
import sys
from pathlib import Path
import numpy as np
from scipy.io import wavfile

def load(p):
    sr, x = wavfile.read(p)
    x = x.astype(np.float64) / 32768.0
    return sr, x

def compare(a, b):
    n = min(len(a), len(b)); a, b = a[:n], b[:n]
    rms = np.sqrt(np.mean((a - b) ** 2))
    corr = np.corrcoef(a.reshape(-1), b.reshape(-1))[0, 1]
    return rms, corr

def main():
    py_dir, onnx_dir = Path(sys.argv[1]), Path(sys.argv[2])
    ok = True
    for stem in ["vocals.wav", "no_vocals.wav"]:
        _, a = load(py_dir / stem); _, b = load(onnx_dir / stem)
        rms, corr = compare(a, b)
        status = "PASS" if corr > 0.99 else "FAIL"
        if corr <= 0.99: ok = False
        print(f"{stem:14} rms={rms:.5f} corr={corr:.5f} {status}")
    sys.exit(0 if ok else 1)

if __name__ == "__main__":
    main()
```

- [ ] **Step 2: Run parity** on `/tmp/bgm-test/sample.wav` (+ one real song): run the Python path and the ONNX path, then compare.

Run: `.venv/bin/python script/verify_parity.py <python_out>/htdemucs/sample <onnx_out>/htdemucs/sample`
Expected: corr > 0.99 for both stems. If FAIL → revisit chunking window / source index / normalization (spec §9). Per spec §3, a non-parity result does not block merge (ONNX stays behind fallback) but should be investigated.

- [ ] **Step 3: Commit**

```bash
git add script/verify_parity.py
git commit -m "test(onnx): parity verification script vs python path"
```

---

## Task 9: Bundle config + docs

**Files:**
- Modify: `src-tauri/tauri.conf.json`, `README.md`, `README.zh-CN.md`

- [ ] **Step 1: Add model to bundle resources** in `tauri.conf.json`:
```json
"bundle": {
  "resources": ["resources/models/htdemucs.onnx"],
  ...
}
```

- [ ] **Step 2: Build the bundle and confirm the model is inside**

Run: `cd /Users/cver/workspace/bgm-player && bash script/dev.sh` first to confirm dev still works, then `npm run tauri:build 2>&1 | tail -20`.
Expected: build succeeds; the `.app` contains `Contents/Resources/models/htdemucs.onnx`. Launch the built `.app`, extract a song offline — verify it works without the venv on PATH (true standalone for htdemucs 2-track).

- [ ] **Step 3: Document** — add a "Build (standalone)" section to both READMEs: run `.venv/bin/python script/export_onnx.py` to generate the model before `npm run tauri:build`; note the ONNX path covers htdemucs 2-track and other models still need the Python env; note the `coreml` feature is a future toggle and `BGM_DISABLE_ONNX=1` forces Python.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/tauri.conf.json README.md README.zh-CN.md
git commit -m "feat(onnx): bundle htdemucs.onnx as resource + docs"
```

---

## Done criteria
- htdemucs 2-track extraction runs entirely in Rust (no python3 spawn) when the model is present; logs show no fallback.
- Other models/modes and any ONNX failure still work via the Python path.
- Parity script: corr > 0.99 vs Python on test inputs (or investigated per spec §3).
- `npm run tauri:build` produces a `.app` that does htdemucs 2-track offline with no external Python/torch.
- `cargo test --lib separator::` passes (chunking + wav always; decode/model when ffmpeg/model present).

## Follow-ups (out of scope, note in PR description)
- CoreML EP behind `coreml` feature + benchmark vs CPU (spec §3, §9).
- ONNX-ify htdemucs_ft / htdemucs_6s; evaluate MDX-Net for a no-export path.
- Pure-Rust decode (symphonia) to drop ffmpeg from the extraction path.
- Codesign + notarization for distribution.
