//! Logika murni KOHA. Crate ini tidak boleh memanggil API OS maupun GUI.
//!
//! Peta modul:
//! - [`config`]: model dan parser `config.toml` (menu, aksi, tema tambahan).
//! - [`state`]: `state.toml`, yang ditulis aplikasi (tema aktif, item yang di-OFF).
//! - [`theme`] dan [`rgb`]: tema, warna, dan identitas font per tema.
//! - [`menu`]: state machine navigasi (`Input` masuk, `Effect` keluar).
//!
//! Crate ini hanya mengurai dan menghasilkan teks; membaca dan menulis file
//! dilakukan oleh `koha-app`.

pub mod config;
pub mod menu;
pub mod rgb;
pub mod state;
pub mod theme;

pub use config::{Action, Builtin, Config, ConfigError, DEFAULT_CONFIG, ItemKind, MenuItem};
pub use menu::{Effect, Input, MENU_SETTINGS_LABEL, MenuState, Row};
pub use rgb::{ParseRgbError, Rgb};
pub use state::State;
pub use theme::{DEFAULT_THEME, Theme, ThemeSet};

/// Nama aplikasi, dipakai bersama oleh crate lain.
pub const APP_NAME: &str = "koha";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_name_is_koha() {
        assert_eq!(APP_NAME, "koha");
    }
}
