//! Logika murni KOHA. Crate ini tidak boleh memanggil API OS maupun GUI.

pub mod config;
pub mod rgb;
pub mod theme;

pub use config::{Action, Builtin, Config, ConfigError, ItemKind, MenuItem};
pub use rgb::{ParseRgbError, Rgb};
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
