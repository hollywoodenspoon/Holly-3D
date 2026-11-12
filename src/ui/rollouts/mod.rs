//! Edit Poly rollouts for the Modify tab.

pub mod adv_mods;
pub mod buildings;
pub mod cad;
pub mod deformers;
pub mod edit_geometry;
pub mod edit_sub;
pub mod geogen;
pub mod ghosttown;
pub mod mech;
pub mod modifiers;
pub mod planet;
pub mod ruin;
pub mod smoke;
pub mod selection;
pub mod smoothing;
pub mod spline;
pub mod spline_tools;
pub mod stack_mods;
pub mod town_toggles;
pub mod worlds;

use crate::editor::Editor;
use crate::mesh::ops::PolyOp;
use crate::ui::command::CONTENT_W;
use crate::ui::widgets::{action, split_button};
use bevy_egui::egui;

pub(crate) const HALF: f32 = CONTENT_W / 2.0 - 4.0;

/// One split-caddy cell: main applies remembered params, `[□]` opens the caddy.
pub(crate) fn caddy_cell(ui: &mut egui::Ui, ed: &mut Editor, label: &str, default: PolyOp, enabled: bool) {
    let (main, caddy) = split_button(ui, label, enabled, HALF);
    if main {
        let op = ed.remembered(default.clone());
        ed.apply_op(op);
    } else if caddy {
        let op = ed.remembered(default);
        ed.open_caddy(op);
    }
}

/// One plain action cell.
pub(crate) fn op_cell(ui: &mut egui::Ui, ed: &mut Editor, label: &str, op: PolyOp, enabled: bool) {
    if action(ui, label, enabled, HALF) {
        ed.apply_op(op);
    }
}
