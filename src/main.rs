//! MiniMax — clean 3ds Max-style modeler (Phase 1 & 2).

use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use minimax::editor::Editor;
use minimax::primitives::PrimitiveKind;
use minimax::ui::UiPlugin;
use minimax::viewport::{lines, render_sync};

fn main() {
    let mut editor = Editor::default();
    if std::env::args().any(|a| a == "--demo") {
        editor.create_primitive(PrimitiveKind::Box, Vec3::ZERO);
        editor.create_primitive(PrimitiveKind::Sphere, Vec3::new(1.8, 0.4, 0.6));
        editor.create_primitive(PrimitiveKind::Torus, Vec3::new(-1.8, 0.2, 0.25));
        editor.history = Default::default();
    }
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window { title: "MiniMax — 3ds Max-style Modeler".into(), resolution: (1600, 920).into(), ..default() }),
        ..default()
    }))
    .add_plugins(EguiPlugin::default())
    .add_plugins(UiPlugin)
    .insert_resource(editor)
    .insert_resource(ClearColor(render_sync::VIEWPORT_BG))
    .init_resource::<render_sync::RenderMap>()
    .add_systems(Startup, render_sync::setup_scene)
    .add_systems(Update, (render_sync::sync_cameras, render_sync::sync_meshes));
    lines::register(&mut app);
    app.run();
}
