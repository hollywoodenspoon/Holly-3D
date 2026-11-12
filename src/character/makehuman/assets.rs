//! Phase 10A: the bundled MakeHuman 1.x "hm08" CC0 assets
//! (`assets/makehuman/`, zlib-compressed). Each file is inflated once on
//! first use and cached for the life of the process.

use std::sync::OnceLock;

macro_rules! asset {
    ($fn:ident, $file:literal) => {
        pub fn $fn() -> &'static [u8] {
            static CELL: OnceLock<Vec<u8>> = OnceLock::new();
            CELL.get_or_init(|| inflate(include_bytes!(concat!("../../../assets/makehuman/", $file))))
        }
    };
}

asset!(base_obj, "base.obj.z");
asset!(targets_pack, "targets.mht.z");
asset!(modifiers_json, "modeling_modifiers.json.z");
asset!(eyes_high_obj, "eyes-high.obj.z");
asset!(eyes_high_mhclo, "eyes-high.mhclo.z");
asset!(eyes_low_obj, "eyes-low.obj.z");
asset!(eyes_low_mhclo, "eyes-low.mhclo.z");
asset!(skeleton_json, "default.mhskel.z");
asset!(weights_json, "default_weights.mhw.z");

fn inflate(z: &[u8]) -> Vec<u8> {
    miniz_oxide::inflate::decompress_to_vec_zlib(z).unwrap_or_default()
}

pub fn text(bytes: &'static [u8]) -> &'static str {
    std::str::from_utf8(bytes).unwrap_or("")
}
