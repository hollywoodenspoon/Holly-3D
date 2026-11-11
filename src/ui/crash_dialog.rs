//! The crash recovery dialog (shown at startup after a crash) and the
//! Help -> View Crash Reports window.

use super::theme;
use crate::diagnostics::{hook, recovery};
use crate::editor::Editor;
use bevy_egui::egui::{self, RichText};
use std::path::{Path, PathBuf};

pub fn show(ctx: &egui::Context, ed: &mut Editor) {
    startup(ctx, ed);
    reports(ctx, ed);
}

fn dim(s: &str) -> RichText {
    RichText::new(s).color(theme::text_dim())
}

/// The first line of section 1 worth showing: the crash message.
fn headline(report: &str) -> String {
    let after = report.split("## 1.").nth(1).unwrap_or(report);
    let msg = after.split("```text").nth(1).and_then(|s| s.split("```").next()).unwrap_or("");
    let msg = msg.trim();
    let mut s: String = msg.chars().take(240).collect();
    if msg.chars().count() > 240 {
        s.push('…');
    }
    s
}

fn copy_button(ui: &mut egui::Ui, text: &str) {
    let b = ui.button("📋 Copy Crash Report for Claude").on_hover_text("Copies the whole Markdown report; paste it into Claude Code and ask it to fix the bug");
    if b.clicked() {
        ui.ctx().copy_text(text.to_string());
    }
}

/// Opens the emergency scene as an unsaved scene (Save As keeps it).
fn restore_button(ui: &mut egui::Ui, ed: &mut Editor, path: Option<&Path>) -> bool {
    let tip = "Opens the scene MiniMax saved when it crashed. It opens unsaved, so use Save As to keep it";
    let b = ui.add_enabled(path.is_some(), egui::Button::new("🔄 Restore Unsaved Scene (crash_recovery.mmax)")).on_hover_text(tip);
    let b = b.on_disabled_hover_text("No emergency scene was saved for this crash");
    if b.clicked()
        && let Some(p) = path
    {
        match ed.open_scene(p) {
            Ok(()) => {
                ed.scene_path = None;
                ed.status = "Restored the scene from the crash. It is unsaved: use File -> Save As to keep it".into();
                return true;
            }
            Err(e) => ed.status = format!("Could not restore {}: {e}", p.display()),
        }
    }
    false
}

fn folder_button(ui: &mut egui::Ui, ed: &mut Editor) {
    let dir = recovery::crash_dir();
    if ui.button("📂 Open Crash Reports Folder").on_hover_text(dir.display().to_string()).clicked()
        && let Err(e) = recovery::open_folder(&dir)
    {
        ed.status = format!("Could not open {}: {e}", dir.display());
    }
}

fn startup(ctx: &egui::Context, ed: &mut Editor) {
    let Some(p) = ed.iface.crash_pending.clone() else { return };
    let mut done = false;
    egui::Window::new("MiniMax closed unexpectedly").collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
        ui.set_max_width(520.0);
        ui.label("MiniMax crashed last time. A report of what happened, and your scene as it was, were saved.");
        ui.add_space(4.0);
        ui.label(RichText::new(headline(&p.text)).monospace().color(theme::amber()));
        ui.label(dim(&p.report.display().to_string()));
        ui.add_space(6.0);
        copy_button(ui, &p.text);
        done |= restore_button(ui, ed, p.recovery.as_deref());
        folder_button(ui, ed);
        let tip = "Close this dialog. The reports stay in the folder (Help -> View Crash Reports...)";
        if ui.button("🗑 Dismiss / Clear").on_hover_text(tip).clicked() {
            done = true;
        }
    });
    if done {
        recovery::dismiss(&recovery::crash_dir());
        ed.iface.crash_pending = None;
    }
}

/// The selected report and its text (kept in egui's temporary memory).
type Picked = Option<(PathBuf, String)>;

fn reports(ctx: &egui::Context, ed: &mut Editor) {
    if !ed.iface.crash_open {
        return;
    }
    let mut open = true;
    let id = egui::Id::new("crash_reports_pick");
    let mut pick: Picked = ctx.data(|d| d.get_temp(id)).flatten();
    let dir = recovery::crash_dir();
    egui::Window::new("Crash Reports").open(&mut open).default_size([620.0, 460.0]).show(ctx, |ui| {
        let list = recovery::list_reports(&dir);
        ui.horizontal_wrapped(|ui| {
            let tip = "Writes a sample report (and saves the scene) without crashing, to check the crash reporter works";
            if ui.button("⚠ Simulate Test Crash Report").on_hover_text(tip).clicked() {
                match hook::simulate() {
                    Some(b) => {
                        let text = std::fs::read_to_string(&b.report).unwrap_or_default();
                        ed.status = format!("Test crash report written to {}", b.report.display());
                        pick = Some((b.report, text));
                    }
                    None => ed.status = "Could not write the test crash report (see the terminal)".into(),
                }
            }
            folder_button(ui, ed);
            let del = ui.add_enabled(!list.is_empty(), egui::Button::new("🗑 Delete All Reports")).on_hover_text("Deletes every report and the emergency scene");
            if del.clicked() {
                let n = recovery::clear(&dir);
                ed.iface.crash_pending = None;
                ed.status = format!("Deleted {n} crash files");
                pick = None;
            }
        });
        ui.label(dim(&dir.display().to_string()));
        ui.separator();
        if list.is_empty() && pick.is_none() {
            ui.label("No crash reports. MiniMax writes one here automatically if it ever crashes.");
            return;
        }
        ui.horizontal_top(|ui| {
            egui::ScrollArea::vertical().id_salt("crash_list").max_width(220.0).max_height(360.0).show(ui, |ui| {
                for p in &list {
                    let name = p.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default();
                    let on = pick.as_ref().is_some_and(|(q, _)| q == p);
                    if ui.selectable_label(on, name).clicked() {
                        pick = Some((p.clone(), std::fs::read_to_string(p).unwrap_or_else(|e| format!("Could not read: {e}"))));
                    }
                }
            });
            ui.separator();
            ui.vertical(|ui| {
                let Some((_, text)) = &pick else {
                    ui.label("Pick a report on the left.");
                    return;
                };
                ui.horizontal_wrapped(|ui| {
                    copy_button(ui, text);
                    let rec = Some(dir.join(recovery::RECOVERY_FILE)).filter(|p| p.is_file());
                    restore_button(ui, ed, rec.as_deref());
                });
                egui::ScrollArea::both().id_salt("crash_text").max_height(320.0).show(ui, |ui| {
                    ui.label(RichText::new(text.as_str()).monospace().size(11.0));
                });
            });
        });
    });
    ctx.data_mut(|d| d.insert_temp(id, pick));
    ed.iface.crash_open = open;
}
