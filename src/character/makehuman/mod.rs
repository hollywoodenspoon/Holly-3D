//! Phase 10A: MakeHuman 1.x hm08 support (CC0 base mesh, targets,
//! modifiers, proxies). See `assets/makehuman/LICENSE.ASSETS.md`.

pub mod assets;
pub mod library;
pub mod macros;
pub mod mhclo;
pub mod modifiers;
pub mod obj;
pub mod pack;
pub mod target;
pub mod unzip;
pub mod user_lib;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_user;
