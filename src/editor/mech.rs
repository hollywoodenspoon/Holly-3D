//! Contact Sets and Motion Links, as edited from the toolbar and the
//! Assembly rollout (each change is one undo step).

use super::*;

impl Editor {
    /// Toolbar `[Contact]`: Contact Members push each other while on.
    pub fn toggle_contacts(&mut self) {
        let a = &mut self.scene.assembly;
        a.contacts_on = !a.contacts_on;
        self.status = format!("Contact Sets {}", if a.contacts_on { "on" } else { "off" });
    }

    pub fn set_contact_member(&mut self, id: ObjId, on: bool) {
        self.checkpoint("Contact Member");
        let m = &mut self.scene.assembly.contact_members;
        m.retain(|&x| x != id);
        if on {
            m.push(id);
        }
    }

    /// Links the current part's hinge / slider to `driver`'s (None unlinks),
    /// starting from the Auto-Ratio.
    pub fn set_driver(&mut self, driver: Option<ObjId>) {
        let Some(id) = self.current() else { return };
        self.checkpoint("Motion Link");
        let Some(d) = driver else {
            self.scene.assembly.links.retain(|l| l.driven != id);
            return;
        };
        let ratio = self.scene.auto_ratio(d, id).unwrap_or(1.0);
        if let Err(e) = self.scene.link_motion(d, id, ratio) {
            self.history.discard_last();
            self.status = e;
        }
    }

    /// The Ratio spinner (one undo step per drag).
    pub fn edit_link_ratio(&mut self, started: bool, ratio: f32) {
        let Some(id) = self.current() else { return };
        if started {
            self.checkpoint("Link Ratio");
        }
        self.scene.set_link_ratio(id, ratio);
    }

    /// `[Auto-Ratio from Gear Teeth / Radius]`.
    pub fn auto_link_ratio(&mut self) {
        let Some(id) = self.current() else { return };
        let Some(l) = self.scene.assembly.links.iter().find(|l| l.driven == id) else { return };
        let Some(r) = self.scene.auto_ratio(l.driver, id) else { return };
        self.checkpoint("Auto-Ratio");
        self.scene.set_link_ratio(id, r);
        self.status = format!("Motion Link ratio {r:.4}");
    }
}
