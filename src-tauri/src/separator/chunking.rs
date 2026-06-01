//! Segment planning + weighted overlap-add, matching demucs `apply.py`.
//!
//! demucs splits the signal into overlapping segments, runs the model on each,
//! and recombines them with a triangular center-weighting window so that the
//! seams between adjacent segments are smoothly cross-faded. Without this the
//! reconstructed audio shows periodic artifacts at the segment boundaries.
//!
//! Constants per demucs: `overlap = 0.25` (applied by the caller via `stride`),
//! `transition_power = 1.0` (the triangular weight is used as-is, not raised to
//! a power). The segment length is supplied by the caller (e.g. 7.8s * sr).

/// Triangular center-weighting window (demucs `transition_power = 1.0`).
///
/// Matches demucs' `weight = cat([arange(1, L//2+1), arange(L - L//2, 0, -1)])`
/// normalized by its max. For `L = 8` this yields `[1,2,3,4,4,3,2,1] / 4`.
pub fn triangular_weight(seg: usize) -> Vec<f32> {
    let half = seg / 2;
    let mut w = Vec::with_capacity(seg);
    for i in 1..=half {
        w.push(i as f32);
    }
    for i in (1..=(seg - half)).rev() {
        w.push(i as f32);
    }
    let max = w.iter().cloned().fold(0.0_f32, f32::max);
    w.iter().map(|x| x / max).collect()
}

#[derive(Clone, Copy)]
pub struct Segment {
    pub start: usize,
    pub len: usize,
}

pub struct SegmentPlan {
    pub seg: usize,
    pub segments: Vec<Segment>,
}

impl SegmentPlan {
    pub fn new(total: usize, seg: usize, stride: usize) -> Self {
        let mut segments = Vec::new();
        let mut start = 0;
        while start < total {
            let len = seg.min(total - start);
            segments.push(Segment { start, len });
            if start + seg >= total {
                break;
            }
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
pub struct OverlapAdder {
    out: Vec<f32>,
    wsum: Vec<f32>,
    weight: Vec<f32>,
    seg: usize,
}

impl OverlapAdder {
    pub fn new(total: usize, seg: usize) -> Self {
        OverlapAdder {
            out: vec![0.0; total],
            wsum: vec![0.0; total],
            weight: triangular_weight(seg),
            seg,
        }
    }

    pub fn add(&mut self, start: usize, chunk: &[f32]) {
        let total = self.out.len();
        for j in 0..self.seg {
            let idx = start + j;
            if idx >= total {
                break;
            }
            self.out[idx] += chunk[j] * self.weight[j];
            self.wsum[idx] += self.weight[j];
        }
    }

    pub fn finish(mut self) -> Vec<f32> {
        for i in 0..self.out.len() {
            if self.wsum[i] > 1e-8 {
                self.out[i] /= self.wsum[i];
            }
        }
        std::mem::take(&mut self.out)
    }
}

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
