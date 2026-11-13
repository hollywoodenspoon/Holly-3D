//! UI fonts. Fira Code (OFL-1.1) is built into the program and is the
//! default for all text. egui's own fonts stay behind it as fallbacks, so
//! emoji and icon glyphs still draw.

use crate::prefs::UiFont;
use bevy_egui::egui::{self, FontData, FontDefinitions, FontFamily};
use std::sync::Arc;

const REGULAR: &[u8] = include_bytes!("../../assets/fonts/FiraCode-Regular.ttf");
const BOLD: &[u8] = include_bytes!("../../assets/fonts/FiraCode-Bold.ttf");

/// Family name of the bold face (headings).
pub const BOLD_FAMILY: &str = "FiraCode-Bold";

fn installed_id() -> egui::Id {
    egui::Id::new("minimax_ui_font")
}

/// The font family headings use: the bold face once [`install`] has put
/// Fira Code into this context, the plain family otherwise.
pub fn heading_family(ctx: &egui::Context) -> FontFamily {
    match ctx.data(|d| d.get_temp::<UiFont>(installed_id())) {
        Some(UiFont::FiraCode) => FontFamily::Name(BOLD_FAMILY.into()),
        _ => FontFamily::Proportional,
    }
}

/// The font this context was last given (`None` before the first install).
pub fn installed(ctx: &egui::Context) -> Option<UiFont> {
    ctx.data(|d| d.get_temp::<UiFont>(installed_id()))
}

/// egui font definitions for a Preferences > Theme font choice.
pub fn definitions(font: UiFont) -> FontDefinitions {
    let mut d = FontDefinitions::default();
    if font == UiFont::Classic {
        return d;
    }
    d.font_data.insert("FiraCode".into(), Arc::new(FontData::from_static(REGULAR)));
    d.font_data.insert(BOLD_FAMILY.into(), Arc::new(FontData::from_static(BOLD)));
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        d.families.entry(family).or_default().insert(0, "FiraCode".into());
    }
    // Bold first, then everything the regular family falls back to.
    let mut bold = vec![BOLD_FAMILY.to_string()];
    bold.extend(d.families.get(&FontFamily::Proportional).cloned().unwrap_or_default());
    d.families.insert(FontFamily::Name(BOLD_FAMILY.into()), bold);
    d
}

/// Installs the fonts of a Preferences > Theme choice. egui loads them at
/// the start of the next frame, so styles that name the bold family must
/// wait until then (see `ui_system`).
pub fn install(ctx: &egui::Context, font: UiFont) {
    ctx.set_fonts(definitions(font));
    ctx.data_mut(|d| d.insert_temp(installed_id(), font));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn width(ctx: &egui::Context, text: &str, family: FontFamily) -> f32 {
        ctx.fonts_mut(|f| f.layout_no_wrap(text.into(), egui::FontId::new(14.0, family), egui::Color32::WHITE).rect.width())
    }

    fn context(font: UiFont) -> egui::Context {
        let ctx = egui::Context::default();
        install(&ctx, font);
        ctx.run_ui(Default::default(), |_| {}).textures_delta.clear();
        ctx
    }

    #[test]
    fn fira_code_is_the_default_font_with_fallbacks_for_icons() {
        assert_eq!(UiFont::default(), UiFont::FiraCode);
        let d = definitions(UiFont::FiraCode);
        for family in [FontFamily::Proportional, FontFamily::Monospace, FontFamily::Name(BOLD_FAMILY.into())] {
            let list = &d.families[&family];
            assert!(list[0].starts_with("FiraCode") && list.len() > 2, "{family:?}: {list:?}");
        }
        // Both faces are real fonts built into the program.
        assert!(REGULAR.len() > 100_000 && BOLD.len() > 100_000 && REGULAR != BOLD);
        // Fira Code is monospaced: narrow and wide letters take the same room, in both families.
        let ctx = context(UiFont::FiraCode);
        for family in [FontFamily::Proportional, FontFamily::Monospace, heading_family(&ctx)] {
            assert!((width(&ctx, "iiii", family.clone()) - width(&ctx, "MMMM", family)).abs() < 0.01);
        }
        // Icons and emoji the UI uses still draw (from the fallback fonts).
        let blank = width(&ctx, "\u{E000}", FontFamily::Proportional);
        assert!(blank >= 0.0 && width(&ctx, "🎨📚★", FontFamily::Proportional) > 20.0);
    }

    #[test]
    fn the_classic_font_stays_available() {
        let d = definitions(UiFont::Classic);
        assert!(!d.font_data.contains_key("FiraCode"));
        let ctx = context(UiFont::Classic);
        assert!(width(&ctx, "iiii", FontFamily::Proportional) < width(&ctx, "MMMM", FontFamily::Proportional));
        assert_eq!(heading_family(&ctx), FontFamily::Proportional);
        assert_eq!(heading_family(&egui::Context::default()), FontFamily::Proportional, "nothing installed yet");
        assert_eq!(installed(&ctx), Some(UiFont::Classic));
    }
}
