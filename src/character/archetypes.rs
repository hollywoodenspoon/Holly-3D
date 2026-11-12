//! One-click RPG archetypes: MakeHuman macro + micro slider recipes.

use super::params::{Archetype, CharacterParams, Hair, Outfit, Weapon};

impl Archetype {
    /// Resets `p` to this archetype (keeps the seed, eyes and creature).
    pub fn apply(self, p: &mut CharacterParams) {
        let keep = (p.seed, p.eyes);
        *p = CharacterParams { archetype: self, seed: keep.0, eyes: keep.1, ..Default::default() };
        let m = &mut p.macros;
        let recipe: &[(&str, f32)] = match self {
            Archetype::Human => &[],
            Archetype::Elf => {
                (m.height, m.weight, m.muscle) = (0.8, 0.3, 0.45);
                (p.hair, p.outfit, p.weapon) = (Hair::Long, Outfit::Ranger, Weapon::Staff);
                &[
                    ("ears/r-ear-shape-pointed|triangle", -1.0),
                    ("ears/r-ear-scale-vert-decr|incr", 0.8),
                    ("ears/r-ear-rot-backward|forward", -0.5),
                    ("chin/chin-width-decr|incr", -0.5),
                    ("cheek/r-cheek-bones-decr|incr", 0.6),
                    ("nose/nose-scale-horiz-decr|incr", -0.4),
                ]
            }
            Archetype::Dwarf => {
                (m.gender, m.height, m.weight, m.muscle, m.age) = (1.0, 0.0, 0.75, 0.85, 0.6);
                (p.hair, p.outfit, p.weapon) = (Hair::Beard, Outfit::Knight, Weapon::Axe);
                &[
                    ("torso/torso-scale-horiz-decr|incr", 0.6),
                    ("armslegs/lowerlegs-height-decr|incr", -0.8),
                    ("armslegs/upperlegs-height-decr|incr", -0.8),
                    ("nose/nose-volume-decr|incr", 0.7),
                    ("head/head-scale-horiz-decr|incr", 0.3),
                ]
            }
            Archetype::Goblin => {
                (m.height, m.weight, m.muscle, m.gender) = (0.0, 0.2, 0.3, 0.7);
                (p.hair, p.outfit, p.weapon) = (Hair::Bald, Outfit::Tunic, Weapon::Sword);
                &[
                    ("ears/r-ear-shape-pointed|triangle", -1.0),
                    ("ears/r-ear-scale-decr|incr", 1.0),
                    ("ears/r-ear-wing-decr|incr", 1.0),
                    ("nose/nose-point-width-decr|incr", 0.8),
                    ("nose/nose-scale-depth-decr|incr", 1.0),
                    ("head/head-scale-horiz-decr|incr", 0.4),
                    ("chin/chin-prognathism-decr|incr", 0.6),
                ]
            }
            Archetype::Orc => {
                (m.gender, m.height, m.weight, m.muscle) = (1.0, 0.9, 0.7, 1.0);
                (m.african, m.asian, m.caucasian) = (0.6, 0.1, 0.3);
                (p.hair, p.outfit, p.weapon) = (Hair::Ponytail, Outfit::Ranger, Weapon::Axe);
                &[
                    ("chin/chin-prognathism-decr|incr", 1.0),
                    ("chin/chin-width-decr|incr", 1.0),
                    ("ears/r-ear-shape-pointed|triangle", -0.6),
                    ("nose/nose-scale-horiz-decr|incr", 0.8),
                    ("torso/torso-vshape-decr|incr", 1.0),
                    ("neck/neck-scale-horiz-decr|incr", 1.0),
                ]
            }
            Archetype::Chibi => {
                (m.height, m.age, m.weight) = (0.0, 0.25, 0.55);
                (p.anime, p.hair, p.outfit) = (1.0, Hair::Spiky, Outfit::Tunic);
                &[
                    ("head/head-scale-vert-decr|incr", 1.0),
                    ("head/head-scale-horiz-decr|incr", 1.0),
                    ("head/head-scale-depth-decr|incr", 1.0),
                    ("armslegs/lowerlegs-height-decr|incr", -1.0),
                    ("armslegs/upperlegs-height-decr|incr", -1.0),
                ]
            }
            Archetype::AnimeHero => {
                (m.gender, m.height, m.muscle, m.weight) = (1.0, 0.75, 0.65, 0.35);
                (p.anime, p.hair, p.outfit, p.weapon) = (0.8, Hair::Spiky, Outfit::Knight, Weapon::Sword);
                &[("torso/torso-vshape-decr|incr", 0.7)]
            }
            Archetype::AnimeHeroine => {
                (m.gender, m.height, m.weight) = (0.0, 0.6, 0.35);
                (p.anime, p.hair, p.outfit, p.weapon) = (0.9, Hair::Ponytail, Outfit::Tunic, Weapon::Staff);
                &[("armslegs/lowerlegs-height-decr|incr", 0.5)]
            }
        };
        for (k, v) in recipe {
            p.set_micro(k, *v);
        }
    }
}
