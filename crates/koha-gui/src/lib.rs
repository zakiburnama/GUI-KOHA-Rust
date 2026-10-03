//! GUI KOHA. Logika yang bisa dites tanpa jendela ada di [`layout`],
//! [`render`], dan [`input`]; [`window`] adalah perekat tipis ke `winit` dan
//! `softbuffer`.

pub mod input;
pub mod layout;
pub mod render;
pub mod window;

pub use input::{FocusGate, KeyAction, map_key};
pub use layout::{Layout, Rect, centered_position};
pub use render::render;
pub use window::{GuiError, run};

/// Judul jendela popup.
pub fn window_title() -> String {
    koha_core::APP_NAME.to_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_is_uppercase_app_name() {
        assert_eq!(window_title(), "KOHA");
    }
}
