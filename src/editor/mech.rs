//! Contact Sets and Motion Links (toolbar and Assembly rollout) and the
//! Mech primitives (Create tab): Gear, Hydraulic Piston and Hose. Each
//! change is one undo step.

use super::*;
use crate::assembly::Frame;
use crate::assembly::mount::{Mount, piston_at};
use crate::mech::{Gear, Hose, MechKind, MechPart, PistonPart};
use crate::scene::BaseObject;
use bevy::math::Quat;

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

    /// Create -> Geometry -> Mech. A piston is a housing and a rod (rod
    /// above, upside down) joined by a Hydraulic Piston joint.
    pub fn create_mech(&mut self, kind: MechKind, at: Vec3) {
        self.checkpoint("Create Mech");
        let mech = |m| BaseObject::Mech(m);
        let id = match kind {
            MechKind::Gear => self.scene.add(mech(MechPart::Gear(Gear::default())), at),
            MechKind::Hose => self.scene.add(mech(MechPart::Hose(Hose::default())), at),
            MechKind::Bolt | MechKind::Nut | MechKind::TappedHole => {
                let f = crate::mech::Fastener::new(kind.fastener().expect("a fastener kind"));
                self.scene.add(mech(MechPart::Fastener(f)), at)
            }
            MechKind::Piston => {
                let (hp, rp) = (PistonPart::housing(), PistonPart::rod());
                let reach = 1.2 * hp.eyelet + hp.length + 1.2 * rp.eyelet + 0.3 * rp.length;
                let h = self.scene.add(mech(MechPart::Piston(hp)), at);
                let r = self.scene.add(mech(MechPart::Piston(rp)), at + Vec3::Z * reach);
                self.scene.objects[r].transform.rotation = Quat::from_rotation_x(std::f32::consts::PI);
                let f = |s: &Scene, id| Frame::of(&s.objects[id].transform);
                let (a, b) =
                    (Mount { obj: h, point: at, normal: Vec3::Z }, Mount { obj: r, point: at + Vec3::Z * reach, normal: -Vec3::Z });
                let j = piston_at((h, f(&self.scene, h)), (r, f(&self.scene, r)), &a, &b);
                // New parts cannot form a loop.
                let _ = self.scene.add_joint(j);
                h
            }
        };
        self.scene.select_only(Some(id));
        self.sub = SubObject::Object;
        self.tab = CommandTab::Modify;
        self.status = format!("Created {}", self.scene.objects[id].name);
    }

    /// Binds the current hose's top (start) or bottom (end) to `obj`, or frees it.
    pub fn bind_hose(&mut self, top: bool, obj: Option<ObjId>) {
        let Some(id) = self.current() else { return };
        if obj == Some(id) {
            return;
        }
        self.checkpoint("Bind Hose");
        if let Some(MechPart::Hose(h)) = self.scene.objects[id].mech_mut() {
            if top {
                h.top = obj;
            } else {
                h.bottom = obj;
            }
        }
        self.scene.update_hoses();
    }

    /// Create -> Geometry -> Mech -> `[Spider: Railgun]`: the 4-leg Spider
    /// Platform with the Railgun turret, every joint live, hull Grounded.
    pub fn create_spider_railgun(&mut self, at: Vec3) {
        self.checkpoint("Create Spider Platform");
        for o in self.scene.objects.values_mut() {
            o.selected = false;
        }
        let rig = crate::mech::spider::spawn_railgun(&mut self.scene, at);
        for id in rig.parts() {
            self.scene.objects[id].selected = true;
        }
        self.sub = SubObject::Object;
        self.status = "Spider Platform (Railgun): drag a foot with W to bend its leg, turn joints with E".into();
    }
}
