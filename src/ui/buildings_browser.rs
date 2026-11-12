//! Buildings & Props Browser (Tools -> Buildings & Props Browser..., the
//! Create tab and the Create menu): a visual grid of all 44 standalone
//! building / prop types with flat-shaded 3/4 thumbnails, plus the Create
//! tab's "Geometry — Buildings & Props" rollout.

use crate::editor::{Editor, Tool};
use crate::generators::WorldKind;
use crate::generators::buildings::{BuildingKind, BuildingParams, Category};
use crate::materials::ghost_pack::swatch;
use crate::ui::command::CONTENT_W;
use crate::ui::widgets::{dim, rollout};
use bevy::math::{Vec2, Vec3};
use bevy_egui::egui::{self, Color32, Pos2, RichText, Sense, Stroke};
use std::sync::OnceLock;

const TILE: f32 = 132.0;

/// Polygons of a thumbnail in 0..1 tile space (y down), back to front.
type Thumb = Vec<(Vec<Vec2>, Color32)>;

static THUMBS: [OnceLock<Thumb>; 44] = [const { OnceLock::new() }; 44];

/// Browser UI state kept in egui memory: category filter and search text.
#[derive(Clone, Default)]
struct State {
    cat: Option<usize>,
    search: String,
}

pub fn show(ctx: &egui::Context, ed: &mut Editor) {
    if !ed.buildings_browser_open {
        return;
    }
    let mut open = true;
    egui::Window::new("Buildings & Props Browser").open(&mut open).default_size([640.0, 520.0]).show(ctx, |ui| body(ui, ed));
    ed.buildings_browser_open &= open;
}

fn body(ui: &mut egui::Ui, ed: &mut Editor) {
    let id = egui::Id::new("buildings_browser_state");
    let mut st: State = ui.data_mut(|d| d.get_temp(id).unwrap_or_default());
    ui.horizontal_wrapped(|ui| {
        if ui.selectable_label(st.cat.is_none(), "All (44)").clicked() {
            st.cat = None;
        }
        for (i, c) in Category::ALL.into_iter().enumerate() {
            let label = format!("{} ({})", c.label(), c.kinds().count());
            if ui.selectable_label(st.cat == Some(i), label).clicked() {
                st.cat = Some(i);
            }
        }
    });
    ui.horizontal(|ui| {
        ui.label("Search:");
        ui.add(egui::TextEdit::singleline(&mut st.search).desired_width(200.0).hint_text("tavern, bridge, lamp..."));
        ui.label(dim("Click: arm the drag tool. Right-click: place at origin."));
    });
    ui.separator();
    let needle = st.search.to_lowercase();
    let kinds: Vec<BuildingKind> = BuildingKind::ALL
        .into_iter()
        .filter(|k| st.cat.is_none_or(|i| k.category() == Category::ALL[i]))
        .filter(|k| needle.is_empty() || k.label().to_lowercase().contains(&needle))
        .collect();
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        let cols = ((ui.available_width() / (TILE + 6.0)) as usize).max(1);
        egui::Grid::new("buildings_browser_grid").spacing([6.0, 6.0]).show(ui, |ui| {
            for (i, k) in kinds.iter().enumerate() {
                tile(ui, ed, *k);
                if i % cols == cols - 1 {
                    ui.end_row();
                }
            }
        });
        if kinds.is_empty() {
            ui.label(dim("Nothing matches the search."));
        }
    });
    ui.data_mut(|d| d.insert_temp(id, st));
}

/// One thumbnail tile; returns nothing, acts on the editor directly.
fn tile(ui: &mut egui::Ui, ed: &mut Editor, k: BuildingKind) {
    let kind = WorldKind::Building(k);
    let active = ed.tool == Tool::CreateWorld(kind);
    let (rect, r) = ui.allocate_exact_size(egui::vec2(TILE, TILE + 18.0), Sense::click());
    let v = ui.visuals();
    let fill = if r.hovered() { v.widgets.hovered.bg_fill } else { v.extreme_bg_color };
    let border = if active { Color32::from_rgb(0xE5, 0x9C, 0x24) } else { v.widgets.noninteractive.bg_stroke.color };
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 4.0, fill);
    p.rect_stroke(rect, 4.0, Stroke::new(if active { 2.0 } else { 1.0 }, border), egui::StrokeKind::Inside);
    let img = egui::Rect::from_min_size(rect.min + egui::vec2(8.0, 6.0), egui::vec2(TILE - 16.0, TILE - 16.0));
    for (pts, c) in thumb(k) {
        let pts: Vec<Pos2> = pts.iter().map(|q| img.min + egui::vec2(q.x, q.y) * img.size()).collect();
        p.add(egui::Shape::convex_polygon(pts, *c, Stroke::NONE));
    }
    p.text(
        egui::pos2(rect.center().x, rect.max.y - 10.0),
        egui::Align2::CENTER_CENTER,
        k.label(),
        egui::FontId::proportional(11.0),
        v.text_color(),
    );
    let (w, l, f) = k.defaults();
    let r = r.on_hover_text(format!("{}\n{w} x {l} m{}", k.label(), if k.has_floors() { format!(", {f} floors") } else { String::new() }));
    if r.clicked() {
        ed.tool = if active { Tool::None } else { Tool::CreateWorld(kind) };
        ed.status = format!("{}: drag in a viewport to set the footprint", k.label());
    }
    if r.secondary_clicked() {
        ed.create_world(kind, Vec3::ZERO);
    }
}

/// Flat-shaded 3/4 view of a type's default build (built once, cached).
fn thumb(k: BuildingKind) -> &'static Thumb {
    THUMBS[k as usize].get_or_init(|| {
        let m = BuildingParams::of(k).build();
        let (yaw, pitch) = (35f32.to_radians(), 25f32.to_radians());
        let view = |p: Vec3| {
            let x = p.x * yaw.cos() + p.y * yaw.sin();
            let y = -p.x * yaw.sin() + p.y * yaw.cos();
            (Vec2::new(x, -(p.z * pitch.cos() + y * pitch.sin())), y * pitch.cos() - p.z * pitch.sin())
        };
        let eye = Vec3::new(yaw.sin() * pitch.cos(), -yaw.cos() * pitch.cos(), pitch.sin());
        let sun = Vec3::new(0.3, -0.75, 0.6).normalize();
        let mut polys: Vec<(f32, Vec<Vec2>, Color32)> = vec![];
        for (pid, poly) in &m.polygons {
            let n = m.poly_normal(pid);
            if n.dot(eye) <= 0.0 {
                continue;
            }
            let pts: Vec<(Vec2, f32)> = poly.vertex_ids().map(|v| view(m.pos(v))).collect();
            let depth = pts.iter().map(|q| q.1).sum::<f32>() / pts.len() as f32;
            let shade = 0.45 + 0.55 * n.dot(sun).max(0.0);
            let [r, g, b] = swatch(poly.material_id).map(|c| ((c * shade).clamp(0.0, 1.0) * 255.0) as u8);
            polys.push((depth, pts.into_iter().map(|q| q.0).collect(), Color32::from_rgb(r, g, b)));
        }
        polys.sort_by(|a, b| b.0.total_cmp(&a.0));
        let (lo, hi) = polys.iter().flat_map(|p| p.1.iter()).fold((Vec2::MAX, Vec2::MIN), |(a, b), q| (a.min(*q), b.max(*q)));
        let size = (hi - lo).max_element().max(1e-3);
        let pad = (Vec2::splat(size) - (hi - lo)) * 0.5;
        polys.into_iter().map(|(_, pts, c)| (pts.into_iter().map(|q| (q - lo + pad) / size).collect(), c)).collect()
    })
}

/// Create tab: one collapsible section per category with 2-column grids.
pub fn create_rollout(ui: &mut egui::Ui, ed: &mut Editor) {
    rollout(ui, "Geometry — Buildings & Props", true, |ui| {
        if ui
            .add_sized([CONTENT_W, 26.0], egui::Button::new("Open Buildings & Props Browser...").selected(ed.buildings_browser_open))
            .clicked()
        {
            ed.buildings_browser_open ^= true;
        }
        for c in Category::ALL {
            egui::CollapsingHeader::new(RichText::new(c.label())).id_salt(("create_bld", c.label())).show(ui, |ui| {
                egui::Grid::new(("create_bld_grid", c.label())).num_columns(2).show(ui, |ui| {
                    for (i, k) in c.kinds().enumerate() {
                        let kind = WorldKind::Building(k);
                        let active = ed.tool == Tool::CreateWorld(kind);
                        let r = ui
                            .add_sized([CONTENT_W / 2.0 - 12.0, 24.0], egui::Button::new(k.label()).selected(active))
                            .on_hover_text(c.label());
                        if r.clicked() {
                            ed.tool = if active { Tool::None } else { Tool::CreateWorld(kind) };
                        }
                        if r.secondary_clicked() {
                            ed.create_world(kind, Vec3::ZERO);
                        }
                        if i % 2 == 1 {
                            ui.end_row();
                        }
                    }
                });
            });
        }
        ui.label(dim("Each piece stands alone with the 28-ID GhostTown pack. Drag sets Width x Length; right-click places at the origin."));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_type_has_a_thumbnail_in_the_unit_tile() {
        for k in BuildingKind::ALL {
            let t = thumb(k);
            assert!(t.len() >= 3, "{k:?}");
            assert!(t.iter().flat_map(|p| p.0.iter()).all(|q| q.x >= -1e-3 && q.x <= 1.001 && q.y >= -1e-3 && q.y <= 1.001), "{k:?}");
        }
    }
}
