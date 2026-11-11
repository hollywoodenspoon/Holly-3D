//! Adaptive sampling: Welford running mean / variance of each pixel's
//! luminance; a pixel stops once it has the minimum samples and the worst
//! relative standard error in its 3x3 neighbourhood is under the Noise
//! Threshold (so a noisy neighbour keeps edges sampling).

/// Running statistics of one pixel (Welford's online algorithm).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Welford {
    pub n: u32,
    pub mean: f64,
    m2: f64,
}

impl Welford {
    pub const ZERO: Welford = Welford { n: 0, mean: 0.0, m2: 0.0 };

    pub fn push(&mut self, x: f32) {
        let x = f64::from(x);
        self.n += 1;
        let d = x - self.mean;
        self.mean += d / f64::from(self.n);
        self.m2 += d * (x - self.mean);
    }

    pub fn variance(&self) -> f64 {
        if self.n < 2 { 0.0 } else { self.m2 / f64::from(self.n - 1) }
    }

    /// Standard error of the mean relative to the (tone-ish) brightness:
    /// `sqrt(var / n) / (mean + 0.05)`, so dark pixels need not reach
    /// a tiny absolute error.
    pub fn rel_error(&self) -> f32 {
        if self.n < 2 {
            return f32::INFINITY;
        }
        ((self.variance() / f64::from(self.n)).sqrt() / (self.mean.abs() + 0.05)) as f32
    }
}

/// Which pixels of a `w` x `h` image are done (true = stop sampling).
pub fn converged(stats: &[Welford], w: usize, h: usize, threshold: f32, min_samples: u32) -> Vec<bool> {
    let mut out = vec![false; w * h];
    for y in 0..h {
        for x in 0..w {
            if stats[y * w + x].n < min_samples.max(2) {
                continue;
            }
            let mut worst = 0.0f32;
            for ny in y.saturating_sub(1)..(y + 2).min(h) {
                for nx in x.saturating_sub(1)..(x + 2).min(w) {
                    worst = worst.max(stats[ny * w + nx].rel_error());
                }
            }
            out[y * w + x] = worst < threshold;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn welford_matches_the_two_pass_formula() {
        let xs = [0.2f32, 0.9, 0.4, 0.4, 1.7, 0.05];
        let mut w = Welford::default();
        xs.iter().for_each(|&x| w.push(x));
        let mean = xs.iter().map(|&x| f64::from(x)).sum::<f64>() / 6.0;
        let var = xs.iter().map(|&x| (f64::from(x) - mean).powi(2)).sum::<f64>() / 5.0;
        assert!((w.mean - mean).abs() < 1e-9 && (w.variance() - var).abs() < 1e-9);
        assert_eq!(Welford::default().rel_error(), f32::INFINITY);
    }

    #[test]
    fn flat_pixels_converge_but_a_noisy_neighbour_holds_its_block() {
        let (w, h) = (5, 3);
        let mut stats = vec![Welford::default(); w * h];
        for (i, s) in stats.iter_mut().enumerate() {
            for k in 0..16 {
                // Pixel 4 (top-right corner) is noisy; the rest are constant.
                s.push(if i == 4 { (k % 2) as f32 * 2.0 } else { 0.5 });
            }
        }
        let done = converged(&stats, w, h, 0.02, 8);
        assert!(done[0] && done[1] && done[2] && done[10], "far from the noise");
        assert!(!done[4] && !done[3] && !done[8] && !done[9], "noisy pixel and its 3x3 block");
        let early = converged(&stats, w, h, 0.02, 32);
        assert!(early.iter().all(|d| !d), "min samples not reached");
    }
}
