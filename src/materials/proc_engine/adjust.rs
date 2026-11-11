//! Per-pixel adjustments: Levels and Height Blend.
//!
//! Ported from MaterialCraft (https://github.com/storytold/materialcraft:
//! `levels_fn` in `crates/nodes/src/atomic/levels.rs` and `height_blend`
//! in `filters.rs` before the old procedural library was removed),
//! copyright (c) 2026 ArtCraft Team and the MaterialCraft contributors,
//! MIT OR Apache-2.0.

/// Levels: remaps the input range, midtone and output range of one value.
pub fn levels(v: f32, in_low: f32, in_mid: f32, in_high: f32, out_low: f32, out_high: f32) -> f32 {
    let span = in_high - in_low;
    let t = if span.abs() < 1e-6 { if v >= in_high { 1.0 } else { 0.0 } } else { ((v - in_low) / span).clamp(0.0, 1.0) };
    // NaN (from an input or a bad parameter) would otherwise spread through the graph.
    let t = if t.is_finite() { t } else { 0.0 };
    let g = 0.5f32.ln() / in_mid.clamp(0.001, 0.999).ln();
    out_low + t.powf(g) * (out_high - out_low)
}

/// Height blend: how much of the top layer shows (0..1) where its height
/// `top` meets the bottom layer's height `bottom`, so the top layer fills
/// low crevices first. `contrast` 1 is a hard edge, 0 a soft one.
pub fn height_blend(top: f32, bottom: f32, contrast: f32) -> f32 {
    let soft = (1.0 - contrast.clamp(0.0, 0.999)) * 0.5 + 1e-4;
    ((top - bottom) / soft * 0.5 + 0.5).clamp(0.0, 1.0)
}

/// 32-bin histogram of values in 0..1, scaled so the tallest bin is 1.
pub fn histogram(values: impl Iterator<Item = f32>) -> [f32; 32] {
    let mut bins = [0.0f32; 32];
    for v in values {
        bins[((v.clamp(0.0, 1.0) * 32.0) as usize).min(31)] += 1.0;
    }
    let peak = bins.iter().copied().fold(1.0, f32::max);
    bins.map(|b| b / peak)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_remaps_the_input_range_midtones_and_output_range() {
        let close = |a: f32, b: f32| (a - b).abs() < 1e-5;
        for v in [0.0, 0.25, 0.5, 1.0] {
            assert!(close(levels(v, 0.0, 0.5, 1.0, 0.0, 1.0), v));
        }
        assert!(close(levels(0.4, 0.2, 0.5, 0.6, 0.0, 1.0), 0.5));
        assert!(close(levels(0.1, 0.2, 0.5, 0.6, 0.0, 1.0), 0.0));
        assert!(close(levels(0.25, 0.0, 0.25, 1.0, 0.0, 1.0), 0.5));
        assert!(close(levels(0.5, 0.0, 0.5, 1.0, 0.2, 0.6), 0.4));
        assert!(close(levels(0.0, 0.0, 0.5, 1.0, 1.0, 0.0), 1.0));
        assert_eq!(levels(0.49, 0.5, 0.5, 0.5, 0.0, 1.0), 0.0);
        assert!(levels(f32::NAN, 0.0, 0.5, 1.0, 0.0, 1.0).is_finite());
    }

    #[test]
    fn height_blend_fills_low_areas_first_and_histograms_count() {
        assert_eq!(height_blend(0.8, 0.2, 0.9), 1.0);
        assert_eq!(height_blend(0.2, 0.8, 0.9), 0.0);
        assert!((height_blend(0.5, 0.5, 0.5) - 0.5).abs() < 1e-5);
        let (soft, hard) = (height_blend(0.55, 0.5, 0.0), height_blend(0.55, 0.5, 0.95));
        assert!(soft > 0.5 && soft < hard);
        // A layer at level 0.3 covers a crevice (0.1) but not a peak (0.9).
        assert!(height_blend(0.3, 0.1, 0.8) > 0.9 && height_blend(0.3, 0.9, 0.8) < 0.1);
        let h = histogram([0.0, 0.01, 0.02, 0.99].into_iter());
        assert_eq!((h[0], h[31]), (1.0, 1.0 / 3.0));
    }
}
