//! ONNX-based Demucs inference module
//! Implements STFT/iSTFT and ONNX model inference for audio source separation

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use hound::{WavSpec, WavWriter};
use ndarray::{s, Array1, Array2, Array3, Array4, ArrayView2, Axis};
use num_complex::Complex;
use ort::session::{builder::GraphOptimizationLevel, Session};
use ort::value::Tensor;
use rustfft::{num_complex::Complex as FftComplex, FftPlanner};
use std::f32::consts::PI;
use std::fs::File;
use std::path::{Path, PathBuf};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// Execution provider for ONNX Runtime
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExecutionProvider {
    Cpu,
    CoreML,
    Cuda,
}

/// Progress callback type for reporting separation progress
/// Parameters: (progress_percent: f32, stage: &str, status: &str)
pub type ProgressCallback = Box<dyn Fn(f32, &str, &str) + Send + Sync>;

/// Demucs model configuration
#[derive(Debug, Clone)]
pub struct DemucsConfig {
    pub sample_rate: u32,
    pub channels: usize,
    pub sources: Vec<String>,
    pub nfft: usize,
    pub hop_length: usize,
    pub segment_samples: usize,
    /// Number of random shifts for equivariant stabilization (Demucs "shift trick").
    /// `0` disables the shift trick.
    pub shifts: usize,
    pub execution_provider: ExecutionProvider,
}

impl Default for DemucsConfig {
    fn default() -> Self {
        Self {
            sample_rate: 44100,
            channels: 2,
            sources: vec![
                "drums".to_string(),
                "bass".to_string(),
                "other".to_string(),
                "vocals".to_string(),
            ],
            nfft: 4096,
            hop_length: 1024,
            segment_samples: 343980, // ~7.8 seconds
            shifts: 1,
            execution_provider: ExecutionProvider::Cpu,
        }
    }
}

/// Hann window function (periodic, matching PyTorch)
fn hann_window(size: usize) -> Vec<f32> {
    (0..size)
        .map(|i| {
            let x = (PI * i as f32) / size as f32;
            x.sin().powi(2)
        })
        .collect()
}

/// STFT implementation compatible with PyTorch (normalized=True, center=True)
pub struct Stft {
    nfft: usize,
    hop_length: usize,
    window: Vec<f32>,
    fft_planner: FftPlanner<f32>,
}

impl Stft {
    pub fn new(nfft: usize, hop_length: usize) -> Self {
        Self {
            nfft,
            hop_length,
            window: hann_window(nfft),
            fft_planner: FftPlanner::new(),
        }
    }

    /// Reflect padding (matching PyTorch's reflect mode)
    fn reflect_pad(signal: &[f32], pad_left: usize, pad_right: usize) -> Vec<f32> {
        let len = signal.len();
        let total_len = pad_left + len + pad_right;
        let mut padded = vec![0.0f32; total_len];

        // Copy original signal
        padded[pad_left..pad_left + len].copy_from_slice(signal);

        // Left padding (reflect)
        for i in 0..pad_left {
            let idx = pad_left - 1 - i;
            let src_idx = (i + 1) % (2 * len - 2);
            let src_idx = if src_idx < len { src_idx } else { 2 * len - 2 - src_idx };
            padded[idx] = signal[src_idx.min(len - 1)];
        }

        // Right padding (reflect)
        for i in 0..pad_right {
            let idx = pad_left + len + i;
            let reflect_idx = i % (2 * len - 2);
            let src_idx = if reflect_idx < len - 1 {
                len - 2 - reflect_idx
            } else {
                reflect_idx - (len - 2)
            };
            padded[idx] = signal[src_idx.min(len - 1)];
        }

        padded
    }

    /// Compute STFT for a single channel (matching PyTorch torch.stft with normalized=True, center=True)
    pub fn forward(&mut self, signal: &[f32]) -> Array2<Complex<f32>> {
        let pad = self.nfft / 2;
        let padded = Self::reflect_pad(signal, pad, pad);
        let padded_len = padded.len();

        let num_frames = (padded_len - self.nfft) / self.hop_length + 1;
        let freq_bins = self.nfft / 2 + 1;

        let mut output = Array2::zeros((freq_bins, num_frames));
        let fft = self.fft_planner.plan_fft_forward(self.nfft);

        // PyTorch normalized=True uses 1/sqrt(nfft)
        let norm_factor = (self.nfft as f32).sqrt();

        for frame_idx in 0..num_frames {
            let start = frame_idx * self.hop_length;

            // Apply window and prepare FFT input
            let mut buffer: Vec<FftComplex<f32>> = (0..self.nfft)
                .map(|i| {
                    let sample = padded[start + i] * self.window[i];
                    FftComplex::new(sample, 0.0)
                })
                .collect();

            // Compute FFT
            fft.process(&mut buffer);

            // Store positive frequencies (normalized)
            for (freq_idx, &val) in buffer.iter().take(freq_bins).enumerate() {
                output[[freq_idx, frame_idx]] = Complex::new(
                    val.re / norm_factor,
                    val.im / norm_factor,
                );
            }
        }

        output
    }

    /// Compute inverse STFT (matching PyTorch torch.istft with normalized=True, center=True)
    pub fn inverse(&mut self, spectrogram: &Array2<Complex<f32>>, length: usize) -> Vec<f32> {
        let freq_bins = spectrogram.shape()[0];
        let num_frames = spectrogram.shape()[1];
        let nfft = (freq_bins - 1) * 2;

        let output_len = (num_frames - 1) * self.hop_length + nfft;
        let mut output = vec![0.0f32; output_len];
        let mut window_sum = vec![0.0f32; output_len];

        let ifft = self.fft_planner.plan_fft_inverse(nfft);
        let norm_factor = (nfft as f32).sqrt();

        for frame_idx in 0..num_frames {
            // Reconstruct full spectrum (with conjugate symmetry)
            let mut buffer: Vec<FftComplex<f32>> = vec![FftComplex::new(0.0, 0.0); nfft];

            for freq_idx in 0..freq_bins {
                let val = spectrogram[[freq_idx, frame_idx]];
                buffer[freq_idx] = FftComplex::new(val.re * norm_factor, val.im * norm_factor);
            }

            // Conjugate symmetry for negative frequencies
            for freq_idx in 1..freq_bins - 1 {
                let val = spectrogram[[freq_idx, frame_idx]];
                buffer[nfft - freq_idx] = FftComplex::new(val.re * norm_factor, -val.im * norm_factor);
            }

            // Compute IFFT
            ifft.process(&mut buffer);

            // Overlap-add with window
            let start = frame_idx * self.hop_length;
            for i in 0..nfft {
                if start + i < output_len {
                    output[start + i] += buffer[i].re * self.window[i] / nfft as f32;
                    window_sum[start + i] += self.window[i] * self.window[i];
                }
            }
        }

        // Normalize by window sum (COLA condition)
        for i in 0..output_len {
            if window_sum[i] > 1e-8 {
                output[i] /= window_sum[i];
            }
        }

        // Remove center padding and trim to length
        let pad = nfft / 2;
        let start = pad.min(output.len());
        let end = (start + length).min(output.len());
        output[start..end].to_vec()
    }
}

/// Convert complex-as-channels (CAC) back to complex (static version).
///
/// **Important:** This matches Demucs' `_magnitude` layout:
/// channel dimension is interleaved per audio channel: `[ch0_re, ch0_im, ch1_re, ch1_im, ...]`.
fn from_magnitude_static(cac: &Array4<f32>, num_channels: usize) -> Array4<Complex<f32>> {
    let sources = cac.shape()[0];
    let freq = cac.shape()[2];
    let time = cac.shape()[3];

    let mut output = Array4::zeros((sources, num_channels, freq, time));

    for s in 0..sources {
        for c in 0..num_channels {
            for f in 0..freq {
                for t in 0..time {
                    let re = cac[[s, 2 * c, f, t]];
                    let im = cac[[s, 2 * c + 1, f, t]];
                    output[[s, c, f, t]] = Complex::new(re, im);
                }
            }
        }
    }

    output
}

/// ONNX Demucs inference engine
pub struct OnnxDemucs {
    sessions: Vec<Session>,
    ensemble_weights: Option<Vec<Vec<f32>>>, // [model][source]
    ensemble_totals: Vec<f32>,               // [source]
    config: DemucsConfig,
    stft: Stft,
}

impl OnnxDemucs {
    /// Load ONNX model with specified execution provider
    pub fn new(
        model_paths: Vec<PathBuf>,
        config: DemucsConfig,
        ensemble_weights: Option<Vec<Vec<f32>>>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        if model_paths.is_empty() {
            return Err("No ONNX model paths provided".into());
        }

        let mut sessions = Vec::with_capacity(model_paths.len());
        for p in &model_paths {
            // NOTE: For now we always use the default execution provider (CPU).
            // The `ExecutionProvider` field is kept for future expansion.
            let session = Session::builder()?
                .with_optimization_level(GraphOptimizationLevel::Level3)?
                .commit_from_file(p)?;
            sessions.push(session);
        }

        let num_sources = config.sources.len();
        let ensemble_totals = if let Some(ref weights) = ensemble_weights {
            if weights.len() != model_paths.len() {
                return Err(format!(
                    "Ensemble weights length mismatch: weights={} models={}",
                    weights.len(),
                    model_paths.len()
                )
                .into());
            }
            let mut totals = vec![0.0f32; num_sources];
            for w in weights {
                if w.len() != num_sources {
                    return Err(format!(
                        "Ensemble weights source length mismatch: got={} expected={}",
                        w.len(),
                        num_sources
                    )
                    .into());
                }
                for (s, &v) in w.iter().enumerate() {
                    totals[s] += v;
                }
            }
            for t in &mut totals {
                if *t <= 0.0 {
                    *t = 1.0;
                }
            }
            totals
        } else {
            vec![model_paths.len() as f32; num_sources]
        };

        let stft = Stft::new(config.nfft, config.hop_length);

        Ok(Self {
            sessions,
            ensemble_weights,
            ensemble_totals,
            config,
            stft,
        })
    }

    /// Compute magnitude representation (complex as channels - CAC mode)
    fn to_magnitude(&self, z: &Array3<Complex<f32>>) -> Array3<f32> {
        let (channels, freq, time) = (z.shape()[0], z.shape()[1], z.shape()[2]);
        let mut mag = Array3::zeros((channels * 2, freq, time));

        for c in 0..channels {
            for f in 0..freq {
                for t in 0..time {
                    let val = z[[c, f, t]];
                    // Demucs CAC layout: [ch0_re, ch0_im, ch1_re, ch1_im, ...]
                    mag[[2 * c, f, t]] = val.re;
                    mag[[2 * c + 1, f, t]] = val.im;
                }
            }
        }

        mag
    }

    /// Compute STFT with Demucs-specific processing (_spec method)
    fn compute_spec(&mut self, audio: ArrayView2<f32>) -> (Array3<Complex<f32>>, usize) {
        let length = audio.shape()[1];
        let hl = self.config.hop_length;

        // Calculate expected number of frames
        let le = (length as f32 / hl as f32).ceil() as usize;

        // Padding like Demucs: pad = hl // 2 * 3
        let pad = hl / 2 * 3;

        // Compute STFT for each channel
        let mut spectrograms = Vec::new();
        for c in 0..self.config.channels {
            let channel_data: Vec<f32> = audio.row(c).to_vec();

            // Apply reflect padding
            let padded = Stft::reflect_pad(&channel_data, pad, pad + le * hl - length);

            let spec = self.stft.forward(&padded);
            spectrograms.push(spec);
        }

        // Stack into 3D array and apply Demucs-specific trimming
        let freq_bins = spectrograms[0].shape()[0] - 1; // Remove last freq bin
        let time_frames = spectrograms[0].shape()[1];

        // Trim time frames: [2: 2 + le]
        let start_frame = 2;
        let end_frame = (start_frame + le).min(time_frames);
        let actual_frames = end_frame - start_frame;

        let mut z = Array3::zeros((self.config.channels, freq_bins, actual_frames));
        for (c, spec) in spectrograms.iter().enumerate() {
            for f in 0..freq_bins {
                for t in 0..actual_frames {
                    z[[c, f, t]] = spec[[f, start_frame + t]];
                }
            }
        }

        (z, le)
    }

    /// Compute inverse STFT with Demucs-specific processing (_ispec method)
    fn compute_ispec(&mut self, z: &Array3<Complex<f32>>, length: usize) -> Array2<f32> {
        let channels = z.shape()[0];
        let freq = z.shape()[1];
        let time = z.shape()[2];
        let hl = self.config.hop_length;

        // Add back the last frequency bin (zeros)
        let mut z_padded = Array3::zeros((channels, freq + 1, time + 4)); // +4 for time padding

        // Copy data with time padding offset of 2
        for c in 0..channels {
            for f in 0..freq {
                for t in 0..time {
                    z_padded[[c, f, t + 2]] = z[[c, f, t]];
                }
            }
        }

        // Compute iSTFT for each channel
        let pad = hl / 2 * 3;
        let le = hl * (length as f32 / hl as f32).ceil() as usize + 2 * pad;

        let mut output = Array2::zeros((channels, length));

        for c in 0..channels {
            // Extract spectrogram for this channel
            let mut spec = Array2::zeros((freq + 1, time + 4));
            for f in 0..(freq + 1) {
                for t in 0..(time + 4) {
                    spec[[f, t]] = z_padded[[c, f, t]];
                }
            }

            let reconstructed = self.stft.inverse(&spec, le);

            // Remove padding and copy to output
            for i in 0..length.min(reconstructed.len().saturating_sub(pad)) {
                if pad + i < reconstructed.len() {
                    output[[c, i]] = reconstructed[pad + i];
                }
            }
        }

        output
    }

    /// Separate audio into sources (full implementation with spectral + time domain)
    ///
    /// # Arguments
    /// * `audio` - Input audio as 2D array (channels x samples)
    /// * `progress_callback` - Optional callback for progress reporting (progress%, stage, status)
    /// * `cancel_flag` - Optional atomic flag to check for cancellation
    pub fn separate(
        &mut self,
        audio: &Array2<f32>,
        progress_callback: Option<&ProgressCallback>,
        cancel_flag: Option<&Arc<AtomicBool>>,
    ) -> Result<Vec<Array2<f32>>, Box<dyn std::error::Error>> {
        let original_length = audio.shape()[1];
        let shifts = self.config.shifts;

        // Demucs "shift trick" (equivariant stabilization).
        // Default Demucs CLI uses shifts=1; higher values improve quality but cost time.
        if shifts > 0 {
            // Check for cancellation early
            if let Some(flag) = cancel_flag {
                if flag.load(Ordering::Relaxed) {
                    return Err("Cancelled by user".into());
                }
            }

            let max_shift = ((self.config.sample_rate as f32) * 0.5).round() as usize;
            let padded_len = original_length + 2 * max_shift;

            // Zero-pad left/right by max_shift
            let mut padded = Array2::zeros((self.config.channels, padded_len));
            for c in 0..self.config.channels {
                for i in 0..original_length {
                    padded[[c, max_shift + i]] = audio[[c, i]];
                }
            }

            let num_sources = self.config.sources.len();
            let mut accum: Vec<Array2<f32>> = (0..num_sources)
                .map(|_| Array2::zeros((self.config.channels, original_length)))
                .collect();

            // Deterministic pseudo-random offsets (avoid adding rand dependency).
            let mut state: u64 = (original_length as u64)
                ^ ((self.config.segment_samples as u64) << 1)
                ^ ((self.config.nfft as u64) << 17)
                ^ 0x9E37_79B9_7F4A_7C15u64;

            // If shifts == 1, forward progress from the inner separation.
            // (Otherwise the UI may appear stuck at the initial progress for a long time.)
            if shifts == 1 {
                state ^= state >> 12;
                state ^= state << 25;
                state ^= state >> 27;
                state = state.wrapping_mul(0x2545F4914F6CDD1Du64);

                let offset = if max_shift == 0 {
                    0
                } else {
                    (state as usize) % (max_shift + 1)
                };

                if let Some(cb) = progress_callback {
                    cb(10.0, "processing", "Equivariant stabilization (shift 1/1)...");
                }

                let shifted = padded.slice(s![.., offset..(original_length + max_shift)]);
                let shifted_out = self.separate_no_shifts(shifted, progress_callback, cancel_flag)?;

                let crop_start = max_shift.saturating_sub(offset);
                for (s_idx, src) in shifted_out.iter().enumerate() {
                    for c in 0..self.config.channels {
                        for i in 0..original_length {
                            accum[s_idx][[c, i]] = src[[c, crop_start + i]];
                        }
                    }
                }

                return Ok(accum);
            }

            for shift_idx in 0..shifts {
                // Check for cancellation between shifts
                if let Some(flag) = cancel_flag {
                    if flag.load(Ordering::Relaxed) {
                        return Err("Cancelled by user".into());
                    }
                }

                // xorshift64* (deterministic)
                state ^= state >> 12;
                state ^= state << 25;
                state ^= state >> 27;
                state = state.wrapping_mul(0x2545F4914F6CDD1Du64 ^ (shift_idx as u64));

                let offset = if max_shift == 0 {
                    0
                } else {
                    (state as usize) % (max_shift + 1)
                };

                if let Some(cb) = progress_callback {
                    let p = 5.0 + (shift_idx as f32 / shifts as f32) * 80.0;
                    cb(
                        p,
                        "processing",
                        &format!("Equivariant stabilization (shift {}/{})...", shift_idx + 1, shifts),
                    );
                }

                // In Demucs, shifted chunk length is `original + max_shift - offset`
                // and ends at `original + max_shift` (within padded tensor).
                let shifted = padded.slice(s![.., offset..(original_length + max_shift)]);
                let shifted_out = self.separate_no_shifts(shifted, None, cancel_flag)?;

                let crop_start = max_shift.saturating_sub(offset);

                for (s_idx, src) in shifted_out.iter().enumerate() {
                    for c in 0..self.config.channels {
                        for i in 0..original_length {
                            accum[s_idx][[c, i]] += src[[c, crop_start + i]];
                        }
                    }
                }
            }

            let inv = 1.0f32 / shifts as f32;
            for out in &mut accum {
                out.mapv_inplace(|v| v * inv);
            }

            return Ok(accum);
        }

        self.separate_no_shifts(audio.view(), progress_callback, cancel_flag)
    }

    fn separate_no_shifts(
        &mut self,
        audio: ArrayView2<f32>,
        progress_callback: Option<&ProgressCallback>,
        cancel_flag: Option<&Arc<AtomicBool>>,
    ) -> Result<Vec<Array2<f32>>, Box<dyn std::error::Error>> {
        let original_length = audio.shape()[1];

        // Handle long audio with segmentation
        if original_length > self.config.segment_samples {
            return self.separate_long_no_shifts(audio, progress_callback, cancel_flag);
        }

        // Report progress
        if let Some(cb) = progress_callback {
            cb(10.0, "processing", "Computing STFT...");
        }

        // Pad to segment length
        let mut padded = Array2::zeros((self.config.channels, self.config.segment_samples));
        let copy_len = original_length.min(self.config.segment_samples);
        for c in 0..self.config.channels {
            for i in 0..copy_len {
                padded[[c, i]] = audio[[c, i]];
            }
        }

        // Compute STFT with Demucs processing
        let (z, _le) = self.compute_spec(padded.view());

        // Convert to magnitude (real/imag as channels)
        let mag = self.to_magnitude(&z);

        // Report progress
        if let Some(cb) = progress_callback {
            cb(30.0, "processing", "Running ONNX inference...");
        }

        // Check for cancellation
        if let Some(flag) = cancel_flag {
            if flag.load(Ordering::Relaxed) {
                return Err("Cancelled by user".into());
            }
        }

        // Prepare ONNX inputs
        let mag_input: Array4<f32> = mag.insert_axis(Axis(0));
        let time_input: Array3<f32> = padded.clone().insert_axis(Axis(0));

        // Run inference
        let mag_shape: Vec<i64> = mag_input.shape().iter().map(|&x| x as i64).collect();
        let mag_data: Vec<f32> = mag_input.into_raw_vec_and_offset().0;

        let time_shape: Vec<i64> = time_input.shape().iter().map(|&x| x as i64).collect();
        let time_data: Vec<f32> = time_input.into_raw_vec_and_offset().0;

        // Run inference (single model or ensemble)
        let mut spec_accum: Option<ndarray::Array5<f32>> = None;
        let mut time_accum: Option<ndarray::Array4<f32>> = None;

        let total_models = self.sessions.len().max(1);
        for (model_idx, session) in self.sessions.iter_mut().enumerate() {
            if let Some(cb) = progress_callback {
                let p = 30.0 + (model_idx as f32 / total_models as f32) * 35.0;
                cb(
                    p,
                    "processing",
                    &format!("Running ONNX inference ({}/{})...", model_idx + 1, total_models),
                );
            }

            // NOTE: We intentionally create fresh ORT tensors per session run.
            // Some ORT value wrappers are not safe to clone/reuse across sessions.
            let mag_tensor = Tensor::from_array((mag_shape.clone(), mag_data.clone()))?;
            let time_tensor = Tensor::from_array((time_shape.clone(), time_data.clone()))?;

            let outputs = session.run(ort::inputs![
                "mag_input" => mag_tensor,
                "time_input" => time_tensor
            ])?;

            let spec_output = outputs[0].try_extract_array::<f32>()?;
            let time_output = outputs[1].try_extract_array::<f32>()?;

            let spec_out = spec_output.to_owned().into_dimensionality::<ndarray::Ix5>()?;
            let time_out = time_output.to_owned().into_dimensionality::<ndarray::Ix4>()?;

            drop(outputs);

            if spec_accum.is_none() {
                spec_accum = Some(ndarray::Array5::zeros(spec_out.raw_dim()));
            }
            if time_accum.is_none() {
                time_accum = Some(ndarray::Array4::zeros(time_out.raw_dim()));
            }

            let spec_accum_ref = spec_accum.as_mut().unwrap();
            let time_accum_ref = time_accum.as_mut().unwrap();

            let num_sources = self.config.sources.len();
            for s in 0..num_sources {
                let w = self
                    .ensemble_weights
                    .as_ref()
                    .and_then(|weights| weights.get(model_idx))
                    .and_then(|w_row| w_row.get(s))
                    .copied()
                    .unwrap_or(1.0);

                // spec: (1, S, C2, F, T)
                let out_view = spec_out.slice(s![0, s, .., .., ..]);
                let mut acc_view = spec_accum_ref.slice_mut(s![0, s, .., .., ..]);
                ndarray::Zip::from(&mut acc_view)
                    .and(&out_view)
                    .for_each(|a, &b| *a += b * w);

                // time: (1, S, C, samples)
                let out_view_t = time_out.slice(s![0, s, .., ..]);
                let mut acc_view_t = time_accum_ref.slice_mut(s![0, s, .., ..]);
                ndarray::Zip::from(&mut acc_view_t)
                    .and(&out_view_t)
                    .for_each(|a, &b| *a += b * w);
            }
        }

        let mut spec_out = spec_accum.ok_or("Missing spec output")?;
        let mut time_out = time_accum.ok_or("Missing time output")?;

        // Normalize by ensemble totals per source
        let num_sources = self.config.sources.len();
        for s in 0..num_sources {
            let denom = self.ensemble_totals.get(s).copied().unwrap_or(1.0).max(1e-8);
            spec_out
                .slice_mut(s![0, s, .., .., ..])
                .mapv_inplace(|v| v / denom);
            time_out
                .slice_mut(s![0, s, .., ..])
                .mapv_inplace(|v| v / denom);
        }

        // Report progress
        if let Some(cb) = progress_callback {
            cb(70.0, "processing", "Computing inverse STFT...");
        }

        // Store config values to avoid borrowing self
        let num_sources = self.config.sources.len();
        let num_channels = self.config.channels;
        let z_freq = z.shape()[1];
        let z_time = z.shape()[2];

        // Process each source
        let mut results = Vec::new();

        for s in 0..num_sources {
            // Extract spectral mask for this source
            let mut source_mask = Array4::zeros((1, num_channels * 2, z_freq, z_time));
            for c in 0..(num_channels * 2) {
                for f in 0..z_freq {
                    for t in 0..z_time {
                        source_mask[[0, c, f, t]] = spec_out[[0, s, c, f, t]];
                    }
                }
            }

            // Convert mask to complex
            let mask_complex = from_magnitude_static(&source_mask, num_channels);

            // Extract the source's masked spectrogram
            let mut source_spec = Array3::zeros((num_channels, z_freq, z_time));
            for c in 0..num_channels {
                for f in 0..z_freq {
                    for t in 0..z_time {
                        source_spec[[c, f, t]] = mask_complex[[0, c, f, t]];
                    }
                }
            }

            // Compute iSTFT to get spectral domain output
            let x_spec = self.compute_ispec(&source_spec, original_length);

            // Get time domain output
            let mut x_time = Array2::zeros((num_channels, original_length));
            for c in 0..num_channels {
                for i in 0..original_length {
                    x_time[[c, i]] = time_out[[0, s, c, i]];
                }
            }

            // Combine spectral and time domain outputs (like Demucs)
            let mut source_audio = Array2::zeros((num_channels, original_length));
            for c in 0..num_channels {
                for i in 0..original_length {
                    source_audio[[c, i]] = x_spec[[c, i]] + x_time[[c, i]];
                }
            }

            results.push(source_audio);
        }

        Ok(results)
    }

    /// Separate long audio using segmentation with overlap-add
    ///
    /// # Arguments
    /// * `audio` - Input audio as 2D array (channels x samples)
    /// * `progress_callback` - Optional callback for progress reporting (progress%, stage, status)
    /// * `cancel_flag` - Optional atomic flag to check for cancellation
    fn separate_long_no_shifts(
        &mut self,
        audio: ArrayView2<f32>,
        progress_callback: Option<&ProgressCallback>,
        cancel_flag: Option<&Arc<AtomicBool>>,
    ) -> Result<Vec<Array2<f32>>, Box<dyn std::error::Error>> {
        let original_length = audio.shape()[1];
        let segment_length = self.config.segment_samples;
        let stride = segment_length * 3 / 4; // 25% overlap

        let num_sources = self.config.sources.len();
        let mut outputs: Vec<Array2<f32>> = (0..num_sources)
            .map(|_| Array2::zeros((self.config.channels, original_length)))
            .collect();
        let mut weights: Array1<f32> = Array1::zeros(original_length);

        // Demucs-style triangular weight (normalized to max=1).
        // Matches demucs.apply.apply_model weight construction (transition_power=1).
        let mut window = vec![0.0f32; segment_length];
        let half = segment_length / 2;
        for i in 0..half {
            window[i] = (i + 1) as f32;
        }
        for i in half..segment_length {
            window[i] = (segment_length - i) as f32;
        }
        let max_w = window
            .iter()
            .cloned()
            .fold(0.0f32, |a, b| if a > b { a } else { b })
            .max(1.0);
        for w in &mut window {
            *w /= max_w;
        }

        // Calculate total segments for progress reporting
        let total_segments = ((original_length as f32 - segment_length as f32) / stride as f32).ceil() as usize + 1;
        let mut segment_idx = 0;

        let mut pos = 0;
        while pos < original_length {
            // Check for cancellation at the start of each segment
            if let Some(flag) = cancel_flag {
                if flag.load(Ordering::Relaxed) {
                    return Err("Cancelled by user".into());
                }
            }

            let end = (pos + segment_length).min(original_length);
            let seg_len = end - pos;

            // Report segment-level progress
            if let Some(cb) = progress_callback {
                let progress = 10.0 + (segment_idx as f32 / total_segments as f32) * 80.0;
                cb(
                    progress,
                    "processing",
                    &format!("Processing segment {}/{}", segment_idx + 1, total_segments),
                );
            }

            // Extract segment
            let mut segment = Array2::zeros((self.config.channels, segment_length));
            for c in 0..self.config.channels {
                for i in 0..seg_len {
                    segment[[c, i]] = audio[[c, pos + i]];
                }
            }

            // Process segment (pass None for nested progress to avoid double reporting)
            let seg_results = self.separate_no_shifts(segment.view(), None, cancel_flag)?;

            // Overlap-add with window
            for (s, seg_result) in seg_results.iter().enumerate() {
                for c in 0..self.config.channels {
                    for i in 0..seg_len {
                        outputs[s][[c, pos + i]] += seg_result[[c, i]] * window[i];
                    }
                }
            }

            for i in 0..seg_len {
                weights[pos + i] += window[i];
            }

            pos += stride;
            segment_idx += 1;
        }

        // Report final progress
        if let Some(cb) = progress_callback {
            cb(90.0, "processing", "Normalizing output...");
        }

        // Normalize by weights
        for output in &mut outputs {
            for c in 0..self.config.channels {
                for i in 0..original_length {
                    if weights[i] > 1e-8 {
                        output[[c, i]] /= weights[i];
                    }
                }
            }
        }

        Ok(outputs)
    }

    /// Get source names
    pub fn source_names(&self) -> &[String] {
        &self.config.sources
    }
}

// ============================================================================
// Audio I/O Functions
// ============================================================================

/// Read audio file and return stereo f32 samples at target sample rate
pub fn read_audio(path: &Path, target_sr: u32) -> Result<Array2<f32>, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe().format(
        &hint,
        mss,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    )?;

    let mut format = probed.format;
    let track = format
        .default_track()
        .ok_or("No default track found")?;

    let track_id = track.id;
    let codec_params = track.codec_params.clone();
    let source_sr = codec_params.sample_rate.unwrap_or(44100);
    let channels = codec_params.channels.map(|c| c.count()).unwrap_or(2);

    let mut decoder = symphonia::default::get_codecs()
        .make(&codec_params, &DecoderOptions::default())?;

    let mut samples: Vec<Vec<f32>> = vec![Vec::new(); channels];

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(_)) => break,
            Err(e) => return Err(e.into()),
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = decoder.decode(&packet)?;
        let spec = *decoded.spec();
        let duration = decoded.capacity() as u64;

        let mut sample_buf = SampleBuffer::<f32>::new(duration, spec);
        sample_buf.copy_interleaved_ref(decoded);

        let interleaved = sample_buf.samples();
        for (i, sample) in interleaved.iter().enumerate() {
            let ch = i % channels;
            samples[ch].push(*sample);
        }
    }

    // Convert to stereo if needed
    let stereo = if channels == 1 {
        vec![samples[0].clone(), samples[0].clone()]
    } else if channels >= 2 {
        vec![samples[0].clone(), samples[1].clone()]
    } else {
        return Err("No audio channels found".into());
    };

    // Resample if needed
    let resampled = if source_sr != target_sr {
        resample_linear(&stereo, source_sr, target_sr)
    } else {
        stereo
    };

    // Convert to ndarray
    let len = resampled[0].len();
    let mut audio = Array2::zeros((2, len));
    for (c, channel) in resampled.iter().enumerate() {
        for (i, &sample) in channel.iter().enumerate() {
            audio[[c, i]] = sample;
        }
    }

    Ok(audio)
}

/// Simple linear resampling
fn resample_linear(audio: &[Vec<f32>], source_sr: u32, target_sr: u32) -> Vec<Vec<f32>> {
    let ratio = target_sr as f64 / source_sr as f64;
    let new_len = (audio[0].len() as f64 * ratio) as usize;

    audio
        .iter()
        .map(|channel| {
            (0..new_len)
                .map(|i| {
                    let src_pos = i as f64 / ratio;
                    let src_idx = src_pos as usize;
                    let frac = src_pos - src_idx as f64;

                    if src_idx + 1 < channel.len() {
                        let a = channel[src_idx];
                        let b = channel[src_idx + 1];
                        a + (b - a) * frac as f32
                    } else if src_idx < channel.len() {
                        channel[src_idx]
                    } else {
                        0.0
                    }
                })
                .collect()
        })
        .collect()
}

/// Write stereo audio to WAV file
pub fn write_wav(
    path: &Path,
    audio: &Array2<f32>,
    sample_rate: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let spec = WavSpec {
        channels: 2,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = WavWriter::create(path, spec)?;
    let num_samples = audio.shape()[1];

    for i in 0..num_samples {
        for c in 0..2 {
            let sample = audio[[c, i]];
            // Clamp and convert to i16
            let clamped = sample.clamp(-1.0, 1.0);
            let int_sample = (clamped * 32767.0) as i16;
            writer.write_sample(int_sample)?;
        }
    }

    writer.finalize()?;
    Ok(())
}

/// High-level function to separate audio file into sources
pub fn separate_file(
    input_path: &Path,
    output_dir: &Path,
    model_path: &Path,
    config: Option<DemucsConfig>,
    progress_callback: Option<&ProgressCallback>,
    cancel_flag: Option<&Arc<AtomicBool>>,
) -> Result<Vec<std::path::PathBuf>, Box<dyn std::error::Error>> {
    let config = config.unwrap_or_default();
    let sample_rate = config.sample_rate;

    // Read audio
    let audio = read_audio(input_path, sample_rate)?;

    // Load model and separate
    let mut demucs = OnnxDemucs::new(vec![model_path.to_path_buf()], config, None)?;
    let sources = demucs.separate(&audio, progress_callback, cancel_flag)?;
    let source_names = demucs.source_names().to_vec();

    // Create output directory
    std::fs::create_dir_all(output_dir)?;

    // Get input file stem for naming
    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");

    // Write each source to file
    let mut output_paths = Vec::new();
    for (source, name) in sources.iter().zip(source_names.iter()) {
        let output_path = output_dir.join(format!("{}_{}.wav", stem, name));
        write_wav(&output_path, source, sample_rate)?;
        output_paths.push(output_path);
    }

    Ok(output_paths)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hann_window() {
        let window = hann_window(4);
        assert_eq!(window.len(), 4);
        // Periodic Hann window: sin^2(pi * i / n)
        assert!((window[0] - 0.0).abs() < 1e-6);
        assert!((window[2] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_stft_roundtrip() {
        let mut stft = Stft::new(1024, 256);
        let signal: Vec<f32> = (0..4096).map(|i| (i as f32 * 0.01).sin()).collect();

        let spec = stft.forward(&signal);
        let reconstructed = stft.inverse(&spec, signal.len());

        // Check reconstruction error
        let error: f32 = signal.iter()
            .zip(reconstructed.iter())
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f32>()
            .sqrt() / signal.len() as f32;

        assert!(error < 0.1, "STFT roundtrip error too high: {}", error);
    }
}
