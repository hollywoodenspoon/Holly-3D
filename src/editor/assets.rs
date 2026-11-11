//! Preferences -> Assets commands: download, cancel and remove the CC0
//! packs, and pick up finished downloads.

use super::Editor;
use crate::assets::download::{self, Phase};
use crate::assets::{self, Pack, PackState};

impl Editor {
    /// `[ Download ]`: fetches `pack` in the background (resumes a partial one).
    pub fn download_pack(&mut self, pack: Pack) {
        if self.downloads.get(&pack).is_some_and(|j| j.running()) {
            return;
        }
        let job = download::start(pack, self.prefs.assets.folder.clone(), self.prefs.assets.sources.clone());
        self.downloads.insert(pack, job);
        self.status = format!("Downloading {} into {}", pack.label(), pack.path(&self.prefs.assets.folder).display());
    }

    /// `[ Cancel ]`: stops a running download; what arrived stays for a later resume.
    pub fn cancel_download(&mut self, pack: Pack) {
        if let Some(j) = self.downloads.get(&pack) {
            j.cancel();
        }
    }

    /// `[ Remove ]`: deletes the pack's folder.
    pub fn remove_pack(&mut self, pack: Pack) {
        self.cancel_download(pack);
        self.downloads.remove(&pack);
        self.status = match assets::remove(pack, &self.prefs.assets.folder) {
            Ok(()) => format!("{} removed", pack.label()),
            Err(e) => format!("Remove failed: {e}"),
        };
        if pack == Pack::Photos {
            assets::photo::rescan(&self.prefs.assets.folder);
        }
    }

    /// Whether `pack` is missing, partial or installed.
    pub fn pack_state(&self, pack: Pack) -> PackState {
        assets::state(pack, &self.prefs.assets.folder)
    }

    /// Called every frame: reports downloads that just finished and adds
    /// new photo materials to the Material Library.
    pub fn poll_downloads(&mut self) {
        let finished: Vec<(Pack, Phase, u32)> = self
            .downloads
            .iter()
            .map(|(p, j)| (*p, j.snapshot()))
            .filter(|(_, s)| s.phase != Phase::Running)
            .map(|(p, s)| (p, s.phase, s.skipped))
            .collect();
        for (pack, phase, skipped) in finished {
            self.downloads.remove(&pack);
            self.status = match phase {
                Phase::Done => {
                    let extra = if pack == Pack::Photos {
                        format!(": {} photo materials in the Material Library", assets::photo::rescan(&self.prefs.assets.folder))
                    } else {
                        String::new()
                    };
                    let skipped = if skipped > 0 { format!(" ({skipped} optional maps were not on the host)") } else { String::new() };
                    format!("{} installed{extra}{skipped}", pack.label())
                }
                Phase::Cancelled => format!("{} download cancelled (Download resumes it)", pack.label()),
                Phase::Failed(e) => format!("{} download failed: {e}", pack.label()),
                Phase::Running => continue,
            };
        }
    }
}
