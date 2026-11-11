//! egui front-end: fixed 5-zone 3ds Max layout + floating windows.

pub mod action_ui;
pub mod actions;
pub mod align_dialog;
pub mod array_dialog;
pub mod caddy;
pub mod clone_dialog;
pub mod command;
pub mod env_dialog;
pub mod explorer;
pub mod file_menu;
pub mod fonts;
pub mod help;
pub mod hotkeys;
pub mod material_editor;
pub mod menubar;
pub mod mirror_dialog;
pub mod paintbox;
pub mod prefs;
pub mod quad_menu;
pub mod render_fx;
pub mod render_setup;
pub mod render_window;
pub mod ribbon;
pub mod rollouts;
pub mod sky_dialog;
pub mod snaps;
pub mod status_bar;
pub mod theme;
pub mod tool_windows;
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
    /// Theme settings last applied to egui.
    themed: Option<(crate::prefs::ThemePrefs, crate::prefs::ViewportPrefs)>,
    pub prefs: prefs::PrefsUi,
    pub viewport: viewport_ui::ViewportUiState,
    pub modifier_list_open: bool,
    pub uv: uv_editor::UvEditorState,
    pub mat: material_editor::MatEditorUi,
    pub rfw: render_window::RfwState,
    pub help: help::HelpUi,
}

fn ui_system(mut contexts: EguiContexts, mut ed: ResMut<Editor>, mut st: ResMut<UiState>) -> Result {
    let ctx = contexts.ctx_mut()?.clone();
    let want = (ed.prefs.theme.clone(), ed.prefs.viewport.clone());
    if fonts::installed(&ctx) != Some(want.0.font) {
        // egui loads new fonts at the start of the next frame; the theme (which
        // names the bold face) is applied then.
        fonts::install(&ctx, want.0.font);
        st.themed = None;
        ctx.request_repaint();
    } else if st.themed.as_ref() != Some(&want) {
        let first = st.themed.is_none();
        theme::apply(&ctx, &want.0, &want.1);
        st.themed = Some(want);
        // egui's debug options only exist in debug builds.
        #[cfg(debug_assertions)]
        if first && std::env::var("MINIMAX_DEBUG_UI").is_ok() {
            ctx.global_style_mut(|s| s.debug.show_expand_width = true);
        }
    }
    let ed = &mut *ed;
    hotkeys::handle(&ctx, ed);
    let mut root =
        Ui::new(ctx.clone(), "minimax_root".into(), UiBuilder::new().layer_id(LayerId::background()).max_rect(ctx.viewport_rect()));
    let screen = ctx.viewport_rect();
    menubar::show(&mut root, ed);
    let mut top = toolbar::show(&mut root, ed).max.y;
    if ed.iface.ribbon {
        top = ribbon::show(&mut root, ed).max.y;
    }
    let bottom = status_bar::show(&mut root, ed).min.y;
    // Side panels are fixed-width (3ds Max layout), so the viewport edges are
    // derived from those widths rather than from content-dependent rects.
    let left = if ed.explorer_visible { explorer::show(&mut root, ed).min.x + explorer::WIDTH } else { screen.min.x };
    let right = command::show(&mut root, ed, &mut st).max.x.min(screen.max.x) - command::PANEL_W;
    let area = egui::Rect::from_min_max(egui::pos2(left, top), egui::pos2(right, bottom));
    viewport_ui::show(&mut root, ed, &mut st.viewport, area);
    caddy::show(&ctx, ed);
    clone_dialog::show(&ctx, ed);
    array_dialog::show(&ctx, ed);
    mirror_dialog::show(&ctx, ed);
    align_dialog::show(&ctx, ed);
    quad_menu::show(&ctx, ed);
    tool_windows::show(&ctx, ed);
    prefs::show(&ctx, ed, &mut st.prefs);
    ed.autosave_tick(ctx.input(|i| i.time));
    ed.poll_downloads();
    uv_editor::show(&ctx, ed, &mut st.uv);
    material_editor::show(&ctx, ed, &mut st.mat);
    material_editor::library::show(&ctx, ed, &mut st.mat.library);
    env_dialog::show(&ctx, ed);
    paintbox::show(&ctx, ed);
    render_setup::show(&ctx, ed, &mut st.rfw.setup_tab);
    render_window::show(&ctx, ed, &mut st.rfw);
    help::show(&ctx, ed, &mut st.help);
    ed.update_caddy_preview();
    ed.scene.solve_assembly();
    ed.scene.update_hoses();
    Ok(())
}
