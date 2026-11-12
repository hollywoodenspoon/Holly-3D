//! The 240 MakeHuman micro sliders, read from the bundled
//! `modeling_modifiers.json`: a two-sided slider `-1..1` blends its
//! `min` target (negative) or `max` target (positive); a one-sided
//! slider `0..1` blends its single target.

use super::assets;
use super::target::Pack;
use bevy::math::Vec3;
use std::collections::BTreeMap;
use std::sync::OnceLock;

#[derive(Clone, Debug, PartialEq)]
pub struct Slider {
    /// Group, e.g. `"nose"`.
    pub group: String,
    /// Key, e.g. `"nose/nose-trans-down|up"` (unique; stored in params).
    pub key: String,
    /// Display label, e.g. `"Nose Trans"`.
    pub label: String,
    pub neg: Option<String>,
    pub pos: String,
}

impl Slider {
    pub fn two_sided(&self) -> bool {
        self.neg.is_some()
    }

    pub fn apply(&self, pack: &Pack, coords: &mut [Vec3], v: f32) {
        if v > 0.0 {
            pack.apply(&self.pos, coords, v.min(1.0));
        } else if v < 0.0
            && let Some(n) = &self.neg
        {
            pack.apply(n, coords, (-v).min(1.0));
        }
    }
}

fn label(target: &str) -> String {
    target
        .split('-')
        .map(|w| {
            let mut c = w.chars();
            c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn parse(json: &str) -> Vec<Slider> {
    let Ok(serde_json::Value::Array(groups)) = serde_json::from_str::<serde_json::Value>(json) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for g in &groups {
        let group = g["group"].as_str().unwrap_or("");
        if group.starts_with("macrodetails") || group == "genitals" {
            continue;
        }
        for m in g["modifiers"].as_array().into_iter().flatten() {
            let Some(t) = m["target"].as_str() else { continue };
            let (min, max) = (m["min"].as_str(), m["max"].as_str());
            let (neg, pos, key) = match (min, max) {
                (Some(a), Some(b)) => (Some(format!("{group}/{t}-{a}")), format!("{group}/{t}-{b}"), format!("{group}/{t}-{a}|{b}")),
                _ => (None, format!("{group}/{t}"), format!("{group}/{t}")),
            };
            out.push(Slider { group: group.to_string(), key, label: label(t), neg, pos });
        }
    }
    out
}

/// All micro sliders (cached).
pub fn sliders() -> &'static [Slider] {
    static CELL: OnceLock<Vec<Slider>> = OnceLock::new();
    CELL.get_or_init(|| parse(assets::text(assets::modifiers_json())))
}

/// Applies every non-zero micro slider value in `values` (key -> value).
pub fn apply(values: &BTreeMap<String, f32>, pack: &Pack, coords: &mut [Vec3]) {
    for s in sliders() {
        if let Some(&v) = values.get(&s.key) {
            s.apply(pack, coords, v);
        }
    }
}
