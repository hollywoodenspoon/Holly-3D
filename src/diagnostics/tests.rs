use super::breadcrumbs::{CAPACITY, Ring};
use super::recovery::{self, PENDING_FILE, RECOVERY_FILE};
use super::report::{self, BuildInfo, CrashInfo};
use super::state::Observer;
use crate::editor::{Editor, Tool};
use crate::primitives::PrimitiveKind;
use bevy::math::Vec3;
use std::path::PathBuf;

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("ilex_crash_{name}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

const BT: &str = "   0: std::backtrace::Backtrace::force_capture
             at /rustc/library/std/src/backtrace.rs:312:13
   1: ilex::diagnostics::hook::install::{{closure}}
             at ./src/diagnostics/hook.rs:70:28
   2: ilex::mesh::ops::extrude::extrude_faces
             at ./src/mesh/ops/extrude.rs:41:9
   3: <ilex::editor::Editor>::apply_op
             at ./src/editor/actions.rs:99:5
   4: bevy_ecs::system::run
";

#[test]
fn ring_keeps_only_the_last_50_and_folds_repeats() {
    let mut r = Ring::new(CAPACITY);
    for i in 0..120 {
        r.push(i as f64, "Action", format!("step {i}"));
    }
    assert_eq!(r.len(), CAPACITY);
    let c = r.crumbs();
    assert_eq!(c.first().unwrap().text, "step 70");
    assert_eq!(c.last().unwrap().text, "step 119");
    assert_eq!(r.total, 120);
    // Identical consecutive entries fold into one line with a count.
    r.push(200.0, "Evaluate", "Box001");
    r.push(200.1, "Evaluate", "Box001");
    r.push(200.2, "Evaluate", "Box001");
    let c = r.crumbs();
    assert_eq!(r.len(), CAPACITY);
    assert_eq!(c.last().unwrap().repeat, 3);
    assert!(c.last().unwrap().line().contains("×3"));
    assert_eq!(c.last().unwrap().text, "Box001");
}

#[test]
fn utc_timestamps() {
    assert_eq!(report::timestamp(0), "1970-01-01 00:00:00 UTC");
    assert_eq!(report::timestamp(1_700_000_000), "2023-11-14 22:13:20 UTC");
    assert_eq!(report::file_stamp(951_782_400), "20000229-000000");
}

#[test]
fn own_frames_skip_the_reporter_and_libraries() {
    let all = report::frames(BT);
    assert_eq!(all.len(), 5);
    let own = report::own_frames(BT);
    let names: Vec<&str> = own.iter().map(|f| f.function.as_str()).collect();
    assert_eq!(names, ["ilex::mesh::ops::extrude::extrude_faces", "<ilex::editor::Editor>::apply_op"]);
    assert_eq!(own[0].at, ["./src/mesh/ops/extrude.rs:41:9"]);
}

#[test]
fn report_has_the_five_sections_in_order() {
    let mut r = Ring::new(CAPACITY);
    r.push(1.0, "Hotkey", "W -> Select and Move");
    r.push(2.5, "Op", "Extrude");
    let info = CrashInfo {
        message: "index out of bounds".into(),
        location: None,
        thread: "main".into(),
        backtrace: BT.into(),
        unix_time: 1_700_000_000,
        cause: "panic".into(),
    };
    let build = BuildInfo { version: "0.1.0".into(), git: "abc123".into(), os: "Linux".into(), kernel: "6.1".into(), gpu: "Test GPU".into() };
    let md = report::format(&info, &build, Some(&r.crumbs()), Some("- **Objects in scene:** 3\n"));
    let heads = [
        "## 1. Crash Summary & Location",
        "## 2. Last 50 User Actions (Flight Recorder Breadcrumbs)",
        "## 3. Active Scene & Selection State",
        "## 4. Ilex Stack Frames",
        "## 5. Full Backtrace",
    ];
    let pos: Vec<usize> = heads.iter().map(|h| md.find(h).unwrap_or_else(|| panic!("missing {h}"))).collect();
    assert!(pos.windows(2).all(|w| w[0] < w[1]));
    // With no panic location, the innermost Ilex frame is the location.
    assert!(md.contains("**Location:** `./src/mesh/ops/extrude.rs:41:9`"));
    assert!(md.contains("1. `+    1.00s` **Hotkey** W -> Select and Move"));
    assert!(md.contains("2. `+    2.50s` **Op** Extrude"));
    assert!(md.contains("Objects in scene:** 3"));
    assert!(md.contains("abc123") && md.contains("Test GPU") && md.contains("2023-11-14"));
    assert!(!md[pos[3]..pos[4]].contains("diagnostics::hook"));
    // Fenced blocks are balanced so the Markdown renders.
    assert_eq!(md.matches("```").count() % 2, 0);
    // A busy recorder still yields a valid report.
    let md = report::format(&info, &build, None, None);
    assert!(md.contains("flight recorder was busy"));
}

#[test]
fn bundle_writes_report_and_loadable_emergency_scene() {
    let dir = tmp("bundle");
    let mut ed = Editor::default();
    ed.create_primitive(PrimitiveKind::Box, Vec3::ZERO);
    ed.create_primitive(PrimitiveKind::Sphere, Vec3::X * 2.0);
    let b = recovery::write_bundle(&dir, "20261111-120000", "# report", Some(&ed.scene)).unwrap();
    assert_eq!(b.report, dir.join("crash_20261111-120000.md"));
    assert_eq!(b.recovery, Some(dir.join(RECOVERY_FILE)));
    let back = crate::io::scene_file::load(&dir.join(RECOVERY_FILE)).unwrap();
    assert_eq!(back.objects.len(), 2);
    // Restoring through the editor gives an unsaved scene with both objects.
    let mut ed2 = Editor::default();
    ed2.open_scene(&dir.join(RECOVERY_FILE)).unwrap();
    assert_eq!(ed2.scene.objects.len(), 2);
    // A second crash in the same second does not overwrite the first.
    let b2 = recovery::write_bundle(&dir, "20261111-120000", "# second", None).unwrap();
    assert_eq!(b2.report, dir.join("crash_20261111-120000-2.md"));
    assert_eq!(recovery::list_reports(&dir).len(), 2);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn next_launch_detects_then_dismiss_and_clear() {
    let dir = tmp("pending");
    assert!(recovery::detect_pending(&dir).is_none(), "no folder, no dialog");
    recovery::write_bundle(&dir, "20261110-080000", "# old", None).unwrap();
    recovery::write_bundle(&dir, "20261111-090000", "# new crash", Some(&Editor::default().scene)).unwrap();
    let p = recovery::detect_pending(&dir).expect("pending crash");
    assert_eq!(p.report, dir.join("crash_20261111-090000.md"));
    assert_eq!(p.text, "# new crash");
    assert!(p.recovery.is_some());
    assert_eq!(recovery::list_reports(&dir)[0], p.report, "newest first");
    recovery::dismiss(&dir);
    assert!(recovery::detect_pending(&dir).is_none());
    assert!(!dir.join(PENDING_FILE).exists());
    assert_eq!(recovery::list_reports(&dir).len(), 2, "dismiss keeps the reports");
    assert_eq!(recovery::clear(&dir), 3);
    assert!(recovery::list_reports(&dir).is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn observer_records_tool_subobject_and_dialog_changes() {
    let mut ed = Editor::default();
    let mut o = Observer::default();
    o.changes(&ed);
    assert!(o.changes(&ed).is_empty(), "nothing changed");
    ed.tool = Tool::Create(PrimitiveKind::Box);
    ed.mat_editor.open = true;
    let v = o.changes(&ed);
    assert!(v.iter().any(|(k, t)| *k == "Tool" && t == "Create"), "{v:?}");
    assert!(v.iter().any(|(k, t)| *k == "Dialog" && t == "Material Editor opened"), "{v:?}");
    assert!(o.changes(&ed).is_empty(), "each change is recorded once");
}

#[test]
fn scene_summary_names_the_selection_and_stack() {
    let mut ed = Editor::default();
    ed.create_primitive(PrimitiveKind::Box, Vec3::ZERO);
    ed.add_modifier("Bend");
    let s = super::state::describe(&ed);
    assert!(s.contains("Objects in scene:** 1"), "{s}");
    assert!(s.contains("Primitive") && s.contains("Bend"), "{s}");
    assert!(s.contains("verts"), "{s}");
}
