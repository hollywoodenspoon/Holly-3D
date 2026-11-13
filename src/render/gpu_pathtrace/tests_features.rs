//! Feature switches, fallback reasons, batch sizing and the kernel build:
//! every switch combination must parse and validate.

use super::features::{Features, SPP_RANGE, TARGET_MS, join, next_spp, reasons};
use crate::render::scene_build::RenderScene;
use crate::scene::Scene;
use crate::viewport::camera::{ViewCamera, ViewKind};

fn empty() -> RenderScene {
    RenderScene::build(&Scene::default(), ViewCamera::new(ViewKind::Perspective), false)
}

#[test]
fn every_switch_combination_of_the_kernel_validates() {
    let src = super::source();
    for f in Features::NAMES {
        assert!(src.contains(&format!("override {f}: bool = false;")), "{f} is declared");
    }
    for mask in 0..Features::COUNT {
        let mut s = src.clone();
        for (i, f) in Features::NAMES.iter().enumerate() {
            s = s.replace(&format!("override {f}: bool = false;"), &format!("const {f}: bool = {};", mask >> i & 1 == 1));
        }
        let module = naga::front::wgsl::parse_str(&s).unwrap_or_else(|e| panic!("mask {mask:#09b}: {}", e.emit_to_string(&s)));
        naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::all())
            .validate(&module)
            .unwrap_or_else(|e| panic!("mask {mask:#09b}: {e:?}"));
    }
}

/// With an adapter, every mask also builds a real pipeline.
#[test]
fn every_feature_mask_builds_a_gpu_pipeline() {
    let Ok(g) = super::gpu() else { return };
    for mask in 0..Features::COUNT {
        let layout = g.pipeline(Features(mask)).get_bind_group_layout(0);
        drop(layout);
        // Cached once, then freed: holding every mask's driver code at once
        // takes over 10 GB on a software rasteriser (other tests rebuild theirs).
        let cached = g.pipelines.lock().unwrap().remove(&mask);
        assert!(cached.is_some(), "mask {mask:#b} is cached");
    }
}

#[test]
fn constants_follow_the_mask_bits() {
    let c = Features(Features::TEXTURES | Features::AREA).constants();
    assert_eq!(c.len(), Features::NAMES.len());
    let on: Vec<_> = c.iter().filter(|(_, v)| *v == 1.0).map(|(n, _)| *n).collect();
    assert_eq!(on, ["FEAT_TEXTURES", "FEAT_AREA"]);
}

#[test]
fn a_plain_scene_needs_no_switches_and_no_fallback() {
    let rs = empty();
    assert_eq!(Features::of(&rs), Features(0));
    assert!(reasons(&rs).is_empty());
}

#[test]
fn every_fallback_reason_is_listed_once() {
    let mut rs = empty();
    rs.translucent = true;
    rs.procedural = Some(Default::default());
    let r = reasons(&rs);
    assert_eq!(r, ["see-through (opacity) surfaces"], "the sky renders on the GPU now");
    assert_eq!(super::pack::unsupported(&rs), Some(r[0]));
    let f = Features::of(&rs);
    assert_eq!(f.0, Features::ENV_MAP | Features::OPACITY);
}

#[test]
fn reasons_join_into_one_sentence() {
    assert_eq!(join(&[]), "");
    assert_eq!(join(&["a"]), "a");
    assert_eq!(join(&["a", "b"]), "a and b");
    assert_eq!(join(&["a", "b", "c"]), "a, b and c");
}

#[test]
fn batch_size_follows_the_submit_time() {
    let (lo, hi) = SPP_RANGE;
    assert_eq!(next_spp(1, 5.0, TARGET_MS), 2, "fast submits grow, at most doubling");
    assert_eq!(next_spp(4, 25.0, TARGET_MS), hi);
    assert_eq!(next_spp(8, 5.0, TARGET_MS), hi, "never above the cap");
    assert_eq!(next_spp(8, 400.0, TARGET_MS), 4, "slow submits shrink, at most halving");
    assert_eq!(next_spp(1, 1000.0, TARGET_MS), lo, "never below one sample");
    assert_eq!(next_spp(4, 50.0, TARGET_MS), 4, "on target stays put");
    assert_eq!(next_spp(2, 0.0, TARGET_MS), 4, "an unmeasurable time counts as fast");
}

#[test]
fn the_device_label_names_the_driver() {
    assert_eq!(super::device_label("RX", "Vulkan", "RADV"), "GPU: RX (Vulkan, RADV)");
    assert_eq!(super::device_label("RX", "Vulkan", " "), "GPU: RX (Vulkan)");
}
