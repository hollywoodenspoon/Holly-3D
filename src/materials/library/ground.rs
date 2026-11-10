//! Ground & terrain, and roads & urban ground.

use super::*;

pub const GROUND: &[Spec] = &[
    s("Fresh Grass", [0.24, 0.42, 0.12], 0.85, noise(9.0, [0.36, 0.55, 0.16])),
    s("Dry Grass", [0.55, 0.5, 0.26], 0.9, noise(9.0, [0.68, 0.6, 0.34])),
    s("Patchy Grass", [0.3, 0.42, 0.15], 0.9, dirt(1.5, 0.45, [0.42, 0.33, 0.22])),
    s("Lawn", [0.2, 0.4, 0.1], 0.8, noise(14.0, [0.28, 0.5, 0.14])),
    s("Dry Mud", [0.45, 0.36, 0.26], 0.95, cracks(3.0, 0.05, [0.24, 0.18, 0.12])),
    s("Wet Mud", [0.24, 0.18, 0.12], 0.35, noise(4.0, [0.32, 0.25, 0.17])),
    s("Gravel", [0.5, 0.48, 0.45], 0.9, cells(14.0, [0.32, 0.3, 0.28])),
    s("Pebbles", [0.58, 0.55, 0.5], 0.7, cells(8.0, [0.3, 0.28, 0.26])),
    s("Beach Sand", [0.86, 0.78, 0.6], 0.9, noise(18.0, [0.78, 0.7, 0.52])),
    s("Desert Sand", [0.84, 0.64, 0.4], 0.9, noise(3.0, [0.92, 0.74, 0.5])),
    s("Wet Sand", [0.6, 0.52, 0.38], 0.45, noise(14.0, [0.52, 0.45, 0.32])),
    s("Red Clay", [0.62, 0.32, 0.2], 0.85, noise(5.0, [0.5, 0.24, 0.15])),
    s("Cracked Earth", [0.66, 0.52, 0.36], 0.95, cracks(2.5, 0.06, [0.2, 0.14, 0.09])),
    s("Forest Floor", [0.32, 0.24, 0.14], 0.95, cells(6.0, [0.22, 0.28, 0.12])),
    s("Dirt Path", [0.46, 0.36, 0.25], 0.9, noise(6.0, [0.56, 0.45, 0.32])),
    s("Rocky Ground", [0.48, 0.44, 0.4], 0.9, cracks(2.0, 0.1, [0.34, 0.28, 0.22])),
    s("Fresh Snow", [0.94, 0.95, 0.98], 0.6, noise(5.0, [0.84, 0.87, 0.94])),
    s("Packed Snow", [0.85, 0.87, 0.9], 0.5, dirt(2.0, 0.25, [0.66, 0.66, 0.66])),
    s("Slush", [0.7, 0.72, 0.74], 0.25, dirt(1.8, 0.5, [0.42, 0.38, 0.34])),
    s("Ice", [0.72, 0.85, 0.94], 0.08, cracks(2.0, 0.02, [0.9, 0.95, 1.0])),
    s("Moss", [0.28, 0.42, 0.14], 0.95, cells(10.0, [0.2, 0.32, 0.1])),
    s("Lava Crust", [0.12, 0.1, 0.09], 0.9, cracks(2.0, 0.12, [1.0, 0.35, 0.05])).fx(Fx::Glow(2.0)),
    s("Volcanic Ash", [0.25, 0.24, 0.24], 0.95, noise(7.0, [0.17, 0.16, 0.16])),
    s("Peat", [0.2, 0.14, 0.09], 0.9, noise(5.0, [0.28, 0.2, 0.12])),
    s("Salt Flat", [0.92, 0.9, 0.86], 0.8, cracks(1.5, 0.04, [0.72, 0.7, 0.66])),
    s("Tundra", [0.45, 0.45, 0.35], 0.9, dirt(2.0, 0.4, [0.6, 0.55, 0.45])),
    s("Savanna Earth", [0.66, 0.5, 0.3], 0.9, dirt(1.5, 0.4, [0.55, 0.52, 0.25])),
    s("Clay Soil", [0.52, 0.4, 0.3], 0.85, noise(4.0, [0.6, 0.46, 0.34])),
    s("Leaf Litter", [0.45, 0.28, 0.12], 0.9, cells(9.0, [0.62, 0.42, 0.16])),
    s("Alpine Meadow", [0.32, 0.46, 0.18], 0.9, dirt(1.2, 0.3, [0.5, 0.48, 0.42])),
];

const ASPHALT: [f32; 3] = [0.2, 0.2, 0.21];
const WHITE_LINE: [f32; 3] = [0.92, 0.92, 0.9];
const YELLOW_LINE: [f32; 3] = [0.92, 0.72, 0.12];
const CONCRETE: [f32; 3] = [0.62, 0.61, 0.58];

pub const ROADS: &[Spec] = &[
    s("New Asphalt", [0.12, 0.12, 0.13], 0.85, noise(30.0, [0.2, 0.2, 0.21])),
    s("Worn Asphalt", ASPHALT, 0.9, dirt(1.5, 0.4, [0.36, 0.35, 0.33])),
    s("Cracked Asphalt", ASPHALT, 0.9, cracks(1.5, 0.03, [0.08, 0.08, 0.08])),
    s("Wet Asphalt", [0.1, 0.1, 0.11], 0.12, stains(1.5, 0.5, [0.2, 0.2, 0.21])),
    s("Patched Asphalt", ASPHALT, 0.9, stains(0.8, 0.55, [0.1, 0.1, 0.11])),
    s("Concrete Road", CONCRETE, 0.85, tiles(1.0, 2.0, 0.02, [0.3, 0.3, 0.3])),
    s("Road: White Centre Line", ASPHALT, 0.85, lines(1.0, 0.04, 0.0, WHITE_LINE)),
    s("Road: Yellow Centre Line", ASPHALT, 0.85, lines(1.0, 0.04, 0.0, YELLOW_LINE)),
    s("Road: Dashed White Line", ASPHALT, 0.85, lines(1.0, 0.04, 3.0, WHITE_LINE)),
    s("Road: Dashed Yellow Line", ASPHALT, 0.85, lines(1.0, 0.04, 3.0, YELLOW_LINE)),
    s("Road: Edge Lines", WHITE_LINE, 0.85, lines(1.0, 0.92, 0.0, ASPHALT)),
    s("Crosswalk Stripes", ASPHALT, 0.8, lines(6.0, 0.5, 0.0, WHITE_LINE)),
    s("Sidewalk Slabs", CONCRETE, 0.85, tiles(4.0, 4.0, 0.03, [0.42, 0.41, 0.39])),
    s("Curb Stone", [0.55, 0.55, 0.54], 0.8, tiles(2.0, 1.0, 0.04, [0.35, 0.35, 0.34])),
    s("Brick Paving", [0.56, 0.28, 0.2], 0.85, bricks(4.0, 8.0, 0.06, [0.4, 0.38, 0.35])),
    s("Modern Cobbles", [0.45, 0.44, 0.42], 0.75, cobbles(6.0, 8.0, 0.08, [0.25, 0.24, 0.22])),
    s("Medieval Cobbles", [0.5, 0.46, 0.4], 0.8, cobbles(5.0, 6.0, 0.16, [0.3, 0.26, 0.2])),
    s("Flagstone Paving", [0.55, 0.52, 0.46], 0.85, cracks(2.0, 0.1, [0.3, 0.28, 0.24])),
    s("Manhole Iron", [0.3, 0.3, 0.31], 0.55, lines(12.0, 0.4, 0.0, [0.2, 0.2, 0.21])).metal(0.8),
    s("Tactile Paving", [0.86, 0.72, 0.18], 0.7, cells(20.0, [0.72, 0.58, 0.1])),
    s("Parking Lot", ASPHALT, 0.85, lines(4.0, 0.03, 0.0, WHITE_LINE)),
    s("Gravel Road", [0.55, 0.5, 0.44], 0.95, cells(18.0, [0.4, 0.36, 0.3])),
    s("Dirt Road", [0.5, 0.4, 0.28], 0.9, dirt(1.0, 0.35, [0.38, 0.29, 0.2])),
    s("Interlocking Pavers", [0.58, 0.5, 0.42], 0.8, bricks(6.0, 6.0, 0.05, [0.35, 0.3, 0.25])),
    s("Terracotta Floor Tiles", [0.72, 0.42, 0.28], 0.7, tiles(4.0, 4.0, 0.05, [0.82, 0.78, 0.7])),
    s("Checkered Floor", [0.9, 0.9, 0.88], 0.4, Pat::Bricks(BrickKind::Stack, 8.0, 8.0, 0.0, [0.06, 0.06, 0.06])),
    s("Oily Concrete", CONCRETE, 0.75, stains(1.2, 0.6, [0.25, 0.24, 0.22])),
    s("Painted Kerb Red-White", WHITE_LINE, 0.7, Pat::Stripes(4.0, 0.5, 0.0, false, [0.8, 0.12, 0.1])),
    s("Bike Lane Green", [0.2, 0.55, 0.3], 0.8, dirt(2.0, 0.3, ASPHALT)),
    s("Bus Lane Red", [0.6, 0.18, 0.15], 0.8, dirt(2.0, 0.3, ASPHALT)),
];
