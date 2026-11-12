//! Phase 10B Easy Pose rollout: one-click pose presets, a joint picker
//! with Bend / Twist / Side sliders (mirrored L/R by default), side copy
//! and mirror buttons, and `.ilexpose` Save / Load (`.mmaxpose` opens too).

use crate::character::params::CharacterParams;
use crate::character::rig::hands::{set_hand, HandShape};
use crate::character::rig::pose::{mirror, twin, CONTROLS};
use crate::character::rig::presets::PosePreset;
use crate::character::rig::{pose_file, Pose};
use crate::io::scene_file::{LEGACY_POSE_EXTS, POSE_EXT};
use crate::ui::command::CONTENT_W;
use crate::ui::widgets::{Track, dim, rollout};
use bevy_egui::egui;

#[derive(Clone, Copy, Default)]
struct PoseUi {
    control: usize,
    right: bool,
    unmirrored: bool,
}

fn set_pose(t: &mut Track, p: &mut CharacterParams, pose: Pose) {
    p.pose = pose;
    t.started = true;
    t.changed = true;
}

pub fn rollout_ui(ui: &mut egui::Ui, t: &mut Track, p: &mut CharacterParams) {
    rollout(ui, "Character — Easy Pose (Rig)", true, |ui| {
        let id = egui::Id::new("char_easy_pose");
        let mut st: PoseUi = ui.data_mut(|d| d.get_temp(id).unwrap_or_default());
        let msg_id = id.with("msg");
        ui.label(egui::RichText::new("Pose Presets").strong());
        let w = CONTENT_W / 2.0 - 6.0;
        egui::Grid::new("pose_presets").num_columns(2).show(ui, |ui| {
            for (i, preset) in PosePreset::ALL.iter().enumerate() {
                if ui.add_sized([w, 24.0], egui::Button::new(preset.label())).clicked() {
                    set_pose(t, p, preset.pose());
                }
                if i % 2 == 1 {
                    ui.end_row();
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("Hands:");
            for shape in HandShape::ALL {
                if ui.button(shape.label()).on_hover_text("Shape both hands (only the picked side with 'Pose this side only')").clicked() {
                    let mut q = p.pose.clone();
                    let one = st.unmirrored;
                    set_hand(&mut q, shape, !one || !st.right, !one || st.right);
                    set_pose(t, p, q);
                }
            }
        });
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Joint:");
            egui::ComboBox::from_id_salt("pose_joint").width(120.0).selected_text(CONTROLS[st.control].0).show_ui(ui, |ui| {
                for (i, c) in CONTROLS.iter().enumerate() {
                    ui.selectable_value(&mut st.control, i, c.0);
                }
            });
            if CONTROLS[st.control].2.is_some() {
                ui.selectable_value(&mut st.right, false, "Left");
                ui.selectable_value(&mut st.right, true, "Right");
            }
        });
        let (_, l, r) = CONTROLS[st.control];
        let bone = if st.right { r.unwrap_or(l) } else { l };
        let mut rot = p.pose.get(bone);
        let before = rot;
        for (axis, label) in ["Bend (X)", "Twist (Y)", "Side (Z)"].iter().enumerate() {
            ui.horizontal(|ui| {
                ui.add_sized([70.0, 18.0], egui::Label::new(*label));
                let resp = t.r(ui.add(egui::Slider::new(&mut rot[axis], -180.0..=180.0).suffix("°").max_decimals(1)));
                if resp.secondary_clicked() {
                    rot[axis] = 0.0;
                    t.changed = true;
                }
            });
        }
        if r.is_some() {
            ui.checkbox(&mut st.unmirrored, "Pose this side only (no L/R mirror)");
        }
        if rot != before {
            if st.unmirrored || r.is_none() {
                p.pose.set(bone, rot);
            } else {
                p.pose.set_mirrored(bone, rot);
            }
        }
        ui.horizontal_wrapped(|ui| {
            if ui.button("Copy L to R").clicked() {
                let mut q = p.pose.clone();
                q.copy_side(true);
                set_pose(t, p, q);
            }
            if ui.button("Copy R to L").clicked() {
                let mut q = p.pose.clone();
                q.copy_side(false);
                set_pose(t, p, q);
            }
            if ui.button("Mirror Pose").on_hover_text("Swap the left and right sides").clicked() {
                let q = p.pose.mirrored();
                set_pose(t, p, q);
            }
            if ui.button("Reset Joint").clicked() {
                let mut q = p.pose.clone();
                q.set(bone, [0.0; 3]);
                if !st.unmirrored && let Some(tw) = twin(bone) {
                    q.set(&tw, mirror([0.0; 3]));
                }
                set_pose(t, p, q);
            }
            if ui.button("Reset Pose").clicked() {
                set_pose(t, p, Pose::default());
            }
        });
        ui.horizontal(|ui| {
            if ui.button("💾 Save Pose...").clicked()
                && let Some(path) = rfd::FileDialog::new().add_filter("Ilex Pose (*.ilexpose)", &[POSE_EXT]).set_file_name("Pose.ilexpose").save_file()
            {
                let m = match pose_file::save(&p.pose, &path) {
                    Ok(saved) => format!("Saved {}", saved.display()),
                    Err(e) => format!("Error: {e}"),
                };
                ui.data_mut(|d| d.insert_temp(msg_id, m));
            }
            if ui.button("📂 Load Pose...").clicked()
                && let Some(path) = rfd::FileDialog::new().add_filter("Ilex Pose (*.ilexpose, *.mmaxpose)", &[POSE_EXT, LEGACY_POSE_EXTS[0]]).pick_file()
            {
                let m = match pose_file::load(&path) {
                    Ok(q) => {
                        set_pose(t, p, q);
                        format!("Loaded {}", path.display())
                    }
                    Err(e) => format!("Error: {e}"),
                };
                ui.data_mut(|d| d.insert_temp(msg_id, m));
            }
        });
        if let Some(m) = ui.data_mut(|d| d.get_temp::<String>(msg_id)) {
            ui.label(dim(&m));
        }
        let n = p.pose.joints.len();
        ui.label(dim(&format!(
            "MakeHuman default skeleton (163 bones), fitted to this body. {n} posed joint(s). Angles are in the rest-pose axes about each joint; right-click a slider to zero it."
        )));
        ui.data_mut(|d| d.insert_temp(id, st));
    });
}
