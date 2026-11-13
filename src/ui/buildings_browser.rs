//! Prefab Library (Demiurge tab, Tools menu, header `»`): a visual grid of
//! every procedural prefab (buildings, bridges, street props) with
//! flat-shaded 3/4 thumbnails, categories, search and favourites, and the
//! place-mode bar (rotation, scale, Vary Seed, Drop on Surfaces). Plus the
//! Create tab's "Buildings & Props" rollout.

use crate::editor::{Editor, Tool};
use crate::generators::WorldKind;
use crate::generators::buildings::{BuildingKind, BuildingParams, Category};
use crate::materials::ghost_pack::swatch;
use crate::ui::properties::content_w;
use crate::ui::widgets::{dim, max_spinner, rollout};
use bevy::math::{Vec2, Vec3};
use bevy_egui::egui::{self, Color32, Pos2, RichText, Sense, Stroke};
use std::sync::{LazyLock, OnceLock};

const TILE: f32 = 132.0;

/// Polygons of a thumbnail in 0..1 tile space (y down), back to front.
type Thumb = Vec<(Vec<Vec2>, Color32)>;

static THUMBS: LazyLock<Vec<OnceLock<Thumb>>> = LazyLock::new(|| BuildingKind::ALL.iter().map(|_| OnceLock::new()).collect());

/// Which tiles show.
#[derive(Clone, Copy, Default, PartialEq)]
enum Filter {
    #[default]
    All,
    Favourites,
    Cat(Category),
}

/// Browser UI state kept in egui memory.
#[derive(Clone, Default)]
struct State {
    filter: Filter,
    search: String,
    /// Tile click arms the drag-to-size tool instead of place mode.
    drag: bool,
}

pub fn show(ctx: &egui::Context, ed: &mut Editor) {
    if !ed.buildings_browser_open {
        return;
    }
    let mut open = true;
    let win = egui::Window::new("Prefab Library").default_size([680.0, 560.0]);
    crate::ui::popout::window(ctx, crate::ui::popout::Dock::BuildingsBrowser, win, &mut open, |ui| body(ui, ed));
    ed.buildings_browser_open &= open;
}

fn is_fav(ed: &Editor, k: BuildingKind) -> bool {
    let key = format!("{k:?}");
    ed.prefs.layout.prefab_favourites.contains(&key)
}

fn toggle_fav(ed: &mut Editor, k: BuildingKind) {
    let key = format!("{k:?}");
    let favs = &mut ed.prefs.layout.prefab_favourites;
    if let Some(i) = favs.iter().position(|f| *f == key) {
        favs.remove(i);
    } else {
        favs.push(key);
    }
    ed.iface.layout_dirty = true;
}

fn body(ui: &mut egui::Ui, ed: &mut Editor) {
    let id = egui::Id::new("prefab_library_state");
    let mut st: State = ui.data_mut(|d| d.get_temp(id).unwrap_or_default());
    ui.horizontal_wrapped(|ui| {
        let n = BuildingKind::ALL.len();
        if ui.selectable_label(st.filter == Filter::All, format!("All ({n})")).clicked() {
            st.filter = Filter::All;
        }
        let favs = BuildingKind::ALL.iter().filter(|k| is_fav(ed, **k)).count();
        if ui.selectable_label(st.filter == Filter::Favourites, format!("Favourites ({favs})")).clicked() {
            st.filter = Filter::Favourites;
        }
        for c in Category::ALL {
            let label = format!("{} ({})", c.label(), c.kinds().count());
            if ui.selectable_label(st.filter == Filter::Cat(c), label).clicked() {
                st.filter = Filter::Cat(c);
            }
        }
    });
    ui.horizontal(|ui| {
        ui.label("Search:");
        ui.add(egui::TextEdit::singleline(&mut st.search).desired_width(200.0).hint_text("cottage, bridge, lamp..."));
    });
    place_bar(ui, ed, &mut st);
    ui.separator();
    let needle = st.search.to_lowercase();
    let kinds: Vec<BuildingKind> = BuildingKind::ALL
        .into_iter()
        .filter(|k| match st.filter {
            Filter::All => true,
            Filter::Favourites => is_fav(ed, *k),
            Filter::Cat(c) => k.category() == c,
        })
        .filter(|k| needle.is_empty() || k.label().to_lowercase().contains(&needle) || k.category().label().to_lowercase().contains(&needle))
        .collect();
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        let cols = ((ui.available_width() / (TILE + 6.0)) as usize).max(1);
        egui::Grid::new("prefab_library_grid").spacing([6.0, 6.0]).show(ui, |ui| {
            for (i, k) in kinds.iter().enumerate() {
                tile(ui, ed, *k, st.drag);
                if i % cols == cols - 1 {
                    ui.end_row();
                }
            }
        });
        if kinds.is_empty() {
            let msg = if st.filter == Filter::Favourites { "No favourites yet: click the star on a tile." } else { "Nothing matches the search." };
            ui.label(dim(msg));
        }
    });
    ui.data_mut(|d| d.insert_temp(id, st));
}

/// Place-mode options: rotation, scale, Vary Seed, Drop on Surfaces.
fn place_bar(ui: &mut egui::Ui, ed: &mut Editor, st: &mut State) {
    let o = &mut ed.iface.place;
    ui.horizontal_wrapped(|ui| {
        ui.label("Rotation:");
        if ui.button("⟲").on_hover_text("Turn 15° left (mouse wheel while placing)").clicked() {
            o.rotate(-1.0);
        }
        max_spinner(ui, &mut o.yaw_deg, 1.0, 0.0);
        o.yaw_deg = o.yaw_deg.rem_euclid(360.0);
        if ui.button("⟳").on_hover_text("Turn 15° right (mouse wheel while placing)").clicked() {
            o.rotate(1.0);
        }
        ui.label("Scale:");
        if ui.button("−").on_hover_text("Smaller ([ while placing)").clicked() {
            o.rescale(false);
        }
        max_spinner(ui, &mut o.scale, 0.01, 1.0);
        o.scale = o.scale.clamp(0.05, 20.0);
        if ui.button("+").on_hover_text("Bigger (] while placing)").clicked() {
            o.rescale(true);
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.checkbox(&mut o.vary_seed, "Vary Seed").on_hover_text("Each copy gets its own seed, so stamped copies differ");
        ui.checkbox(&mut o.on_surface, "Drop on Surfaces").on_hover_text("Land on the object under the cursor (roofs, terrain, tables); off = the grid");
        ui.checkbox(&mut st.drag, "Drag to Size").on_hover_text("A tile click arms the drag tool instead: drag out the footprint in a viewport");
    });
    let hint = if st.drag {
        "Click a tile, then drag out the footprint in a viewport. Right-click a tile: place at the origin."
    } else {
        "Click a tile, then click in a viewport. Shift-click stamps copies; wheel rotates, [ ] scale, right-click ends."
    };
    ui.label(dim(hint));
}

/// A five-point star (favourite marker) centred on `c`.
fn star(p: &egui::Painter, c: Pos2, r: f32, on: bool, col: Color32) {
    let pts: Vec<Pos2> = (0..10)
        .map(|i| {
            let a = std::f32::consts::PI * (i as f32 / 5.0) - std::f32::consts::FRAC_PI_2;
            let rr = if i % 2 == 0 { r } else { r * 0.45 };
            c + egui::vec2(a.cos(), a.sin()) * rr
        })
        .collect();
    if on {
        // Filled as triangles from the centre (a star is not convex).
        for i in 0..10 {
            p.add(egui::Shape::convex_polygon(vec![c, pts[i], pts[(i + 1) % 10]], col, Stroke::NONE));
        }
    }
    let mut ring = pts;
    ring.push(ring[0]);
    p.add(egui::Shape::line(ring, Stroke::new(1.2, col)));
}

/// One thumbnail tile with its favourite star.
fn tile(ui: &mut egui::Ui, ed: &mut Editor, k: BuildingKind, drag: bool) {
    let kind = WorldKind::Building(k);
    let active = ed.tool == Tool::CreateWorld(kind) || ed.tool == Tool::PlacePrefab(k);
    let (rect, r) = ui.allocate_exact_size(egui::vec2(TILE, TILE + 18.0), Sense::click());
    let v = ui.visuals();
    let fill = if r.hovered() { v.widgets.hovered.bg_fill } else { v.extreme_bg_color };
    let border = if active { super::theme::accent() } else { v.widgets.noninteractive.bg_stroke.color };
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 10.0, fill);
    p.rect_stroke(rect, 10.0, Stroke::new(if active { 2.0 } else { 1.0 }, border), egui::StrokeKind::Inside);
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
    let star_rect = egui::Rect::from_center_size(rect.right_top() + egui::vec2(-14.0, 14.0), egui::vec2(22.0, 22.0));
    let fav = is_fav(ed, k);
    let on_star = r.hover_pos().is_some_and(|q| star_rect.contains(q));
    if fav || r.hovered() {
        let col = if fav {
            super::theme::accent()
        } else if on_star {
            super::theme::hover()
        } else {
            v.weak_text_color()
        };
        star(&p, star_rect.center(), 8.0, fav, col);
    }
    let (w, l, f) = k.defaults();
    let floors = if k.has_floors() { format!(", {f} floors") } else { String::new() };
    let r = r.on_hover_text(format!("{}\n{}\n{w} x {l} m{floors}\nStar: add to Favourites", k.label(), k.category().label()));
    if r.clicked() {
        if r.interact_pointer_pos().is_some_and(|q| star_rect.contains(q)) {
            toggle_fav(ed, k);
        } else if active {
            ed.tool = Tool::None;
        } else if drag {
            ed.tool = Tool::CreateWorld(kind);
            ed.status = format!("{}: drag in a viewport to set the footprint", k.label());
        } else {
            ed.tool = Tool::PlacePrefab(k);
            ed.status = format!("{}: click to drop, Shift-click to stamp copies", k.label());
        }
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
    rollout(ui, "Prefabs: Buildings & Props", true, |ui| {
        if ui
            .add_sized([content_w(), 26.0], egui::Button::new("Open Prefab Library...").selected(ed.buildings_browser_open))
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
                            .add_sized([content_w() / 2.0 - 12.0, 24.0], egui::Button::new(k.label()).selected(active))
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
        ui.label(dim("Each piece stands alone with the 29-ID Settlement pack. Drag sets Width x Length; right-click places at the origin."));
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
