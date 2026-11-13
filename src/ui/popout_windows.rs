//! Bevy side of pop-out windows: one OS window + 2D camera + egui context
//! per popped dialog, spawned and despawned to match what the UI wants.
//! See `popout.rs` for how dialogs draw into them.

use super::popout::{self, Dock, Host};
use super::UiState;
use crate::editor::Editor;
use bevy::camera::RenderTarget;
use bevy::camera::visibility::RenderLayers;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;
use bevy::window::WindowRef;
use bevy_egui::{EguiContext, EguiSchedule};
use std::collections::HashMap;

/// The egui pass of one pop-out window.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PopPass(pub Dock);

/// Marks the camera (and egui context) of a pop-out window.
#[derive(Component)]
pub struct PopCam(pub Dock);

/// Open pop-out windows: dock -> (window, camera).
#[derive(Resource, Default)]
pub struct PopWindows(HashMap<Dock, (Entity, Entity)>);

pub fn register(app: &mut App) {
    app.init_resource::<PopWindows>().add_systems(Update, sync_windows);
    for d in Dock::ALL {
        app.add_systems(
            PopPass(d),
            move |mut q: Query<(&mut EguiContext, &PopCam)>, mut ed: ResMut<Editor>, mut st: ResMut<UiState>| {
                pass(d, &mut q, &mut ed, &mut st);
            },
        );
    }
}

fn pass(d: Dock, q: &mut Query<(&mut EguiContext, &PopCam)>, ed: &mut Editor, st: &mut UiState) {
    let Some((mut c, _)) = q.iter_mut().find(|(_, p)| p.0 == d) else { return };
    let ctx = c.get_mut().clone();
    let mut themed = st.pop_themed.remove(&d);
    super::ensure_theme(&ctx, ed, &mut themed);
    if let Some(t) = themed {
        st.pop_themed.insert(d, t);
    }
    popout::set_host(Some(Host::Pop(d)));
    if popout::arriving(d, Host::Pop(d)) {
        super::clear_dock_textures(st, d);
    }
    super::hotkeys::handle(&ctx, ed);
    super::draw_dock(&ctx, ed, st, d);
    popout::set_host(None);
}

fn sync_windows(mut commands: Commands, st: Res<UiState>, mut open: ResMut<PopWindows>, windows: Query<(), With<Window>>) {
    // Closed by the user: dock the dialog back into the main window.
    let closed: Vec<Dock> = open.0.iter().filter(|(_, (w, _))| windows.get(*w).is_err()).map(|(d, _)| *d).collect();
    for d in &closed {
        if let Some((_, cam)) = open.0.remove(d) {
            commands.entity(cam).despawn();
        }
        popout::dock_back(*d);
        info!("{} docked back into the main window", d.title());
    }
    let gone: Vec<Dock> = open.0.keys().filter(|d| !st.pop_want.contains(d)).copied().collect();
    for d in gone {
        if let Some((win, cam)) = open.0.remove(&d) {
            commands.entity(cam).despawn();
            commands.entity(win).despawn();
        }
    }
    for (i, d) in Dock::ALL.into_iter().enumerate() {
        if !st.pop_want.contains(&d) || open.0.contains_key(&d) || closed.contains(&d) {
            continue;
        }
        let win = commands
            .spawn(Window {
                title: format!("Ilex — {}", d.title()),
                name: Some(d.app_id()),
                resolution: d.size().into(),
                ..default()
            })
            .id();
        let [r, g, b, _] = super::theme::panel().to_array();
        let cam = commands
            .spawn((
                Camera2d,
                Camera { order: 20 + i as isize, clear_color: ClearColorConfig::Custom(Color::srgb_u8(r, g, b)), ..default() },
                RenderTarget::Window(WindowRef::Entity(win)),
                RenderLayers::none(),
                EguiSchedule::new(PopPass(d)),
                PopCam(d),
            ))
            .id();
        open.0.insert(d, (win, cam));
    }
}
