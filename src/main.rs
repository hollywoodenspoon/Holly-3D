//! MiniMax — clean 3ds Max-style modeler, painter and renderer.

use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use minimax::editor::Editor;
use minimax::primitives::PrimitiveKind;
use minimax::prefs::Preferences;
use minimax::ui::UiPlugin;
use minimax::ui::actions::keymap;
use minimax::viewport::{light_icons, light_sync, lines, mat_sync, multi_sync, outline, render_sync, skybox};

fn main() {
    let mut editor = Editor::default();
    if std::env::args().any(|a| a == "--demo") {
        editor.create_primitive(PrimitiveKind::Box, Vec3::ZERO);
        editor.create_primitive(PrimitiveKind::Sphere, Vec3::new(1.8, 0.4, 0.6));
        editor.create_primitive(PrimitiveKind::Torus, Vec3::new(-1.8, 0.2, 0.25));
        if std::env::args().any(|a| a == "--demo-mats") {
            // Box: Wood, Sphere: Polished Gold, Torus: Cel-Shaded.
            let ids: Vec<_> = editor.scene.objects.keys().collect();
            for (id, slot) in ids.into_iter().zip([4, 2, 6]) {
                editor.scene.objects[id].material = Some(editor.scene.materials.slot(slot));
            }
        }
        editor.history = Default::default();
    }
    // `minimax scene.mmax` / `minimax model.fbx`: open or import on start.
    if let Some(path) = std::env::args().skip(1).find(|a| !a.starts_with("--")) {
        let r = editor.open_any(std::path::Path::new(&path));
        editor.report(r);
        editor.history = Default::default();
    }
    // Preferences (theme, hotkeys, undo steps, render defaults) from ~/.config/minimax.
    let path = Preferences::default_path();
    let prefs = Preferences::load_from(&path).unwrap_or_else(|e| {
        editor.status = format!("{e} (using defaults; fix or delete {})", path.display());
        Preferences::default()
    });
    keymap::set_current(keymap::from_prefs(&prefs.keys));
    editor.use_prefs(prefs);
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "MiniMax — 3ds Max-style Modeler".into(),
                    // WM_CLASS / Wayland app ID: matches `StartupWMClass` in
                    // packaging/minimax.desktop so the taskbar shows the icon.
                    name: Some("minimax".into()),
                    resolution: (1600, 920).into(),
                    ..default()
                }),
                ..default()
            })
            .set(minimax::viewport::gpu::render_plugin()),
    )
    .add_plugins(EguiPlugin::default())
    .add_plugins(UiPlugin)
    .insert_resource(editor)
    .insert_resource(ClearColor(render_sync::VIEWPORT_BG))
    .init_resource::<render_sync::RenderMap>()
    .init_resource::<mat_sync::MatCache>()
    .init_resource::<multi_sync::MultiParts>()
    .add_systems(Startup, render_sync::setup_scene)
    .add_systems(
        Update,
        (
            render_sync::sync_cameras,
            (mat_sync::sync_material_cache, render_sync::sync_meshes, multi_sync::sync_multi_parts).chain(),
            outline::sync_outlines,
            light_sync::sync_lights,
            light_icons::draw_light_icons,
        ),
    );
    lines::register(&mut app);
    outline::register(&mut app);
    light_sync::register(&mut app);
    skybox::register(&mut app);
    app.run();
}
