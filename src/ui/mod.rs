//! egui front-end: fixed 5-zone 3ds Max layout + floating windows.

pub mod caddy;
pub mod clone_dialog;
pub mod command;
pub mod explorer;
pub mod hotkeys;
pub mod quad_menu;
pub mod rollouts;
pub mod status_bar;
pub mod theme;
pub mod toolbar;
pub mod uv_editor;
pub mod viewport_ui;
pub mod widgets;

use crate::editor::Editor;
use bevy::prelude::*;
use bevy_egui::egui::{self, LayerId, Ui, UiBuilder};
use bevy_egui::{EguiContexts, EguiPrimaryContextPass};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiState>().add_systems(EguiPrimaryContextPass, ui_system);
    }
}

/// Transient UI-only state (not part of undo).
#[derive(Resource, Default)]
pub struct UiState {
    themed: bool,
    pub viewport: viewport_ui::ViewportUiState,
    pub modifier_list_open: bool,
    pub uv: uv_editor::UvEditorState,
}

fn ui_system(mut contexts: EguiContexts, mut ed: ResMut<Editor>, mut st: ResMut<UiState>) -> Result {
    let ctx = contexts.ctx_mut()?.clone();
    if !st.themed {
        theme::apply(&ctx);
        if std::env::var("MINIMAX_DEBUG_UI").is_ok() {
            ctx.global_style_mut(|s| s.debug.show_expand_width = true);
        }
        st.themed = true;
    }
    let ed = &mut *ed;
    hotkeys::handle(&ctx, ed);
    let mut root =
        Ui::new(ctx.clone(), "minimax_root".into(), UiBuilder::new().layer_id(LayerId::background()).max_rect(ctx.viewport_rect()));
    let screen = ctx.viewport_rect();
    let top = toolbar::show(&mut root, ed).max.y;
    let bottom = status_bar::show(&mut root, ed).min.y;
    // Side panels are fixed-width (3ds Max layout), so the viewport edges are
    // derived from those widths rather than from content-dependent rects.
    let left = if ed.explorer_visible { explorer::show(&mut root, ed).min.x + explorer::WIDTH } else { screen.min.x };
    let right = command::show(&mut root, ed, &mut st).max.x.min(screen.max.x) - command::PANEL_W;
    let area = egui::Rect::from_min_max(egui::pos2(left, top), egui::pos2(right, bottom));
    viewport_ui::show(&mut root, ed, &mut st.viewport, area);
    caddy::show(&ctx, ed);
    clone_dialog::show(&ctx, ed);
    quad_menu::show(&ctx, ed);
    uv_editor::show(&ctx, ed, &mut st.uv);
    ed.update_caddy_preview();
    Ok(())
}
