//! Multiple importance sampling: combines next-event estimation (light
//! sampling) with BSDF sampling so neither small bright emitters nor sharp
//! glossy lobes produce fireflies, without the clamps the classic tracer
//! needs. Weights follow Veach's power heuristic with beta = 2.

/// Power heuristic (beta = 2): weight of a sample drawn from the strategy
/// with density `a` when the other strategy has density `b`.
pub fn power_heuristic(a: f32, b: f32) -> f32 {
    if a <= 0.0 {
        return 0.0;
    }
    if !b.is_finite() {
        return 0.0;
    }
    let r = b / a;
    1.0 / (1.0 + r * r)
}

/// Balance heuristic (beta = 1), for comparison / tests.
pub fn balance_heuristic(a: f32, b: f32) -> f32 {
    if a <= 0.0 { 0.0 } else { a / (a + b.max(0.0)) }
}

/// Converts an area density at a light point into a solid-angle density
/// seen from the shading point: `p_A * d² / |cos_l|`.
pub fn area_to_solid_angle(pdf_area: f32, dist_sq: f32, cos_l: f32) -> f32 {
    if cos_l.abs() <= 1e-6 { f32::INFINITY } else { pdf_area * dist_sq / cos_l.abs() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn power_weights_sum_to_one_and_favour_the_denser_strategy() {
        for (a, b) in [(1.0, 1.0), (4.0, 1.0), (0.1, 30.0), (1e-3, 1e3)] {
            let (wa, wb) = (power_heuristic(a, b), power_heuristic(b, a));
            assert!((wa + wb - 1.0).abs() < 1e-5, "{a} {b}: {wa}+{wb}");
            assert_eq!(wa > wb, a > b);
        }
        assert_eq!(power_heuristic(1.0, 1.0), 0.5);
        assert!((power_heuristic(2.0, 1.0) - 0.8).abs() < 1e-6, "4/(4+1)");
        assert!(power_heuristic(3.0, 1.0) > balance_heuristic(3.0, 1.0), "power is sharper");
        assert_eq!(power_heuristic(0.0, 1.0), 0.0);
        assert_eq!(power_heuristic(1.0, 0.0), 1.0, "only strategy");
        assert_eq!(power_heuristic(1.0, f32::INFINITY), 0.0);
    }

    #[test]
    fn area_density_converts_with_inverse_square_and_cosine() {
        assert!((area_to_solid_angle(0.5, 4.0, 1.0) - 2.0).abs() < 1e-6);
        assert!((area_to_solid_angle(0.5, 4.0, -0.5) - 4.0).abs() < 1e-6);
        assert!(area_to_solid_angle(1.0, 1.0, 0.0).is_infinite());
    }
}
