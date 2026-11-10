//! Glass & gems, plastic & rubber, fabric & leather, nature, glow,
//! toon and sci-fi.

use super::*;

const fn glass(name: &'static str, color: [f32; 3], rough: f32, ior: f32) -> Spec {
    s(name, color, rough, PLAIN).fx(Fx::Glass(ior, 0.92))
}

const fn glow(name: &'static str, color: [f32; 3], k: f32) -> Spec {
    s(name, color, 0.5, PLAIN).fx(Fx::Glow(k))
}

const fn toon(name: &'static str, color: [f32; 3], steps: u32, ink: bool) -> Spec {
    s(name, color, 0.5, PLAIN).fx(Fx::Toon(steps, ink))
}

pub const GLASS: &[Spec] = &[
    glass("Clear Glass", [0.96, 0.98, 1.0], 0.02, 1.5),
    glass("Frosted Glass", [0.94, 0.96, 0.98], 0.45, 1.5),
    glass("Green Tinted Glass", [0.6, 0.85, 0.65], 0.03, 1.5),
    glass("Blue Tinted Glass", [0.6, 0.75, 0.95], 0.03, 1.5),
    glass("Bronze Tinted Glass", [0.75, 0.6, 0.45], 0.03, 1.5),
    s("Dirty Glass", [0.9, 0.92, 0.9], 0.2, dirt(2.0, 0.5, [0.45, 0.42, 0.36])).fx(Fx::Glass(1.5, 0.8)),
    s("Broken Glass", [0.95, 0.97, 1.0], 0.05, cracks(2.0, 0.02, [0.6, 0.65, 0.7])).fx(Fx::Glass(1.5, 0.85)),
    s("Stained Glass", [0.9, 0.3, 0.2], 0.1, cracks(3.0, 0.06, [0.08, 0.08, 0.08])).fx(Fx::Glass(1.5, 0.7)),
    glass("Ruby", [0.9, 0.1, 0.18], 0.02, 1.76),
    glass("Emerald", [0.12, 0.75, 0.35], 0.02, 1.58),
    glass("Sapphire", [0.12, 0.25, 0.9], 0.02, 1.77),
    glass("Diamond", [0.98, 0.99, 1.0], 0.0, 2.42),
    glass("Amethyst", [0.6, 0.3, 0.85], 0.02, 1.54),
    glass("Amber", [0.95, 0.6, 0.15], 0.05, 1.55),
    glass("Topaz", [0.95, 0.78, 0.35], 0.02, 1.62),
    glass("Aquamarine", [0.5, 0.85, 0.9], 0.02, 1.58),
    glass("Clear Ice", [0.85, 0.94, 1.0], 0.06, 1.31),
    glass("Water", [0.75, 0.9, 0.95], 0.02, 1.33),
    glass("Murky Water", [0.45, 0.55, 0.4], 0.1, 1.33),
    glass("Quartz Crystal", [0.93, 0.95, 1.0], 0.04, 1.55),
];

pub const PLASTIC: &[Spec] = &[
    s("Glossy White Plastic", [0.92, 0.92, 0.9], 0.15, PLAIN),
    s("Glossy Black Plastic", [0.04, 0.04, 0.045], 0.12, PLAIN),
    s("Glossy Red Plastic", [0.8, 0.08, 0.06], 0.15, PLAIN),
    s("Matte Grey Plastic", [0.5, 0.5, 0.5], 0.7, PLAIN),
    s("Soft-Touch Black", [0.08, 0.08, 0.085], 0.9, PLAIN),
    s("Car Tyre Rubber", [0.05, 0.05, 0.05], 0.85, noise(30.0, [0.09, 0.09, 0.09])),
    s("Silicone", [0.8, 0.82, 0.85], 0.45, PLAIN),
    s("Toy Plastic Yellow", [0.98, 0.78, 0.08], 0.25, PLAIN),
    s("Toy Plastic Blue", [0.08, 0.35, 0.85], 0.25, PLAIN),
    s("Toy Plastic Green", [0.1, 0.65, 0.2], 0.25, PLAIN),
    s("Translucent Plastic", [0.85, 0.9, 0.95], 0.2, PLAIN).fx(Fx::Glass(1.45, 0.6)),
    s("Scuffed Plastic", [0.25, 0.25, 0.26], 0.45, scratches(1.5, 0.5, [0.5, 0.5, 0.5])),
    s("Vinyl Floor", [0.7, 0.68, 0.62], 0.4, tiles(4.0, 4.0, 0.01, [0.5, 0.48, 0.44])),
    s("Rubber Mat", [0.1, 0.1, 0.1], 0.9, cells(24.0, [0.16, 0.16, 0.16])),
    s("Neoprene", [0.12, 0.12, 0.13], 0.75, noise(40.0, [0.16, 0.16, 0.17])),
];

pub const FABRIC: &[Spec] = &[
    s("Cotton", [0.88, 0.86, 0.82], 0.95, lines(120.0, 0.5, 0.0, [0.8, 0.78, 0.74])),
    s("Denim", [0.2, 0.3, 0.5], 0.95, lines(150.0, 0.4, 0.0, [0.35, 0.45, 0.62])),
    s("Red Velvet", [0.55, 0.05, 0.1], 0.8, noise(10.0, [0.35, 0.02, 0.06])),
    s("Silk", [0.9, 0.85, 0.75], 0.3, lines(200.0, 0.5, 0.0, [0.85, 0.8, 0.7])),
    s("Wool", [0.75, 0.7, 0.62], 1.0, noise(40.0, [0.6, 0.55, 0.48])),
    s("Canvas", [0.82, 0.76, 0.62], 0.95, tiles(60.0, 60.0, 0.2, [0.7, 0.64, 0.5])),
    s("Sacking / Burlap", [0.62, 0.5, 0.32], 1.0, tiles(40.0, 40.0, 0.3, [0.42, 0.33, 0.2])),
    s("Brown Leather", [0.4, 0.22, 0.12], 0.55, cells(40.0, [0.3, 0.16, 0.08])),
    s("Black Leather", [0.06, 0.05, 0.05], 0.45, cells(40.0, [0.1, 0.09, 0.09])),
    s("Suede", [0.6, 0.45, 0.3], 0.95, noise(30.0, [0.5, 0.36, 0.22])),
    s("Tartan", [0.6, 0.1, 0.1], 0.9, tiles(12.0, 12.0, 0.3, [0.1, 0.2, 0.15])),
    s("Carpet", [0.35, 0.3, 0.45], 1.0, noise(50.0, [0.28, 0.24, 0.36])),
];

pub const NATURE: &[Spec] = &[
    s("Skin Light", [0.92, 0.74, 0.62], 0.5, noise(20.0, [0.86, 0.66, 0.55])),
    s("Skin Medium", [0.74, 0.52, 0.38], 0.5, noise(20.0, [0.66, 0.45, 0.32])),
    s("Skin Dark", [0.4, 0.26, 0.18], 0.45, noise(20.0, [0.34, 0.21, 0.14])),
    s("Candle Wax", [0.95, 0.9, 0.78], 0.4, PLAIN),
    s("Bone", [0.88, 0.84, 0.72], 0.6, noise(8.0, [0.76, 0.7, 0.58])),
    s("Tree Bark", [0.35, 0.26, 0.18], 0.95, cracks(4.0, 0.15, [0.15, 0.1, 0.07])),
    s("Birch Bark", [0.9, 0.88, 0.84], 0.85, lines(14.0, 0.06, 2.0, [0.15, 0.13, 0.12])),
    s("Green Leaves", [0.2, 0.42, 0.12], 0.6, cells(12.0, [0.14, 0.3, 0.08])),
    s("Autumn Leaves", [0.8, 0.4, 0.1], 0.7, cells(12.0, [0.6, 0.2, 0.06])),
    s("Hay", [0.85, 0.72, 0.4], 0.95, lines(80.0, 0.4, 0.0, [0.68, 0.55, 0.28])),
    s("Coal", [0.06, 0.06, 0.06], 0.6, cells(10.0, [0.12, 0.12, 0.13])),
    s("Ash", [0.55, 0.54, 0.52], 1.0, noise(20.0, [0.4, 0.39, 0.38])),
    s("Fur", [0.5, 0.36, 0.22], 1.0, lines(300.0, 0.5, 0.0, [0.35, 0.24, 0.14])),
    s("Reptile Scales", [0.3, 0.42, 0.2], 0.5, cells(20.0, [0.12, 0.18, 0.08])),
    s("Coral", [0.95, 0.5, 0.45], 0.8, cells(14.0, [0.8, 0.35, 0.32])),
];

pub const GLOW: &[Spec] = &[
    glow("Neon Pink", [1.0, 0.2, 0.7], 6.0),
    glow("Neon Blue", [0.2, 0.5, 1.0], 6.0),
    glow("Neon Green", [0.3, 1.0, 0.3], 6.0),
    glow("Neon Red", [1.0, 0.15, 0.1], 6.0),
    glow("Warm LED", [1.0, 0.82, 0.6], 8.0),
    glow("Cool LED", [0.8, 0.9, 1.0], 8.0),
    glow("Screen Glow", [0.6, 0.8, 1.0], 3.0),
    s("Lit Night Windows", [0.1, 0.1, 0.12], 0.2, tiles(4.0, 6.0, 0.25, [0.1, 0.1, 0.12])).fx(Fx::Glow(2.5)),
    glow("Fire", [1.0, 0.45, 0.1], 10.0),
    s("Embers", [0.08, 0.05, 0.04], 0.8, cracks(4.0, 0.1, [1.0, 0.3, 0.05])).fx(Fx::Glow(3.0)),
    s("Molten Metal", [1.0, 0.5, 0.1], 0.3, noise(3.0, [1.0, 0.8, 0.3])).fx(Fx::Glow(8.0)),
    glow("Lava", [1.0, 0.3, 0.05], 6.0),
    glow("Hologram Cyan", [0.2, 0.95, 1.0], 4.0),
    glow("Glowing Mushroom", [0.4, 1.0, 0.8], 3.0),
    glow("Street Lamp Sodium", [1.0, 0.65, 0.25], 8.0),
];

pub const TOON: &[Spec] = &[
    toon("Toon Red", [0.9, 0.2, 0.2], 2, true),
    toon("Toon Blue", [0.2, 0.45, 0.9], 2, true),
    toon("Toon Skin", [1.0, 0.8, 0.66], 2, true),
    toon("Toon Grass", [0.35, 0.75, 0.3], 3, true),
    toon("Three-Tone Purple", [0.6, 0.35, 0.85], 3, false),
    s("Comic Ink White", [0.95, 0.95, 0.92], 0.6, PLAIN).fx(Fx::Toon(2, true)),
    toon("Pastel Pink", [1.0, 0.78, 0.85], 3, false),
    toon("Pastel Mint", [0.7, 0.95, 0.85], 3, false),
    s("Flat Orange", [1.0, 0.55, 0.15], 0.5, PLAIN).fx(Fx::Unlit),
    s("Flat Teal", [0.1, 0.6, 0.6], 0.5, PLAIN).fx(Fx::Unlit),
    toon("Retro Mustard", [0.85, 0.65, 0.15], 2, true),
    toon("Retro Olive", [0.45, 0.5, 0.2], 2, true),
];

pub const SCIFI: &[Spec] = &[
    s("Carbon Fibre", [0.08, 0.08, 0.09], 0.25, tiles(40.0, 40.0, 0.0, [0.16, 0.16, 0.18])),
    s("Hologram", [0.2, 0.9, 1.0], 0.2, lines(60.0, 0.4, 0.0, [0.05, 0.3, 0.35])).fx(Fx::Glow(3.0)),
    s("Force Field", [0.4, 0.7, 1.0], 0.1, cells(6.0, [0.1, 0.2, 0.5])).fx(Fx::Glass(1.05, 0.7)),
    s("Alien Chitin", [0.22, 0.3, 0.18], 0.3, cells(10.0, [0.08, 0.1, 0.06])),
    s("Worn Hull Panel", [0.55, 0.56, 0.58], 0.4, tiles(3.0, 3.0, 0.03, [0.25, 0.25, 0.26])).metal(0.8),
    s("Dark Hull Plating", [0.2, 0.21, 0.23], 0.35, tiles(4.0, 2.0, 0.03, [0.1, 0.1, 0.11])).metal(0.9),
    s("Circuit Board", [0.05, 0.3, 0.15], 0.4, lines(30.0, 0.15, 4.0, [0.8, 0.65, 0.3])),
    s("Energy Core", [0.5, 0.2, 1.0], 0.2, noise(3.0, [0.9, 0.6, 1.0])).fx(Fx::Glow(6.0)),
    s("Stealth Coating", [0.1, 0.11, 0.12], 0.9, PLAIN),
    s("Plasma Conduit", [0.3, 0.9, 0.4], 0.2, corrugated(10.0, [0.05, 0.3, 0.1])).fx(Fx::Glow(4.0)),
];
