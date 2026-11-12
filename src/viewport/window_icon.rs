//! The Ilex holly logo as the window / taskbar icon. The PNG is rendered
//! from `packaging/ilex.svg` and embedded; it is set once, on the main
//! thread, as soon as the primary window exists.

use bevy::ecs::system::NonSendMarker;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy::winit::WINIT_WINDOWS;

const ICON_PNG: &[u8] = include_bytes!("../../assets/icon/ilex-64.png");

/// The decoded icon: (RGBA8 pixels, width, height).
pub fn icon_rgba() -> Option<(Vec<u8>, u32, u32)> {
    let img = image::load_from_memory_with_format(ICON_PNG, image::ImageFormat::Png).ok()?.into_rgba8();
    let (w, h) = img.dimensions();
    Some((img.into_raw(), w, h))
}

fn set_icon(mut done: Local<bool>, window: Query<Entity, With<PrimaryWindow>>, _main_thread: NonSendMarker) {
    if *done {
        return;
    }
    let Ok(entity) = window.single() else { return };
    let Some((rgba, w, h)) = icon_rgba() else {
        *done = true;
        return;
    };
    WINIT_WINDOWS.with_borrow(|windows| {
        if let Some(win) = windows.get_window(entity) {
            win.set_window_icon(winit::window::Icon::from_rgba(rgba, w, h).ok());
            *done = true;
        }
    });
}

pub fn register(app: &mut App) {
    app.add_systems(Update, set_icon);
}

#[cfg(test)]
mod tests {
    #[test]
    fn embedded_icon_decodes_to_the_white_holly_on_teal() {
        let (px, w, h) = super::icon_rgba().unwrap();
        assert_eq!((w, h, px.len()), (64, 64, 64 * 64 * 4));
        let at = |x: usize, y: usize| &px[(y * 64 + x) * 4..(y * 64 + x) * 4 + 4];
        let teal = at(8, 32);
        assert!(teal[1] > teal[0] + 40 && teal[3] == 255, "blue-green tile: {teal:?}");
        let white = (0..64 * 64).filter(|i| px[i * 4] > 240 && px[i * 4 + 1] > 240 && px[i * 4 + 2] > 240).count();
        assert!(white > 300, "a white tree ({white} white pixels)");
    }
}
