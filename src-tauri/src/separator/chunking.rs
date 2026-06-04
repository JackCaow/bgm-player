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
        debug_assert!(stride > 0, "stride must be > 0 (else infinite loop)");
        debug_assert!(stride <= seg, "stride must be <= seg (else gaps/silent samples)");
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
        debug_assert_eq!(chunk.len(), self.seg, "chunk must be exactly seg samples");
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
        self.out
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
    fn triangular_weight_exact_values() {
        // Exact demucs window: cat([arange(1,L//2+1), arange(L-L//2,0,-1)]) / max.
        // This pins the actual shape so a corrupted/uniform window is caught.
        let w8 = triangular_weight(8);
        let expected8 = [0.25, 0.5, 0.75, 1.0, 1.0, 0.75, 0.5, 0.25];
        assert_eq!(w8.len(), expected8.len());
        for (i, (&got, &want)) in w8.iter().zip(expected8.iter()).enumerate() {
            assert!((got - want).abs() < 1e-6, "L=8 i={i}: {got} vs {want}");
        }

        let w5 = triangular_weight(5);
        let third = 1.0_f32 / 3.0;
        let expected5 = [third, 2.0 * third, 1.0, 2.0 * third, third];
        assert_eq!(w5.len(), expected5.len());
        for (i, (&got, &want)) in w5.iter().zip(expected5.iter()).enumerate() {
            assert!((got - want).abs() < 1e-6, "L=5 i={i}: {got} vs {want}");
        }
    }

    #[test]
    fn overlap_add_cross_fades_at_seam() {
        // Two overlapping segments with different per-chunk "model output":
        // segment A is all 1.0, segment B is all 3.0. With the triangular
        // window, the overlap region must cross-fade MONOTONICALLY from ~1.0
        // up to ~3.0. A UNIFORM window would instead produce a flat plateau at
        // (1+3)/2 = 2.0, which is NOT monotonic -> this test fails on uniform.
        let total = 1500usize;
        let seg = 1000usize;
        let stride = 500usize;
        let plan = SegmentPlan::new(total, seg, stride);
        // Expect exactly two segments: [0,1000) and [500,1500), overlap 500..1000.
        assert_eq!(plan.segments.len(), 2);
        assert_eq!(plan.segments[0].start, 0);
        assert_eq!(plan.segments[1].start, 500);

        let mut acc = OverlapAdder::new(total, seg);
        acc.add(plan.segments[0].start, &vec![1.0_f32; seg]);
        acc.add(plan.segments[1].start, &vec![3.0_f32; seg]);
        let out = acc.finish();

        // Endpoints outside the overlap reflect their single contributing chunk.
        assert!((out[0] - 1.0).abs() < 1e-4, "start should be ~1.0, got {}", out[0]);
        assert!((out[total - 1] - 3.0).abs() < 1e-4, "end should be ~3.0, got {}", out[total - 1]);

        // Strict interior of the overlap region (avoid off-by-one at the exact
        // boundary indices). Must be non-decreasing AND actually span 1->3.
        let lo = 520usize;
        let hi = 980usize;
        for i in lo..hi {
            assert!(
                out[i + 1] >= out[i] - 1e-5,
                "overlap must be monotonic non-decreasing at i={i}: {} -> {} \
                 (a uniform window would give a flat 2.0 plateau here)",
                out[i],
                out[i + 1]
            );
        }
        // And it genuinely transitions across the seam (not a flat plateau).
        assert!(out[lo] < 1.5, "low side of overlap should be near 1.0, got {}", out[lo]);
        assert!(out[hi] > 2.5, "high side of overlap should be near 3.0, got {}", out[hi]);
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
