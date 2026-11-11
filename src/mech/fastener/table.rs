//! Standard thread sizes: ISO metric coarse and Unified (UNC) inch, with
//! the hex head and nut sizes that go with them. All values in millimetres.

/// One standard size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Size {
    pub name: &'static str,
    /// Major (nominal) diameter.
    pub d: f32,
    /// Thread pitch.
    pub pitch: f32,
    /// Hex width across flats.
    pub flats: f32,
    /// Bolt head height.
    pub head: f32,
    /// Nut height.
    pub nut: f32,
}

const fn mm(name: &'static str, d: f32, pitch: f32, flats: f32, head: f32, nut: f32) -> Size {
    Size { name, d, pitch, flats, head, nut }
}

/// Inch sizes: diameter, threads per inch, flats, head and nut heights in inches.
const fn inch(name: &'static str, d: f32, tpi: f32, flats: f32, head: f32, nut: f32) -> Size {
    const IN: f32 = 25.4;
    Size { name, d: d * IN, pitch: IN / tpi, flats: flats * IN, head: head * IN, nut: nut * IN }
}

/// ISO 261 coarse pitch, ISO 4017 heads, ISO 4032 nuts.
pub const METRIC: [Size; 10] = [
    mm("M3", 3.0, 0.5, 5.5, 2.0, 2.4),
    mm("M4", 4.0, 0.7, 7.0, 2.8, 3.2),
    mm("M5", 5.0, 0.8, 8.0, 3.5, 4.7),
    mm("M6", 6.0, 1.0, 10.0, 4.0, 5.2),
    mm("M8", 8.0, 1.25, 13.0, 5.3, 6.8),
    mm("M10", 10.0, 1.5, 16.0, 6.4, 8.4),
    mm("M12", 12.0, 1.75, 18.0, 7.5, 10.8),
    mm("M16", 16.0, 2.0, 24.0, 10.0, 14.8),
    mm("M20", 20.0, 2.5, 30.0, 12.5, 18.0),
    mm("M24", 24.0, 3.0, 36.0, 15.0, 21.5),
];

/// UNC (ASME B1.1) with ASME B18.2 hex bolt and nut sizes.
pub const IMPERIAL: [Size; 7] = [
    inch("1/4\"-20", 0.25, 20.0, 0.4375, 0.156, 0.219),
    inch("5/16\"-18", 0.3125, 18.0, 0.5, 0.203, 0.266),
    inch("3/8\"-16", 0.375, 16.0, 0.5625, 0.234, 0.328),
    inch("1/2\"-13", 0.5, 13.0, 0.75, 0.312, 0.438),
    inch("5/8\"-11", 0.625, 11.0, 0.9375, 0.391, 0.547),
    inch("3/4\"-10", 0.75, 10.0, 1.125, 0.469, 0.641),
    inch("1\"-8", 1.0, 8.0, 1.5, 0.625, 0.859),
];
