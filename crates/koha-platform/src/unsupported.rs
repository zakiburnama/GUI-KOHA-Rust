//! Implementasi untuk OS yang belum didukung: semua operasi mengembalikan
//! [`PlatformError::Unsupported`] dengan pesan yang jelas. `launch` dan
//! `open_url` untuk Linux dan macOS menyusul di langkah 7.

use crate::{Platform, PlatformError};

#[derive(Debug, Default, Clone, Copy)]
pub struct UnsupportedPlatform;

/// Di OS selain Windows stderr selalu dianggap terlihat (tidak ada pembedaan
/// subsistem GUI dan konsol), jadi tidak ada yang perlu ditempelkan.
pub fn attach_parent_console() -> bool {
    true
}

/// Padanan `show_error_dialog` Windows: tanpa dialog, cukup ke stderr.
pub fn show_error_dialog(title: &str, message: &str) {
    eprintln!("{title}: {message}");
}

impl Platform for UnsupportedPlatform {
    fn launch(&self, _command: &str, _args: &[String], _admin: bool) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported("launch"))
    }

    fn open_url(&self, _url: &str) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported("open_url"))
    }

    fn lock(&self) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported("lock"))
    }

    fn sleep(&self) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported("sleep"))
    }

    fn close_all_windows(&self) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported("close_all_windows"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_operation_reports_itself_as_unsupported() {
        let platform = UnsupportedPlatform;
        assert_eq!(
            platform.launch("x", &[], false),
            Err(PlatformError::Unsupported("launch"))
        );
        assert_eq!(
            platform.open_url("https://x.dev"),
            Err(PlatformError::Unsupported("open_url"))
        );
        assert_eq!(platform.lock(), Err(PlatformError::Unsupported("lock")));
        assert_eq!(platform.sleep(), Err(PlatformError::Unsupported("sleep")));
        assert_eq!(
            platform.close_all_windows(),
            Err(PlatformError::Unsupported("close_all_windows"))
        );
    }

    #[test]
    fn the_message_names_the_operation() {
        let message = PlatformError::Unsupported("lock").to_string();
        assert!(message.contains("lock"), "{message}");
    }
}
