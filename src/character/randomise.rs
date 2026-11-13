//! Randomise a human: body macros, face detail sliders, or both, from a
//! seed (the same seed always gives the same person).

use super::makehuman::modifiers::sliders;
use super::params::CharacterParams;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

const FACE: [&str; 10] = ["head", "forehead", "eyebrows", "eyes", "nose", "mouth", "ears", "chin", "cheek", "neck"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    /// Body macros and face sliders.
    Everything,
    /// Only the body macros (gender, age, weight...).
    Body,
    /// Only the face detail sliders.
    Face,
}

impl Scope {
    pub const ALL: [Scope; 3] = [Self::Everything, Self::Body, Self::Face];
    pub fn label(self) -> &'static str {
        match self {
            Self::Everything => "Everything",
            Self::Body => "Body only",
            Self::Face => "Face only",
        }
    }
}

/// A value near 0.5 (two dice averaged, so extremes are rarer).
fn mid(r: &mut ChaCha8Rng, lo: f32, hi: f32) -> f32 {
    lo + (hi - lo) * (r.r#gen::<f32>() + r.r#gen::<f32>()) * 0.5
}

/// Randomises `p` in `scope`. `keep_gender` leaves the gender slider.
/// `strength` (0..=1) scales how far face sliders move.
pub fn randomise(p: &mut CharacterParams, seed: u64, scope: Scope, keep_gender: bool, strength: f32) {
    let mut r = ChaCha8Rng::seed_from_u64(seed);
    if scope != Scope::Face {
        let m = &mut p.macros;
        if !keep_gender {
            m.gender = if r.gen_bool(0.5) { mid(&mut r, 0.0, 0.25) } else { mid(&mut r, 0.75, 1.0) };
        }
        m.age = mid(&mut r, 0.3, 0.75);
        m.muscle = mid(&mut r, 0.2, 0.8);
        m.weight = mid(&mut r, 0.2, 0.8);
        m.height = mid(&mut r, 0.2, 0.8);
        m.proportions = mid(&mut r, 0.3, 0.8);
        m.breast_size = mid(&mut r, 0.2, 0.8);
        m.breast_firmness = mid(&mut r, 0.3, 0.8);
        let mix: [f32; 3] = [r.r#gen::<f32>().powi(2), r.r#gen::<f32>().powi(2), r.r#gen::<f32>().powi(2)];
        let sum = mix.iter().sum::<f32>().max(1e-3);
        (m.african, m.asian, m.caucasian) = (mix[0] / sum, mix[1] / sum, mix[2] / sum);
    }
    if scope != Scope::Body {
        let k = strength.clamp(0.0, 1.0) * 0.45;
        for s in sliders().iter().filter(|s| FACE.contains(&s.group.as_str()) && !s.key.contains("/l-")) {
            let lo = if s.two_sided() { -1.0 } else { 0.0 };
            let v = if r.gen_bool(0.6) { mid(&mut r, lo, 1.0) * k } else { 0.0 };
            p.set_micro(&s.key, v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_person_and_scopes_stay_apart() {
        let base = CharacterParams::default();
        let (mut a, mut b) = (base.clone(), base.clone());
        randomise(&mut a, 7, Scope::Everything, false, 1.0);
        randomise(&mut b, 7, Scope::Everything, false, 1.0);
        assert_eq!(a, b);
        let mut c = base.clone();
        randomise(&mut c, 8, Scope::Everything, false, 1.0);
        assert_ne!(a, c);
        let mut body = base.clone();
        randomise(&mut body, 3, Scope::Body, true, 1.0);
        assert!(body.micro.is_empty() && body.macros.gender == base.macros.gender && body.macros != base.macros);
        let mut face = base.clone();
        randomise(&mut face, 3, Scope::Face, false, 1.0);
        assert_eq!(face.macros, base.macros);
        assert!(face.micro.values().any(|v| *v != 0.0) && face.micro.values().all(|v| v.abs() <= 0.45 + 1e-6));
        assert!(face.micro.keys().all(|k| FACE.iter().any(|g| k.starts_with(&format!("{g}/")))));
    }
}
