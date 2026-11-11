//! The browser's material-kind filter.

use super::Item;

/// The `[ Procedural | Photo | Both ]` filter of the library browser.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Kind {
    Procedural,
    Photo,
    #[default]
    Both,
}

impl Kind {
    pub const ALL: [Kind; 3] = [Kind::Procedural, Kind::Photo, Kind::Both];
    pub fn label(self) -> &'static str {
        match self {
            Kind::Procedural => "Procedural",
            Kind::Photo => "Photo",
            Kind::Both => "Both",
        }
    }
    pub fn shows(self, item: Item) -> bool {
        match self {
            Kind::Both => true,
            Kind::Photo => item.is_photo(),
            Kind::Procedural => !item.is_photo(),
        }
    }
}
