//! MakeHuman-style macro targets. Each slider (Gender, Age, Muscle, Weight,
//! Height, Proportions, Breast Size / Firmness, ethnic mix) is turned into
//! weights over its key shapes, and every bundled macro target named
//! after a key-shape combination is blended by the product of the weights
//! (the same scheme as MakeHuman 1.x `MacroModifier`).

use super::target::Pack;
use bevy::math::Vec3;

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Macros {
    /// 0 = female, 1 = male.
    pub gender: f32,
    /// 0 = 1 year, 0.1875 = 11, 0.5 = 25, 1 = 90 years.
    pub age: f32,
    pub muscle: f32,
    pub weight: f32,
    pub height: f32,
    pub proportions: f32,
    pub breast_size: f32,
    pub breast_firmness: f32,
    pub african: f32,
    pub asian: f32,
    pub caucasian: f32,
}

impl Default for Macros {
    fn default() -> Self {
        Self {
            gender: 0.5,
            age: 0.5,
            muscle: 0.5,
            weight: 0.5,
            height: 0.5,
            proportions: 0.5,
            breast_size: 0.5,
            breast_firmness: 0.5,
            african: 1.0 / 3.0,
            asian: 1.0 / 3.0,
            caucasian: 1.0 / 3.0,
        }
    }
}

/// Three key shapes (min / average / max) around 0.5.
fn tri(v: f32) -> [f32; 3] {
    let v = v.clamp(0.0, 1.0);
    let lo = (1.0 - 2.0 * v).max(0.0);
    let hi = (2.0 * v - 1.0).max(0.0);
    [lo, 1.0 - lo - hi, hi]
}

/// Age key shapes: baby, child, young, old.
pub fn age_weights(a: f32) -> [f32; 4] {
    let a = a.clamp(0.0, 1.0);
    if a < 0.5 {
        let young = ((a - 0.1875) / 0.3125).max(0.0);
        let baby = (1.0 - a / 0.1875).max(0.0);
        [baby, 1.0 - baby - young, young, 0.0]
    } else {
        let old = 2.0 * a - 1.0;
        [0.0, 0.0, 1.0 - old, old]
    }
}

const GENDERS: [&str; 2] = ["female", "male"];
const AGES: [&str; 4] = ["baby", "child", "young", "old"];
const MUSCLE: [&str; 3] = ["minmuscle", "averagemuscle", "maxmuscle"];
const WEIGHT: [&str; 3] = ["minweight", "averageweight", "maxweight"];
const CUP: [&str; 3] = ["mincup", "averagecup", "maxcup"];
const FIRM: [&str; 3] = ["minfirmness", "averagefirmness", "maxfirmness"];

impl Macros {
    fn races(&self) -> [(&'static str, f32); 3] {
        let (a, b, c) = (self.african.max(0.0), self.asian.max(0.0), self.caucasian.max(0.0));
        let s = (a + b + c).max(1e-6);
        [("african", a / s), ("asian", b / s), ("caucasian", c / s)]
    }

    /// Every (target name, weight) pair these macros blend.
    pub fn weighted_targets(&self) -> Vec<(String, f32)> {
        let g = [1.0 - self.gender.clamp(0.0, 1.0), self.gender.clamp(0.0, 1.0)];
        let ag = age_weights(self.age);
        let (mu, we) = (tri(self.muscle), tri(self.weight));
        let mut out = Vec::new();
        for (gi, gn) in GENDERS.iter().enumerate() {
            for (ai, an) in AGES.iter().enumerate() {
                let w_ga = g[gi] * ag[ai];
                if w_ga <= 0.0 {
                    continue;
                }
                for (race, rw) in self.races() {
                    out.push((format!("macrodetails/{race}-{gn}-{an}"), w_ga * rw));
                }
                for (mi, mn) in MUSCLE.iter().enumerate() {
                    for (wi, wn) in WEIGHT.iter().enumerate() {
                        let w = w_ga * mu[mi] * we[wi];
                        if w > 0.0 {
                            out.push((format!("macrodetails/universal-{gn}-{an}-{mn}-{wn}"), w));
                        }
                    }
                }
            }
            // Height & proportions: only the young-adult key shapes are
            // bundled, so they apply at full strength at every age.
            let h = tri(self.height);
            let p = tri(self.proportions);
            for (mi, mn) in MUSCLE.iter().enumerate() {
                for (wi, wn) in WEIGHT.iter().enumerate() {
                    let w = g[gi] * mu[mi] * we[wi];
                    if w <= 0.0 {
                        continue;
                    }
                    let stem = format!("{gn}-young-{mn}-{wn}");
                    out.push((format!("macrodetails/height/{stem}-minheight"), w * h[0]));
                    out.push((format!("macrodetails/height/{stem}-maxheight"), w * h[2]));
                    out.push((format!("macrodetails/proportions/{stem}-uncommonproportions"), w * p[0]));
                    out.push((format!("macrodetails/proportions/{stem}-idealproportions"), w * p[2]));
                }
            }
        }
        // Breast (female share only).
        let (cup, firm) = (tri(self.breast_size), tri(self.breast_firmness));
        for (ai, an) in AGES.iter().enumerate() {
            for (mi, mn) in MUSCLE.iter().enumerate() {
                for (wi, wn) in WEIGHT.iter().enumerate() {
                    let base = g[0] * ag[ai] * mu[mi] * we[wi];
                    for (ci, cn) in CUP.iter().enumerate() {
                        for (fi, fname) in FIRM.iter().enumerate() {
                            let w = base * cup[ci] * firm[fi];
                            if w > 0.0 && !(ci == 1 && fi == 1) {
                                out.push((format!("breast/female-{an}-{mn}-{wn}-{cn}-{fname}"), w));
                            }
                        }
                    }
                }
            }
        }
        out.retain(|(_, w)| *w > 1e-5);
        out
    }

    /// Applies the macro blend; returns how many targets were found.
    pub fn apply(&self, pack: &Pack, coords: &mut [Vec3]) -> usize {
        self.weighted_targets().iter().filter(|(n, w)| pack.apply(n, coords, *w)).count()
    }
}
