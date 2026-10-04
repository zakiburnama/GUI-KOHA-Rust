//! Abstraksi OS (ports and adapters). `koha-core` memutuskan *apa* yang dijalankan
//! ([`koha_core::Action`]); crate ini tahu *bagaimana* menjalankannya di tiap OS.
//!
//! Implementasi dipilih saat kompilasi dengan `cfg`: [`SystemPlatform`] adalah
//! `WindowsPlatform` di Windows dan `UnsupportedPlatform` di OS lain.

use koha_core::{Action, Builtin};
use thiserror::Error;

pub mod cmdline;
pub mod window_filter;

#[cfg(windows)]
mod win;
#[cfg(windows)]
pub use win::{WindowsPlatform, attach_parent_console, closable_windows, show_error_dialog};
#[cfg(windows)]
pub type SystemPlatform = WindowsPlatform;

#[cfg(not(windows))]
mod unsupported;
#[cfg(not(windows))]
pub use unsupported::{UnsupportedPlatform, attach_parent_console, show_error_dialog};
#[cfg(not(windows))]
pub type SystemPlatform = UnsupportedPlatform;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PlatformError {
    /// Operasi ini belum ada untuk OS yang sedang dipakai.
    #[error("operasi \"{0}\" belum didukung di OS ini")]
    Unsupported(&'static str),
    /// OS menolak atau gagal menjalankan operasi.
    #[error("{operation} gagal: {message}")]
    Failed {
        operation: &'static str,
        message: String,
    },
    #[error("bukan URL yang valid (harus diawali skema seperti https:): {0:?}")]
    InvalidUrl(String),
    /// Pengguna sendiri membatalkan (misalnya menolak prompt UAC). Ini bukan
    /// kegagalan, dan pemanggil sebaiknya keluar tanpa pesan error.
    #[error("dibatalkan oleh pengguna")]
    Cancelled,
    /// Aksi yang seharusnya ditangani menu (pemilih tema), bukan platform.
    #[error("aksi ini ditangani oleh menu dan tidak bisa dijalankan oleh platform")]
    NotExecutable,
}

/// Operasi OS yang dibutuhkan KOHA. Satu implementasi per OS.
pub trait Platform {
    /// Menjalankan program. `admin` meminta elevasi (UAC di Windows).
    fn launch(&self, command: &str, args: &[String], admin: bool) -> Result<(), PlatformError>;
    fn open_url(&self, url: &str) -> Result<(), PlatformError>;
    fn lock(&self) -> Result<(), PlatformError>;
    fn sleep(&self) -> Result<(), PlatformError>;
    fn close_all_windows(&self) -> Result<(), PlatformError>;
}

/// Menjalankan `action` lewat `platform`.
///
/// `&dyn Platform` adalah trait object: satu fungsi ini melayani implementasi
/// apa pun (OS sungguhan atau tiruan di test) tanpa digandakan per tipe.
pub fn execute(platform: &dyn Platform, action: &Action) -> Result<(), PlatformError> {
    match action {
        Action::Exec {
            command,
            args,
            admin,
        } => platform.launch(command, args, *admin),
        Action::Url(url) => platform.open_url(cmdline::validate_url(url)?),
        Action::Builtin(Builtin::Lock) => platform.lock(),
        Action::Builtin(Builtin::Sleep) => platform.sleep(),
        Action::Builtin(Builtin::CloseAllWindows) => platform.close_all_windows(),
        Action::Builtin(Builtin::ThemePicker) => Err(PlatformError::NotExecutable),
    }
}

/// Nama OS tempat crate ini dikompilasi.
pub fn os_name() -> &'static str {
    std::env::consts::OS
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// Platform tiruan: mencatat panggilan, dan bisa dipaksa gagal.
    #[derive(Default)]
    struct Mock {
        calls: RefCell<Vec<String>>,
        fail_with: Option<PlatformError>,
    }

    impl Mock {
        fn record(&self, call: String) -> Result<(), PlatformError> {
            self.calls.borrow_mut().push(call);
            self.fail_with.clone().map_or(Ok(()), Err)
        }

        fn calls(&self) -> Vec<String> {
            self.calls.borrow().clone()
        }
    }

    impl Platform for Mock {
        fn launch(&self, command: &str, args: &[String], admin: bool) -> Result<(), PlatformError> {
            self.record(format!("launch {command} {args:?} admin={admin}"))
        }
        fn open_url(&self, url: &str) -> Result<(), PlatformError> {
            self.record(format!("open_url {url}"))
        }
        fn lock(&self) -> Result<(), PlatformError> {
            self.record("lock".to_owned())
        }
        fn sleep(&self) -> Result<(), PlatformError> {
            self.record("sleep".to_owned())
        }
        fn close_all_windows(&self) -> Result<(), PlatformError> {
            self.record("close_all_windows".to_owned())
        }
    }

    fn exec(command: &str, args: &[&str], admin: bool) -> Action {
        Action::Exec {
            command: command.to_owned(),
            args: args.iter().map(|a| (*a).to_owned()).collect(),
            admin,
        }
    }

    #[test]
    fn exec_goes_to_launch_with_its_arguments_and_admin_flag() {
        let platform = Mock::default();
        execute(&platform, &exec("wt.exe", &["-d", "x"], true)).unwrap();
        assert_eq!(
            platform.calls(),
            [r#"launch wt.exe ["-d", "x"] admin=true"#]
        );
    }

    #[test]
    fn exec_without_admin_does_not_request_elevation() {
        let platform = Mock::default();
        execute(&platform, &exec("notepad", &[], false)).unwrap();
        assert_eq!(platform.calls(), ["launch notepad [] admin=false"]);
    }

    #[test]
    fn url_goes_to_open_url() {
        let platform = Mock::default();
        execute(&platform, &Action::Url("https://example.com".to_owned())).unwrap();
        assert_eq!(platform.calls(), ["open_url https://example.com"]);
    }

    #[test]
    fn invalid_url_is_rejected_before_reaching_the_platform() {
        let platform = Mock::default();
        let result = execute(&platform, &Action::Url("calc.exe".to_owned()));
        assert_eq!(
            result,
            Err(PlatformError::InvalidUrl("calc.exe".to_owned()))
        );
        assert!(platform.calls().is_empty());
    }

    #[test]
    fn each_builtin_goes_to_its_own_operation() {
        for (builtin, expected) in [
            (Builtin::Lock, "lock"),
            (Builtin::Sleep, "sleep"),
            (Builtin::CloseAllWindows, "close_all_windows"),
        ] {
            let platform = Mock::default();
            execute(&platform, &Action::Builtin(builtin)).unwrap();
            assert_eq!(platform.calls(), [expected]);
        }
    }

    #[test]
    fn theme_picker_is_never_executed_by_the_platform() {
        let platform = Mock::default();
        let result = execute(&platform, &Action::Builtin(Builtin::ThemePicker));
        assert_eq!(result, Err(PlatformError::NotExecutable));
        assert!(platform.calls().is_empty());
    }

    #[test]
    fn platform_errors_are_passed_through_unchanged() {
        for error in [
            PlatformError::Cancelled,
            PlatformError::Unsupported("lock"),
            PlatformError::Failed {
                operation: "launch",
                message: "tidak ada".to_owned(),
            },
        ] {
            let platform = Mock {
                fail_with: Some(error.clone()),
                ..Mock::default()
            };
            assert_eq!(
                execute(&platform, &Action::Builtin(Builtin::Lock)),
                Err(error)
            );
        }
    }

    #[test]
    fn error_messages_are_readable() {
        let failed = PlatformError::Failed {
            operation: "launch",
            message: "file tidak ditemukan".to_owned(),
        };
        assert_eq!(failed.to_string(), "launch gagal: file tidak ditemukan");
        assert!(
            PlatformError::InvalidUrl("x".into())
                .to_string()
                .contains("URL")
        );
    }

    #[test]
    fn os_name_is_not_empty() {
        assert!(!os_name().is_empty());
    }
}
