//! Face Cloner window (Tools -> Face Cloner, the Character rollout's
//! `[ Load Photos / Frame Folder... ]`): load photos or a folder of video
//! frames, see how they were sorted into views, fix landmarks with pins,
//! then fit the selected character's head to them.

pub mod pins;
pub mod rollout;

use crate::character::face_cloner::clone::{Options, Report};
use crate::character::face_cloner::frames::{pick, Picking, Shot, ViewBin};
use crate::character::face_cloner::job::{self, file_name, Job, Msg, IMAGE_EXTS};
use crate::editor::Editor;
use crate::scene::ObjId;
use crate::ui::widgets::{dim, rollout as card};
use bevy_egui::egui;

/// Window state (UI only, not undoable).
pub struct FaceUi {
    pub shots: Vec<Shot>,
    pub picking: Picking,
    pub neutral_only: bool,
    /// Read every n-th picture of a frame folder.
    pub stride: usize,
    pub options: Options,
    pub job: Option<Job>,
    /// The running job reads pictures (else it fits).
    loading: bool,
    /// The character a running fit belongs to.
    fit_for: Option<ObjId>,
    pub message: String,
    pub skipped: Vec<String>,
    pub report: Option<Report>,
    pub pins: pins::PinEditor,
}

impl Default for FaceUi {
    fn default() -> Self {
        Self { shots: Vec::new(), picking: Picking::default(), neutral_only: true, stride: 1, options: Options::default(), job: None, loading: false, fit_for: None, message: String::new(), skipped: Vec::new(), report: None, pins: Default::default() }
    }
}

impl FaceUi {
    fn rules(&self) -> Picking {
        Picking { max_mouth_open: if self.neutral_only { self.picking.max_mouth_open } else { f32::MAX }, ..self.picking }
    }

    /// Re-runs the frame picking (this resets hand-ticked "use" boxes).
    pub fn repick(&mut self) {
        let rules = self.rules();
        pick(&mut self.shots, &rules);
    }

    fn start_load(&mut self, paths: Vec<std::path::PathBuf>) {
        if paths.is_empty() {
            self.message = "No pictures found there (PNG, JPG, BMP, WebP or TIFF).".into();
            return;
        }
        self.message = format!("Finding faces in {} picture(s)…", paths.len());
        self.job = Some(job::load(paths));
        self.loading = true;
    }

    pub fn load_photos(&mut self) {
        if let Some(p) = rfd::FileDialog::new().set_title("Face photos").add_filter("Pictures", &IMAGE_EXTS).pick_files() {
            self.start_load(p);
        }
    }

    pub fn load_folder(&mut self) {
        if let Some(dir) = rfd::FileDialog::new().set_title("Folder of video frames").pick_folder() {
            self.start_load(job::image_files(&dir, self.stride));
        }
    }

    /// Takes finished work from the background job.
    fn poll(&mut self, ed: &mut Editor) {
        let Some(j) = &self.job else { return };
        let finished = j.is_finished();
        for m in j.poll() {
            match m {
                Msg::Fitted(r) => self.fitted(ed, *r),
                Msg::Shot(Ok(s)) => self.shots.push(s),
                Msg::Shot(Err(e)) => self.skipped.push(e),
            }
        }
        if finished {
            if std::mem::take(&mut self.loading) {
                self.repick();
                let used = self.shots.iter().filter(|s| s.used).count();
                self.message = format!("{} picture(s) with a face, {used} picked. {} skipped.", self.shots.len(), self.skipped.len());
            }
            self.job = None;
        }
    }

    fn fitted(&mut self, ed: &mut Editor, r: Result<(crate::character::params::CharacterParams, Report), String>) {
        match r {
            Ok((p, rep)) => {
                if let Some(id) = self.fit_for.take().filter(|id| ed.scene.objects.contains_key(*id)) {
                    ed.checkpoint("Clone Face");
                    ed.set_character(id, p);
                    self.message = format!("Face cloned from {} view(s): {:.2} mm -> {:.2} mm average landmark error.", rep.views, rep.before_mm, rep.warp_mm);
                    ed.status = self.message.clone();
                }
                self.report = Some(rep);
            }
            Err(e) => self.message = e,
        }
    }

    fn start_fit(&mut self, ed: &Editor) {
        let Some((id, p)) = ed.character() else {
            self.message = "Select a character first (Create -> Characters).".into();
            return;
        };
        self.fit_for = Some(id);
        self.report = None;
        self.message = "Fitting the head (sliders, then render-and-check passes)…".into();
        self.job = Some(job::fit(p, self.shots.clone(), self.options));
        self.loading = false;
    }
}

pub fn show(ctx: &egui::Context, ed: &mut Editor, st: &mut FaceUi) {
    st.poll(ed);
    if std::mem::take(&mut ed.iface.face_load_request) {
        st.load_photos();
    }
    if !ed.iface.face_cloner_open {
        return;
    }
    if st.job.is_some() {
        ctx.request_repaint_after(std::time::Duration::from_millis(150));
    }
    let mut open = true;
    egui::Window::new("📸 Face Cloner — Photos / Video Frames").open(&mut open).default_size([420.0, 640.0]).show(ctx, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| body(ui, ed, st));
    });
    ed.iface.face_cloner_open &= open;
    pins::show(ctx, st);
}

fn body(ui: &mut egui::Ui, ed: &mut Editor, st: &mut FaceUi) {
    let busy = st.job.is_some();
    card(ui, "1. Pictures", true, |ui| {
        ui.add_enabled_ui(!busy, |ui| {
            ui.horizontal(|ui| {
                if ui.button("📂 Load Photos...").on_hover_text("One or more photos of the same person").clicked() {
                    st.load_photos();
                }
                if ui.button("🎞 Load Frame Folder...").on_hover_text("A folder of frames exported from a video").clicked() {
                    st.load_folder();
                }
                if ui.button("Clear").clicked() {
                    (st.shots, st.skipped, st.report) = (Vec::new(), Vec::new(), None);
                    st.message.clear();
                }
            });
            ui.horizontal(|ui| {
                ui.label("Frame folder: read every");
                ui.add(egui::DragValue::new(&mut st.stride).range(1..=60));
                ui.label("th picture");
            });
        });
        ui.label(dim("Videos are not read directly: export the clip to frames first (any frame-export tool). A slow head turn from one side to the other gives every view."));
        if let Some(j) = &st.job {
            let (d, t) = j.progress();
            ui.horizontal(|ui| {
                ui.add(egui::ProgressBar::new(d as f32 / t.max(1) as f32).desired_width(260.0).text(format!("{d} / {t}")));
                if ui.button("Cancel").clicked() {
                    j.cancel();
                }
            });
        }
        if !st.message.is_empty() {
            ui.label(&st.message);
        }
    });
    card(ui, "2. Frame Picking", true, |ui| {
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("Frames per view:");
            changed |= ui.add(egui::DragValue::new(&mut st.picking.per_view).range(1..=12)).changed();
        });
        ui.horizontal(|ui| {
            ui.label("Drop blurrier than:");
            changed |= ui.add(egui::Slider::new(&mut st.picking.min_sharpness, 0.0..=0.9).custom_formatter(|v, _| format!("{:.0}% of the sharpest", v * 100.0))).changed();
        });
        changed |= ui.checkbox(&mut st.neutral_only, "Neutral expression only (drop open mouths / big smiles)").changed();
        if changed {
            st.repick();
        }
        views(ui, st);
    });
    card(ui, "3. Clone onto the Selected Character", true, |ui| fit_ui(ui, ed, st, busy));
}

fn views(ui: &mut egui::Ui, st: &mut FaceUi) {
    ui.horizontal_wrapped(|ui| {
        for v in ViewBin::ALL {
            let (n, used) = (st.shots.iter().filter(|s| s.view == v).count(), st.shots.iter().filter(|s| s.view == v && s.used).count());
            ui.label(egui::RichText::new(format!("{}: {used}/{n}", v.label())).color(if used > 0 { crate::ui::theme::amber() } else { crate::ui::theme::text_dim() }));
        }
    });
    let mut open_pins = None;
    egui::ScrollArea::vertical().id_salt("face_shots").max_height(220.0).show(ui, |ui| {
        egui::Grid::new("face_shot_grid").num_columns(5).striped(true).show(ui, |ui| {
            for (i, s) in st.shots.iter_mut().enumerate() {
                ui.checkbox(&mut s.used, "").on_hover_text("Use this picture in the fit");
                ui.label(file_name(&s.name)).on_hover_text(&s.name);
                ui.label(format!("{} {:+.0}°", s.view.label(), s.yaw));
                ui.label(if s.note.is_empty() { dim(&format!("sharp {:.0}", s.sharpness)) } else { dim(&s.note) });
                let label = if s.pins.is_empty() { "Pins...".to_string() } else { format!("Pins ({})", s.pins.len()) };
                if ui.small_button(label).on_hover_text("Check and drag the found landmarks").clicked() {
                    open_pins = Some(i);
                }
                ui.end_row();
            }
        });
    });
    if let Some(i) = open_pins {
        st.pins.open(i);
    }
    if !st.skipped.is_empty() {
        ui.collapsing(format!("{} skipped", st.skipped.len()), |ui| {
            for e in &st.skipped {
                ui.label(dim(e));
            }
        });
    }
}

fn fit_ui(ui: &mut egui::Ui, ed: &mut Editor, st: &mut FaceUi, busy: bool) {
    let o = &mut st.options;
    ui.checkbox(&mut o.fine_warp, "Fine warp (detail the sliders cannot reach)");
    ui.horizontal(|ui| {
        ui.label("Render-and-check passes:");
        ui.add(egui::DragValue::new(&mut o.refine).range(0..=4));
    });
    ui.horizontal(|ui| {
        ui.label("Caution:");
        ui.add(egui::Slider::new(&mut o.ridge, 0.0005..=0.05).logarithmic(true).show_value(false)).on_hover_text("Higher keeps the result closer to the unchanged head");
    });
    let used = st.shots.iter().filter(|s| s.used).count();
    let who = ed.character().map(|(id, _)| ed.scene.objects[id].name.clone());
    let text = match &who {
        Some(n) => format!("✨ Clone Face onto {n} ({used} picture(s))"),
        None => "✨ Clone Face (select a character)".into(),
    };
    if ui.add_enabled(!busy && used > 0 && who.is_some(), egui::Button::new(text).min_size(egui::vec2(ui.available_width(), 28.0))).clicked() {
        st.start_fit(ed);
    }
    if let Some(r) = &st.report {
        ui.label(format!("{} view(s) · landmark error {:.2} -> {:.2} (sliders) -> {:.2} mm (warp) · {} sliders moved", r.views, r.before_mm, r.sliders_mm, r.warp_mm, r.moved.len()));
    }
    ui.label(dim("Reshapes the MakeHuman head (same topology, UVs and rig; eyes, teeth, hair and clothes follow). The head keeps the body's scale. Ears and the back of the head are guessed. The result is a starting point: the Detail Sliders and Warp Strength refine it."));
}
