//! Aturan jendela mana yang boleh ditutup oleh "Close All Windows".
//!
//! Murni: bekerja atas [`WindowInfo`] (data hasil pengumpulan), jadi aturannya
//! bisa dites tanpa membuka atau menutup jendela sungguhan.

/// `WS_EX_TOOLWINDOW`: jendela alat (palet kecil), tidak tampil di Alt+Tab.
pub const WS_EX_TOOLWINDOW: u32 = 0x0000_0080;
/// `WS_EX_APPWINDOW`: memaksa jendela tampil di taskbar dan Alt+Tab.
pub const WS_EX_APPWINDOW: u32 = 0x0004_0000;

/// Kelas jendela milik shell Windows yang tidak boleh ditutup.
pub const SHELL_CLASSES: [&str; 4] = [
    "Progman",
    "WorkerW",
    "Shell_TrayWnd",
    "Shell_SecondaryTrayWnd",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowInfo {
    /// Handle jendela (`HWND`) sebagai angka.
    pub hwnd: isize,
    pub visible: bool,
    pub has_owner: bool,
    pub ex_style: u32,
    pub title_len: i32,
    /// Jendela "hantu" aplikasi UWP yang disembunyikan DWM.
    pub cloaked: bool,
    pub pid: u32,
    pub class: String,
}

/// Mengikuti kriteria Alt+Tab: jendela yang tampil sebagai aplikasi biasa bagi
/// pengguna, kecuali milik KOHA sendiri (`own_pid`) dan milik shell.
pub fn is_closable(window: &WindowInfo, own_pid: u32) -> bool {
    // `APPWINDOW` mengalahkan "punya pemilik" dan "tool window".
    let listed_like_alt_tab = window.ex_style & WS_EX_APPWINDOW != 0
        || (!window.has_owner && window.ex_style & WS_EX_TOOLWINDOW == 0);

    window.visible
        && listed_like_alt_tab
        && window.title_len > 0
        && !window.cloaked
        && window.pid != own_pid
        && !SHELL_CLASSES.contains(&window.class.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    const OWN: u32 = 4242;

    /// Jendela aplikasi biasa yang memenuhi semua syarat; tiap test mengubah satu hal.
    fn app_window() -> WindowInfo {
        WindowInfo {
            hwnd: 1,
            visible: true,
            has_owner: false,
            ex_style: 0,
            title_len: 12,
            cloaked: false,
            pid: 100,
            class: "Notepad".to_owned(),
        }
    }

    #[test]
    fn an_ordinary_application_window_is_closable() {
        assert!(is_closable(&app_window(), OWN));
    }

    #[test]
    fn invisible_windows_are_left_alone() {
        let window = WindowInfo {
            visible: false,
            ..app_window()
        };
        assert!(!is_closable(&window, OWN));
    }

    #[test]
    fn windows_without_a_title_are_left_alone() {
        let window = WindowInfo {
            title_len: 0,
            ..app_window()
        };
        assert!(!is_closable(&window, OWN));
    }

    #[test]
    fn cloaked_uwp_ghost_windows_are_left_alone() {
        let window = WindowInfo {
            cloaked: true,
            ..app_window()
        };
        assert!(!is_closable(&window, OWN));
    }

    #[test]
    fn koha_never_closes_its_own_window() {
        let window = WindowInfo {
            pid: OWN,
            ..app_window()
        };
        assert!(!is_closable(&window, OWN));
    }

    #[test]
    fn owned_dialogs_are_left_alone() {
        let window = WindowInfo {
            has_owner: true,
            ..app_window()
        };
        assert!(!is_closable(&window, OWN));
    }

    #[test]
    fn tool_windows_are_left_alone() {
        let window = WindowInfo {
            ex_style: WS_EX_TOOLWINDOW,
            ..app_window()
        };
        assert!(!is_closable(&window, OWN));
    }

    #[test]
    fn appwindow_style_overrides_owner_and_toolwindow() {
        let window = WindowInfo {
            has_owner: true,
            ex_style: WS_EX_TOOLWINDOW | WS_EX_APPWINDOW,
            ..app_window()
        };
        assert!(is_closable(&window, OWN));
    }

    #[test]
    fn shell_windows_are_never_closed_even_if_they_look_like_apps() {
        for class in SHELL_CLASSES {
            let window = WindowInfo {
                class: class.to_owned(),
                ..app_window()
            };
            assert!(!is_closable(&window, OWN), "{class}");
        }
    }

    #[test]
    fn appwindow_style_does_not_override_the_other_exclusions() {
        // Tidak terlihat, tanpa judul, cloaked, milik sendiri: tetap ditolak.
        for window in [
            WindowInfo {
                visible: false,
                ex_style: WS_EX_APPWINDOW,
                ..app_window()
            },
            WindowInfo {
                title_len: 0,
                ex_style: WS_EX_APPWINDOW,
                ..app_window()
            },
            WindowInfo {
                cloaked: true,
                ex_style: WS_EX_APPWINDOW,
                ..app_window()
            },
            WindowInfo {
                pid: OWN,
                ex_style: WS_EX_APPWINDOW,
                ..app_window()
            },
        ] {
            assert!(!is_closable(&window, OWN), "{window:?}");
        }
    }
}
