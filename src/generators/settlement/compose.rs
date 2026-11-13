//! Shared helpers of the PR 6 presets: placing Buildings & Props pieces on
//! lots (respecting the per-type checkboxes, part toggles and Emissive
//! Lights), rivers / sea with stone embankments, bridges, street
//! frontage, the Prop Style street dressing pass and the lights-off pass.

use super::PropStyle;
use super::layout::{Lot, Plan};
use crate::generators::buildings::{BuildingKind, BuildingParams, Category, infra::DECK};
use crate::generators::kit::ids;
use crate::generators::ruin::{DestructionLevel, RuinParams};
use crate::mesh::EditablePoly;
use bevy::math::{Quat, Vec2, Vec3};
use rand::Rng;
use std::f32::consts::FRAC_PI_2;

/// Water surface height in rivers, harbours and moats.
pub const WATER_Z: f32 = -2.4;

impl Plan {
    /// Is a palette type switched on (its checkbox)?
    pub fn allowed(&self, k: BuildingKind) -> bool {
        !self.p.skip.contains(&k)
    }

    /// A random switched-on type among `kinds`.
    pub fn pick(&mut self, kinds: &[BuildingKind]) -> Option<BuildingKind> {
        let on: Vec<BuildingKind> = kinds.iter().copied().filter(|k| self.allowed(*k)).collect();
        (!on.is_empty()).then(|| on[self.rng.gen_range(0..on.len())])
    }

    /// Floors between the Min / Max Floors spinners.
    pub fn floors(&mut self) -> u32 {
        let lo = self.p.min_floors.max(1);
        self.rng.gen_range(lo..=self.p.max_floors.max(lo))
    }

    /// The destruction of one building: the town's level, one step either
    /// way (seeded by `key`, so the layout RNG is untouched).
    pub fn ruin_for(&self, key: u32) -> RuinParams {
        let base = &self.p.ruin;
        if base.level == DestructionLevel::Intact {
            return base.clone();
        }
        let mut r = crate::generators::kit::rng(base.seed ^ key, 0xDE57);
        let d = match r.gen_range(0..4) {
            0 => -1,
            3 => 1,
            _ => 0,
        };
        RuinParams { level: base.level.step(d).max(DestructionLevel::Light), seed: base.seed.wrapping_add(key), ..base.clone() }
    }

    /// Draws a piece with `f` in its own frame, wrecks it per the town's
    /// destruction level, then places it on `lot` at height `z`.
    pub fn wrecked(
        &mut self,
        lot: Lot,
        z: f32,
        medieval: bool,
        f: impl FnOnce(&mut crate::generators::kit::Kit, &mut rand_chacha::ChaCha8Rng),
    ) {
        if self.p.ruin.level == DestructionLevel::Intact {
            let (kit, r) = (&mut self.kit, &mut self.rng);
            kit.at(lot.frame(z), |k| f(k, r));
            return;
        }
        let mut k = crate::generators::kit::Kit::new();
        f(&mut k, &mut self.rng);
        let mut m = k.finish();
        let key = (lot.c.x.to_bits() ^ lot.c.y.to_bits().rotate_left(13)) % 100_000;
        crate::generators::ruin::apply(&mut m, &self.ruin_for(key), medieval);
        m.transform(lot.frame(z));
        self.kit.append(&m);
    }

    /// Claims `lot` and fills it with a Buildings & Props piece (front to
    /// the lot's -Y). False when the type is off or the lot is taken.
    pub fn building(&mut self, lot: Lot, kind: BuildingKind, floors: Option<u32>) -> bool {
        if !self.allowed(kind) || !self.claim(lot) {
            return false;
        }
        self.piece(lot, kind, floors, None);
        true
    }

    /// Builds a piece on `lot` without claiming it (`z` = ground if None).
    pub fn piece(&mut self, lot: Lot, kind: BuildingKind, floors: Option<u32>, z: Option<f32>) {
        let bp = BuildingParams {
            width: lot.half.x * 2.0,
            length: lot.half.y * 2.0,
            floors: floors.unwrap_or(kind.defaults().2),
            seed: self.rng.gen_range(0..100_000),
            parts: self.p.parts,
            emissive: self.p.emissive,
            ..BuildingParams::of(kind)
        };
        // Bridges, quays and street props are left standing.
        let house = matches!(kind.category(), Category::Modern | Category::Medieval | Category::Residential | Category::Commercial | Category::Industrial | Category::Services);
        let bp = if house { BuildingParams { ruin: self.ruin_for(bp.seed), ..bp } } else { bp };
        let z = z.unwrap_or_else(|| lot.corners().iter().chain([&lot.c]).map(|q| self.h(*q)).fold(f32::MAX, f32::min));
        let mut m = bp.build();
        m.transform(lot.frame(z));
        self.kit.append(&m);
    }

    /// A river (or sea / moat band) from `a` to `b`, `w` wide: water
    /// surface, stone embankments and no ground. Call before `ground`.
    pub fn river(&mut self, a: Vec2, b: Vec2, w: f32, embank: bool) -> Lot {
        let d = b - a;
        let lot = Lot::new((a + b) * 0.5, Vec2::new(d.length() * 0.5, w * 0.5), d.y.atan2(d.x));
        self.wet.push(lot);
        let h = lot.half;
        self.kit.at(lot.frame(WATER_Z), |k| {
            k.mat(ids::WATER).block(Vec3::new(-h.x, -h.y, -0.1), Vec3::new(h.x, h.y, 0.0));
            if embank {
                k.mat(ids::FACADE);
                for s in [-1.0, 1.0] {
                    let y = s * h.y;
                    k.block(Vec3::new(-h.x, y.min(y + s * 1.2), -1.0), Vec3::new(h.x, y.max(y + s * 1.2), -WATER_Z + 0.15));
                }
            }
        });
        lot
    }

    /// A bridge `width` wide across `river` at `at` (deck flush with the
    /// streets); `kind` is a bridge type from Buildings & Props.
    pub fn bridge(&mut self, river: Lot, at: Vec2, width: f32, kind: BuildingKind) {
        if !self.allowed(kind) || !self.p.features.bridges {
            return;
        }
        let lot = Lot::new(at, Vec2::new(width * 0.5, river.half.y + 3.0), river.yaw);
        let floors = ((river.half.y * 2.0 / 14.0).ceil() as u32).max(1);
        self.piece(lot, kind, Some(floors), Some(-DECK + 0.12));
        self.used.push(lot);
    }

    /// Buildings along both sides of a straight street `a`-`b` (`sw`
    /// wide), lots up to `depth` deep: `kinds` drawn at random, or with
    /// probability `native` the preset's own burgher townhouse.
    pub fn frontage(&mut self, a: Vec2, b: Vec2, sw: f32, depth: f32, kinds: &[BuildingKind], native: f64) {
        let d = b - a;
        let len = d.length();
        if len < 4.0 {
            return;
        }
        let dir = d / len;
        for side in [-1.0f32, 1.0] {
            let n = dir.perp() * side;
            // Local -Y (the front) points back at the street.
            let yaw = n.y.atan2(n.x) - FRAC_PI_2;
            let mut s = 1.0;
            while s < len - 4.0 {
                let kind = if self.rng.gen_bool(native.clamp(0.0, 1.0)) { None } else { self.pick(kinds) };
                let (w0, l0) = kind.map_or((self.rng.gen_range(5.5..8.0), depth.min(12.0)), |k| (k.defaults().0, k.defaults().1));
                let w = (w0 * self.rng.gen_range(0.85..1.05)).clamp(4.5, (len - s - 0.5).max(4.5));
                let l = l0.min(depth).max(4.0);
                let lot = Lot::new(a + dir * (s + w * 0.5) + n * (sw * 0.5 + l * 0.5 + 0.6), Vec2::new(w * 0.5, l * 0.5), yaw);
                let placed = match kind {
                    Some(k) if self.rng.gen_bool(self.p.density.clamp(0.0, 1.0) as f64) => {
                        let floors = k.has_floors().then(|| self.floors());
                        self.building(lot, k, floors)
                    }
                    Some(_) => false,
                    None => {
                        let before = self.used.len();
                        super::town::place(self, lot);
                        self.used.len() > before
                    }
                };
                s += w + if placed { 0.4 } else { 2.0 };
            }
        }
    }
}

/// The Prop Style street dressing pass over every logged road.
pub fn dress(plan: &mut Plan) {
    let style = plan.p.prop_style;
    if style == PropStyle::Generated || plan.p.prop_density <= 0.0 {
        return;
    }
    use BuildingKind::*;
    let kinds: &[BuildingKind] =
        if plan.p.era.is_medieval() { &[LanternPost, Cart, BarrelCrates, MarketStall] } else { &[Streetlamp, Car, Bench, Dumpster] };
    let step = 24.0 / plan.p.prop_density.clamp(0.1, 1.0);
    for (a, b, w) in plan.roads.clone() {
        let len = a.distance(b);
        let dir = (b - a) / len.max(1e-3);
        let mut s = plan.rng.gen_range(2.0..step);
        while s < len {
            let kind = kinds[plan.rng.gen_range(0..kinds.len())];
            let (kw, kl, _) = kind.defaults();
            let side = if plan.rng.gen_bool(0.5) { 1.0 } else { -1.0 };
            let n = dir.perp() * side;
            // Long props run along the street; lamps face it.
            let (half, yaw) =
                (Vec2::new(kl.max(kw) * 0.5, kl.min(kw) * 0.5), dir.y.atan2(dir.x) + if side < 0.0 { std::f32::consts::PI } else { 0.0 });
            let lot = Lot::new(a + dir * s + n * (w * 0.5 + half.y + 0.3), half, yaw);
            if plan.claim(lot) {
                if style == PropStyle::Realistic {
                    let lot = Lot { half: Vec2::new(kw, kl) * 0.5, yaw: yaw + if kw < kl { FRAC_PI_2 } else { 0.0 }, ..lot };
                    plan.piece(lot, kind, None, None);
                } else {
                    stand_in(plan, lot, kind);
                }
            }
            s += step * plan.rng.gen_range(0.7..1.3);
        }
    }
}

/// Low-poly pack: one or two boxes per prop.
fn stand_in(plan: &mut Plan, lot: Lot, kind: BuildingKind) {
    let z = plan.h(lot.c);
    let glow = if plan.p.emissive { ids::LIT } else { ids::GLASS };
    let h = lot.half;
    plan.kit.at(lot.frame(z), |k| match kind {
        BuildingKind::Streetlamp | BuildingKind::LanternPost => {
            k.mat(ids::IRON).block(Vec3::new(-0.07, -0.07, 0.0), Vec3::new(0.07, 0.07, 4.0));
            k.mat(glow).cuboid(Vec3::Z * 4.1, Vec3::splat(0.18), Quat::IDENTITY);
        }
        BuildingKind::Car => {
            k.mat(ids::RUST).block(Vec3::new(-h.x, -h.y, 0.3), Vec3::new(h.x, h.y, 0.9));
            k.mat(ids::GLASS).block(Vec3::new(-h.x * 0.5, -h.y * 0.9, 0.9), Vec3::new(h.x * 0.4, h.y * 0.9, 1.4));
        }
        _ => {
            k.mat(if kind == BuildingKind::Dumpster { ids::IRON } else { ids::TRIM })
                .block(Vec3::new(-h.x, -h.y, 0.0), Vec3::new(h.x, h.y, 1.1));
        }
    });
}

/// `[ ] Emissive Lights` off: lit windows, lamps and neon become glass.
pub fn lights_off(m: &mut EditablePoly) {
    for p in m.polygons.values_mut() {
        if p.material_id == ids::LIT || p.material_id == ids::NEON {
            p.material_id = ids::GLASS;
        }
    }
}
