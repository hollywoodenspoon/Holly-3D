//! Ilex — clean modeler, painter and renderer.

use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use ilex::editor::Editor;
use ilex::primitives::PrimitiveKind;
use ilex::prefs::Preferences;
use ilex::ui::UiPlugin;
use ilex::ui::actions::keymap;
use ilex::viewport::{cloud_preview, light_icons, light_sync, lines, mat_sync, multi_sync, outline, render_sync, skybox, volume_sync};

fn main() {
    // Crash reporter first, so even a crash during start-up is recorded.
    ilex::diagnostics::hook::install();
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
    // `ilex scene.ilex` / `ilex old.mmax` / `ilex model.fbx`: open or import on start.
    if let Some(path) = std::env::args().skip(1).find(|a| !a.starts_with("--")) {
        let r = editor.open_any(std::path::Path::new(&path));
        editor.report(r);
        editor.history = Default::default();
    }
    // Preferences (theme, hotkeys, undo steps, render defaults) from ~/.config/ilex (or a pre-rename ~/.config/minimax).
    let path = Preferences::default_path();
    let prefs = Preferences::load_from(&path).unwrap_or_else(|e| {
        editor.status = format!("{e} (using defaults; fix or delete {})", path.display());
        Preferences::default()
    });
    keymap::set_current(keymap::from_prefs(&prefs.keys));
    editor.use_prefs(prefs);
    // `--help-shot <id>`: the scene and panels for one Help screenshot (tools/help_shots.sh).
    let args: Vec<String> = std::env::args().collect();
    if let Some(id) = args.iter().position(|a| a == "--help-shot").and_then(|i| args.get(i + 1)) {
        // Fresh scene, but keep the look from the prefs file (default in tools/help_shots.sh).
        let theme = editor.prefs.theme.clone();
        editor = Editor::default();
        editor.prefs.theme = theme;
        if !ilex::ui::help::shots::setup(&mut editor, id) {
            eprintln!("unknown help shot {id}; known: {}", ilex::ui::help::shots::IDS.join(", "));
            std::process::exit(2);
        }
    } else {
        // A crash last time: show the recovery dialog.
        editor.iface.crash_pending = ilex::diagnostics::recovery::detect_pending(&ilex::diagnostics::recovery::crash_dir());
    }
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Ilex".into(),
                    // WM_CLASS / Wayland app ID: matches `StartupWMClass` in
                    // packaging/ilex.desktop so the taskbar shows the icon.
                    name: Some("ilex".into()),
                    resolution: (1600, 920).into(),
                    ..default()
                }),
                // Pop-out dialog windows close with the main window.
                exit_condition: bevy::window::ExitCondition::OnPrimaryClosed,
                ..default()
            })
            .set(ilex::viewport::gpu::render_plugin()),
    )
    .add_plugins(EguiPlugin::default())
    .add_plugins(UiPlugin)
    .insert_resource(editor)
    .insert_resource(ClearColor(render_sync::VIEWPORT_BG))
    .init_resource::<render_sync::RenderMap>()
    .init_resource::<mat_sync::MatCache>()
    .init_resource::<multi_sync::MultiParts>()
    .add_systems(Startup, render_sync::setup_scene)
    .add_systems(Update, record_gpu_name.run_if(run_once))
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
    volume_sync::register(&mut app);
    cloud_preview::register(&mut app);
    skybox::register(&mut app);
    ilex::viewport::window_icon::register(&mut app);
    app.run();
    // Let cancelled render threads stop before the GPU driver unloads.
    drop(app);
    ilex::render::job::join_orphans();
    ilex::generators::smoke_sim::shutdown();
}

/// The GPU name for crash reports.
fn record_gpu_name(info: Option<Res<bevy::render::renderer::RenderAdapterInfo>>) {
    if let Some(i) = info {
        ilex::diagnostics::state::set_gpu(format!("{} ({:?}, {})", i.name, i.backend, i.driver));
    }
}
