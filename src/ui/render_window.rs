//! Rendered Frame Window (F9): toolbar (Render, View, Region / Crop, Save,
//! Copy, Clone, R/G/B/A/Mono), progress and the image; plus cloned windows.

use crate::editor::Editor;
use crate::render::aov::Aov;
use crate::render::{ChannelView, FrameBuffer};
use crate::ui::render_setup::{SetupTab, view_combo};
use crate::ui::theme;
use crate::ui::widgets::{dim, toggle_button};
use bevy_egui::egui::{self, Color32, Pos2, Rect, RichText, Stroke, TextureHandle};
use std::collections::HashMap;

#[derive(Default)]
pub struct RfwState {
    tex: Option<(TextureHandle, u64, ChannelView, Aov)>,
    clones: HashMap<u32, (TextureHandle, ChannelView)>,
    clone_channels: HashMap<u32, ChannelView>,
    drag_from: Option<Pos2>,
    pub setup_tab: SetupTab,
}

fn color_image(fb: &FrameBuffer, ch: ChannelView) -> egui::ColorImage {
    egui::ColorImage::from_rgba_unmultiplied([fb.width as usize, fb.height as usize], &fb.view_rgba8(ch))
}

/// The image with the chosen AOV pass in place of the beauty render.
fn shown_image(ed: &Editor, fb: &FrameBuffer, ch: ChannelView) -> egui::ColorImage {
    let Some(a) = ed.rfw.aovs.as_ref().filter(|_| ed.rfw.aov != Aov::Beauty) else { return color_image(fb, ch) };
    let mut f = fb.clone();
    let v = a.view(ed.rfw.aov, &ed.render.post.tone);
    let [x0, y0, x1, _] = a.region;
    f.put_block(x0, y0, x1 - x0, &v, &v);
    color_image(&f, ch)
}

fn aov_picker(ui: &mut egui::Ui, ed: &mut Editor) {
    let has = ed.rfw.aovs.is_some();
    ui.add_enabled_ui(has, |ui| {
        egui::ComboBox::from_id_salt("rfw_aov").width(170.0).selected_text(ed.rfw.aov.label()).show_ui(ui, |ui| {
            for a in Aov::ALL {
                ui.selectable_value(&mut ed.rfw.aov, a, a.label());
            }
        });
    })
    .response
    .on_hover_text(if has { "AOV render pass shown in the window" } else { "AOV passes come with CPU path-traced renders" });
}

fn channel_buttons(ui: &mut egui::Ui, ch: &mut ChannelView) {
    for (on, label) in [(&mut ch.r, "R"), (&mut ch.g, "G"), (&mut ch.b, "B"), (&mut ch.alpha, "A"), (&mut ch.mono, "Mono")] {
        if toggle_button(ui, *on, label).clicked() {
            *on = !*on;
        }
    }
}

/// Image fitted to the available space; returns its screen rect + response.
fn image(ui: &mut egui::Ui, tex: &TextureHandle, fb: &FrameBuffer, sense: egui::Sense) -> (Rect, egui::Response) {
    let avail = ui.available_size().max(egui::vec2(64.0, 64.0));
    let size = egui::vec2(fb.width as f32, fb.height as f32);
    let k = (avail.x / size.x).min(avail.y.max(240.0) / size.y).min(1.0);
    let r = ui.add(egui::Image::new((tex.id(), size * k)).sense(sense));
    (r.rect, r)
}

fn to_px(rect: Rect, fb: &FrameBuffer, p: Pos2) -> [u32; 2] {
    let u = ((p.x - rect.min.x) / rect.width()).clamp(0.0, 1.0);
    let v = ((p.y - rect.min.y) / rect.height()).clamp(0.0, 1.0);
    [(u * fb.width as f32) as u32, (v * fb.height as f32) as u32]
}

fn toolbar(ui: &mut egui::Ui, ed: &mut Editor) {
    ui.horizontal(|ui| {
        if ed.is_rendering() {
            if ui.button(RichText::new("■ Cancel").strong()).clicked() {
                ed.cancel_render();
            }
        } else if ui.button(RichText::new("▶ Render").strong()).clicked() {
            ed.start_render();
        }
        view_combo(ui, ed, "rfw_view", 150.0);
        if toggle_button(ui, ed.rfw.region_mode, "▣ Region").on_hover_text("Drag on the image to set the region / crop").clicked() {
            ed.rfw.region_mode = !ed.rfw.region_mode;
        }
        if ui.button("⚙ Setup").on_hover_text("Render Setup (F10)").clicked() {
            ed.tab = crate::editor::CommandTab::Render;
        }
        if toggle_button(ui, ed.rfw.sheet.open, "🎞 Turntable / Sprites").on_hover_text("Render Turntable / Sprite Sheet...").clicked() {
            ed.rfw.sheet.open = !ed.rfw.sheet.open;
        }
    });
    ui.horizontal(|ui| {
        if ed.rfw.save_path.is_empty() {
            ed.rfw.save_path = "render.png".into();
        }
        ui.add(egui::TextEdit::singleline(&mut ed.rfw.save_path).desired_width(140.0)).on_hover_text(".png, .jpg or .exr");
        let has = ed.rfw.fb.is_some();
        if ui.add_enabled(has, egui::Button::new("💾 Save Image")).clicked() {
            let p = ed.rfw.save_path.clone();
            ed.save_render(&p);
        }
        if ui.add_enabled(has, egui::Button::new("📋 Copy")).on_hover_text("Copy to Clipboard").clicked()
            && let Some(fb) = &ed.rfw.fb
        {
            ui.ctx().copy_image(color_image(fb, ChannelView::default()));
            ed.rfw.message = "Image copied to the clipboard".into();
        }
        if ui.add_enabled(has, egui::Button::new("🗐 Clone")).on_hover_text("Clone Render Window").clicked() {
            ed.clone_render_window();
        }
        ui.separator();
        channel_buttons(ui, &mut ed.rfw.channels);
        aov_picker(ui, ed);
        ui.separator();
        if toggle_button(ui, ed.rfw.grading_open, "🎨 Grading").on_hover_text("Color Grading & Camera RAW sidebar").clicked() {
            ed.rfw.grading_open = !ed.rfw.grading_open;
        }
        if toggle_button(ui, ed.rfw.lightmix_open, "💡 Luxon LightMix").on_hover_text("Relight the render per light group without re-rendering").clicked() {
            ed.rfw.lightmix_open = !ed.rfw.lightmix_open;
        }
    });
    // Live tonemapping: re-composes the stored layers, no re-render.
    let live = ed.rfw.fb.as_ref().is_some_and(|f| f.can_retone());
    ui.add_enabled_ui(live, |ui| {
        if crate::ui::render_fx::tone_row(ui, &mut ed.render.post.tone) {
            ed.retone_render();
        }
    });
}

fn status(ui: &mut egui::Ui, ed: &Editor) {
    if let Some(j) = &ed.rfw.job {
        let p = j.progress();
        ui.add(egui::ProgressBar::new(p).desired_width(ui.available_width()).text(format!(
            "{:.0}% · {:.1}s{}{}",
            p * 100.0,
            j.started.elapsed().as_secs_f32(),
            if j.pass_label.is_empty() { "" } else { " · " },
            j.pass_label
        )));
        ui.ctx().request_repaint();
    } else if !ed.rfw.message.is_empty() {
        ui.label(RichText::new(&ed.rfw.message).color(theme::amber()));
    }
}

fn main_window(ctx: &egui::Context, ed: &mut Editor, st: &mut RfwState) {
    let mut open = ed.rfw.rfw_open;
    egui::Window::new(RichText::new("Veach Rendered Frame Window").strong())
        .id(egui::Id::new("rfw"))
        .open(&mut open)
        .default_pos([300.0, 70.0])
        .default_size([840.0, 700.0])
        .show(ctx, |ui| {
            toolbar(ui, ed);
            status(ui, ed);
            ui.separator();
            if ed.rfw.grading_open {
                crate::ui::render_grading::sidebar(ui, ed);
            }
            if ed.rfw.lightmix_open {
                crate::ui::render_lightmix::sidebar(ui, ed);
            }
            let Some(fb) = &ed.rfw.fb else {
                ui.label(dim("Press Render (F9) to render the active viewport."));
                return;
            };
            let ch = ed.rfw.channels;
            let aov = ed.rfw.aov;
            let stale = st.tex.as_ref().is_none_or(|(_, rev, c, a)| *rev != fb.rev || *c != ch || *a != aov);
            if stale {
                let img = shown_image(ed, fb, ch);
                match &mut st.tex {
                    Some((t, rev, c, a)) => {
                        t.set(img, egui::TextureOptions::LINEAR);
                        (*rev, *c, *a) = (fb.rev, ch, aov);
                    }
                    None => st.tex = Some((ctx.load_texture("rfw_image", img, egui::TextureOptions::LINEAR), fb.rev, ch, aov)),
                }
            }
            let Some((tex, ..)) = &st.tex else { return };
            let sense = if ed.rfw.region_mode { egui::Sense::drag() } else { egui::Sense::hover() };
            let (rect, r) = image(ui, tex, fb, sense);
            if ed.rfw.region_mode {
                if r.drag_started() {
                    st.drag_from = r.interact_pointer_pos();
                }
                if let (Some(a), Some(b)) = (st.drag_from, r.interact_pointer_pos())
                    && r.dragged()
                {
                    let ([x0, y0], [x1, y1]) = (to_px(rect, fb, a), to_px(rect, fb, b));
                    ed.rfw.region = Some([x0.min(x1), y0.min(y1), x0.max(x1).max(x0.min(x1) + 1), y0.max(y1).max(y0.min(y1) + 1)]);
                }
                let reg = crate::render::job::clamp_region(ed.rfw.region, fb.width, fb.height);
                let k = rect.width() / fb.width as f32;
                let rr = Rect::from_min_max(
                    rect.min + egui::vec2(reg[0] as f32, reg[1] as f32) * k,
                    rect.min + egui::vec2(reg[2] as f32, reg[3] as f32) * k,
                );
                ui.painter().rect_stroke(rr, 0.0, Stroke::new(1.5, theme::amber()), egui::StrokeKind::Inside);
            }
            if let Some(p) = r.hover_pos() {
                let [x, y] = to_px(rect, fb, p);
                if let Some(c) = fb.display.get((y.min(fb.height - 1) * fb.width + x.min(fb.width - 1)) as usize) {
                    let t = format!("{x}, {y}   RGBA {:.3} {:.3} {:.3} {:.3}", c.x, c.y, c.z, c.w);
                    ui.painter().text(
                        rect.left_bottom() + egui::vec2(4.0, -4.0),
                        egui::Align2::LEFT_BOTTOM,
                        t,
                        egui::FontId::monospace(11.0),
                        Color32::WHITE,
                    );
                }
            }
        });
    if !open {
        ed.rfw.rfw_open = false;
    }
}

fn clone_windows(ctx: &egui::Context, ed: &mut Editor, st: &mut RfwState) {
    for c in &mut ed.rfw.clones {
        let ch = st.clone_channels.entry(c.id).or_default();
        let mut open = c.open;
        egui::Window::new(RichText::new(format!("Veach Rendered Frame Window — {}", c.label)).strong())
            .id(egui::Id::new(("rfw_clone", c.id)))
            .open(&mut open)
            .default_pos([340.0 + c.id as f32 * 24.0, 100.0 + c.id as f32 * 24.0])
            .default_size([600.0, 480.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| channel_buttons(ui, ch));
                let stale = st.clones.get(&c.id).is_none_or(|(_, old)| old != ch);
                if stale {
                    let tex = ctx.load_texture(format!("rfw_clone_{}", c.id), color_image(&c.fb, *ch), egui::TextureOptions::LINEAR);
                    st.clones.insert(c.id, (tex, *ch));
                }
                if let Some((tex, _)) = st.clones.get(&c.id) {
                    image(ui, tex, &c.fb, egui::Sense::hover());
                }
            });
        c.open = open;
    }
    let before = ed.rfw.clones.len();
    ed.rfw.clones.retain(|c| c.open);
    if ed.rfw.clones.len() != before {
        let ids: Vec<u32> = ed.rfw.clones.iter().map(|c| c.id).collect();
        st.clones.retain(|k, _| ids.contains(k));
        st.clone_channels.retain(|k, _| ids.contains(k));
    }
}

pub fn show(ctx: &egui::Context, ed: &mut Editor, st: &mut RfwState) {
    ed.poll_render();
    ed.poll_sheet();
    crate::ui::render_sheet::window(ctx, ed);
    if ed.is_rendering() || ed.rfw.sheet.job.is_some() {
        ctx.request_repaint();
    }
    if ed.rfw.rfw_open {
        main_window(ctx, ed, st);
    }
    clone_windows(ctx, ed, st);
}
