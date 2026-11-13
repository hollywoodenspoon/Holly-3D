//! Pivot tab -> Assembly rollout: the J tool, the joint type it creates and
//! the parameters of the joint that places the current part, IK Drag,
//! Grounded parts and Mirror Mech Limb.

use super::content_w;
use crate::assembly::{Axis, JointKind, JointType, Limits};
use crate::editor::mirror::MirrorAxis;
use crate::editor::{Editor, Tool};
use crate::ui::widgets::{Track, action, dim, max_spinner, rollout, toggle_button};
use bevy_egui::egui;

fn grid(ui: &mut egui::Ui, id: &str, body: impl FnOnce(&mut egui::Ui)) {
    egui::Grid::new(id).num_columns(2).spacing([10.0, 6.0]).min_col_width(110.0).show(ui, body);
}

fn axis_row(ui: &mut egui::Ui, t: &mut Track, axis: &mut Axis) {
    ui.label("Axis");
    ui.horizontal(|ui| {
        for a in Axis::ALL {
            t.r(ui.radio_value(axis, a, a.label()));
        }
    });
    ui.end_row();
}

fn spin(ui: &mut egui::Ui, t: &mut Track, label: &str, v: &mut f32, speed: f64) {
    ui.label(label);
    t.r(max_spinner(ui, v, speed, 0.0));
    ui.end_row();
}

fn limit_rows(ui: &mut egui::Ui, t: &mut Track, l: &mut Limits, unit: &str, speed: f64) {
    ui.label("");
    t.r(ui.checkbox(&mut l.on, "Limits (stops)"));
    ui.end_row();
    if l.on {
        spin(ui, t, &format!("Min {unit}"), &mut l.min, speed);
        spin(ui, t, &format!("Max {unit}"), &mut l.max, speed);
    }
}

/// Spinners for one joint; returns the edited copy when anything changed.
fn params(ui: &mut egui::Ui, kind: &JointKind) -> (JointKind, Track) {
    let mut k = kind.clone();
    let mut t = Track::default();
    grid(ui, "joint_params", |ui| match &mut k {
        JointKind::Rigid => {
            ui.label(dim("Welded: follows its parent 1:1."));
            ui.end_row();
        }
        JointKind::Piston => {
            ui.label(dim("Housing and rod aim at each other's mount."));
            ui.end_row();
        }
        JointKind::Revolute { axis, angle, limits } => {
            axis_row(ui, &mut t, axis);
            spin(ui, &mut t, "Angle (°)", angle, 0.5);
            limit_rows(ui, &mut t, limits, "(°)", 0.5);
        }
        JointKind::Ball { cone, twist, .. } => {
            ui.label("Cone Swing Limit");
            t.r(max_spinner(ui, cone, 0.5, 45.0).on_hover_text("Degrees; 180 = free"));
            ui.end_row();
            ui.label("Twist Limit");
            t.r(max_spinner(ui, twist, 0.5, 30.0).on_hover_text("Degrees; 180 = free"));
            ui.end_row();
            *cone = cone.clamp(0.0, 180.0);
            *twist = twist.clamp(0.0, 180.0);
        }
        JointKind::Slider { axis, travel, limits } => {
            axis_row(ui, &mut t, axis);
            spin(ui, &mut t, "Travel", travel, 0.01);
            limit_rows(ui, &mut t, limits, "Travel", 0.01);
        }
    });
    (k, t)
}

pub fn show(ui: &mut egui::Ui, ed: &mut Editor) {
    rollout(ui, "Assembly (Mech Joints)", true, |ui| {
        let on = matches!(ed.tool, Tool::Joint { .. });
        if toggle_button(ui, on, "🔗 Assemble Joint (J)").on_hover_text("Click a mount on the child part, then on the parent").clicked() {
            ed.toggle_joint_tool();
        }
        egui::ComboBox::from_id_salt("new_joint_type").width(content_w() - 12.0).selected_text(ed.joint_type.label()).show_ui(ui, |ui| {
            for k in JointType::ALL {
                ui.selectable_value(&mut ed.joint_type, k, k.label());
            }
        });
        ui.label(dim("Mounts: vertex, edge loop center or face / cap center."));
        let mut ik = !ed.scene.assembly.no_ik;
        if ui.checkbox(&mut ik, "IK Drag (W moves the chain)").on_hover_text("Off: W only moves a part along its own joint").changed() {
            ed.set_ik_drag(ik);
        }
        let Some(id) = ed.current() else { return };
        ui.separator();
        let links: Vec<(usize, String)> = ed
            .scene
            .assembly
            .joints
            .iter()
            .enumerate()
            .filter(|(_, j)| j.child == id || (j.parent == id && !j.kind.is_kinematic()))
            .map(|(i, j)| {
                let other = if j.child == id { j.parent } else { j.child };
                (i, ed.scene.objects.get(other).map_or("?".into(), |o| o.name.clone()))
            })
            .collect();
        let children = ed.scene.assembly.descendants(id).len();
        if links.is_empty() && children == 0 {
            ui.label(dim("This part has no joints."));
            return;
        }
        if let Some(i) = ed.scene.assembly.joint_of(id) {
            let parent = ed.scene.assembly.joints[i].parent;
            let kind = ed.scene.assembly.joints[i].kind.clone();
            ui.label(format!("Parent: {}", links.iter().find(|l| l.0 == i).map_or("?", |l| l.1.as_str())));
            let mut ty = kind.kind();
            egui::ComboBox::from_id_salt("joint_type").width(content_w() - 12.0).selected_text(ty.label()).show_ui(ui, |ui| {
                for k in JointType::ALL.into_iter().filter(|k| *k != JointType::Piston) {
                    ui.selectable_value(&mut ty, k, k.label());
                }
            });
            if ty != kind.kind() {
                ed.change_joint_type(ty);
            } else {
                let (k, t) = params(ui, &kind);
                if t.changed || t.started {
                    ed.edit_joint("Joint Parameters", t.started, |dst| *dst = k);
                }
            }
            if action(ui, "Select Parent", true, content_w()) {
                ed.scene.select_only(Some(parent));
            }
        }
        for (_, name) in links.iter().filter(|(i, _)| !ed.scene.assembly.joints[*i].kind.is_kinematic()) {
            ui.label(format!("Hydraulic Piston with {name}"));
        }
        if children > 0 {
            ui.label(dim(&format!("Carries {children} part(s).")));
        }
        motion_link(ui, ed, id);
        let mut c = ed.scene.assembly.contact_members.contains(&id);
        if ui.checkbox(&mut c, "Contact Member").on_hover_text("Pushes and is pushed by other members (toolbar Contact)").changed() {
            ed.set_contact_member(id, c);
        }
        let mut g = ed.scene.assembly.grounded.contains(&id);
        if ui.checkbox(&mut g, "Grounded (IK stops here)").changed() {
            ed.set_grounded(id, g);
        }
        ui.horizontal(|ui| {
            ui.label("Mirror Mech Limb");
            for a in [MirrorAxis::X, MirrorAxis::Y, MirrorAxis::Z] {
                let tip = "Copy this part and everything it carries across the plane through its parent";
                if ui.add(egui::Button::new(a.label()).min_size(egui::vec2(36.0, 22.0))).on_hover_text(tip).clicked() {
                    ed.mirror_limb(a);
                }
            }
        });
        if action(ui, "Break Joint", true, content_w()) {
            ed.break_joints();
        }
    });
}

/// Motion Link: the part's hinge / slider driven by another part's.
fn motion_link(ui: &mut egui::Ui, ed: &mut Editor, id: crate::scene::ObjId) {
    if ed.scene.joint_value(id).is_none() {
        return;
    }
    let link = ed.scene.assembly.links.iter().find(|l| l.driven == id).cloned();
    let name = |ed: &Editor, o| ed.scene.objects.get(o).map_or("?".to_string(), |o| o.name.clone());
    let drivers: Vec<_> = ed.scene.objects.keys().filter(|&o| o != id && ed.scene.joint_value(o).is_some()).collect();
    let mut pick = link.as_ref().map(|l| l.driver);
    ui.horizontal(|ui| {
        ui.label("Driven by");
        let shown = pick.map_or("None".to_string(), |d| name(ed, d));
        egui::ComboBox::from_id_salt("motion_driver").width(150.0).selected_text(shown).show_ui(ui, |ui| {
            ui.selectable_value(&mut pick, None, "None");
            for d in drivers {
                ui.selectable_value(&mut pick, Some(d), name(ed, d));
            }
        });
    });
    if pick != link.as_ref().map(|l| l.driver) {
        ed.set_driver(pick);
        return;
    }
    let Some(l) = link else { return };
    let mut t = Track::default();
    let mut ratio = l.ratio;
    grid(ui, "motion_link", |ui| {
        ui.label("Ratio");
        t.r(max_spinner(ui, &mut ratio, 0.01, 1.0).on_hover_text("Driven value per unit of driver (negative: opposite way)"));
        ui.end_row();
    });
    if t.changed || t.started {
        ed.edit_link_ratio(t.started, ratio);
    }
    if action(ui, "Auto-Ratio from Gear Teeth / Radius", true, content_w()) {
        ed.auto_link_ratio();
    }
}
