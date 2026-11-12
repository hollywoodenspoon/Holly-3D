//! The Face Cloner's "MobileSAM Masks" card: download (about 41 MB),
//! install from a file, or remove the optional MobileSAM weights.

use crate::character::face_cloner::job::{self, Job, Msg};
use crate::character::face_cloner::sam;
use crate::ui::widgets::{dim, rollout as card};
use bevy_egui::egui;

#[derive(Default)]
pub struct SamUi {
    job: Option<Job>,
    message: String,
}

impl SamUi {
    pub fn busy(&self) -> bool {
        self.job.is_some()
    }

    fn poll(&mut self) {
        let Some(j) = &self.job else { return };
        let finished = j.is_finished();
        for m in j.poll() {
            if let Msg::Sam(r) = m {
                self.message = match r {
                    Ok(()) => "MobileSAM installed and checked. New clones use it.".into(),
                    Err(e) => format!("MobileSAM was not installed: {e}"),
                };
            }
        }
        if finished {
            self.job = None;
        }
    }
}

pub fn show(ui: &mut egui::Ui, st: &mut SamUi) {
    st.poll();
    card(ui, "4. MobileSAM Masks (optional)", false, |ui| {
        ui.label(dim("Without it the skin is masked by the face outline and a skin colour model, which drops dark hair, brows and glasses frames. MobileSAM (Apache-2.0) also cuts out skin-coloured hair, hands and background. It runs on the CPU, about 1-3 s a photo."));
        let have = sam::installed();
        ui.label(if have { format!("Installed: {}", sam::path().display()) } else { "Not installed.".into() });
        if let Some(j) = &st.job {
            let (d, t) = j.progress();
            ui.horizontal(|ui| {
                ui.add(egui::ProgressBar::new(d as f32 / t.max(1) as f32).desired_width(240.0).text(format!("{:.1} / {:.1} MB", d as f32 / 1000.0, t as f32 / 1000.0)));
                if ui.button("Cancel").clicked() {
                    j.cancel();
                }
            });
        }
        ui.add_enabled_ui(st.job.is_none(), |ui| {
            ui.horizontal(|ui| {
                if ui.add_enabled(!have, egui::Button::new(format!("⬇ Download ({:.0} MB)", sam::size_mb()))).on_hover_text("From the MobileSAM GitHub repository; checked against its SHA-256").clicked() {
                    st.message = "Downloading MobileSAM…".into();
                    st.job = Some(job::download_sam());
                }
                if ui.button("Install from File...").on_hover_text("A mobile_sam.pt you downloaded yourself").clicked()
                    && let Some(p) = rfd::FileDialog::new().set_title("MobileSAM weights").add_filter("PyTorch weights", &["pt"]).pick_file()
                {
                    st.message = "Checking the file…".into();
                    st.job = Some(job::install_sam(p));
                }
                if ui.add_enabled(have, egui::Button::new("🗑 Remove")).clicked() {
                    st.message = match sam::remove() {
                        Ok(()) => "MobileSAM removed.".into(),
                        Err(e) => e,
                    };
                }
            });
        });
        if !st.message.is_empty() {
            ui.label(&st.message);
        }
    });
}
