//! Implementasi Windows, memakai crate `windows` (bukan `winapi`).
//!
//! Hampir semua pemanggilan Win32 bersifat `unsafe`, karena kompiler tidak bisa
//! memeriksa kontrak pointer dan handle-nya. Tiap blok `unsafe` di sini dibuat
//! sekecil mungkin dan diberi komentar `// SAFETY:` yang menjelaskan mengapa
//! pemanggilannya aman.

use std::ffi::c_void;
use std::mem::size_of;
use std::sync::Once;

use windows::Win32::Foundation::{ERROR_CANCELLED, GENERIC_WRITE, HWND, LPARAM, WPARAM};
use windows::Win32::Graphics::Dwm::{DWMWA_CLOAKED, DwmGetWindowAttribute};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows::Win32::System::Com::{
    COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoInitializeEx,
};
use windows::Win32::System::Console::{
    ATTACH_PARENT_PROCESS, AttachConsole, GetConsoleWindow, GetStdHandle, STD_ERROR_HANDLE,
    STD_HANDLE, STD_OUTPUT_HANDLE, SetStdHandle,
};
use windows::Win32::System::Power::SetSuspendState;
use windows::Win32::System::Shutdown::LockWorkStation;
use windows::Win32::System::Threading::GetCurrentProcessId;
use windows::Win32::UI::Shell::{
    SEE_MASK_DOENVSUBST, SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC, SHELLEXECUTEINFOW, ShellExecuteExW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GW_OWNER, GWL_EXSTYLE, GetClassNameW, GetWindow, GetWindowLongW,
    GetWindowTextLengthW, GetWindowThreadProcessId, IsWindowVisible, MB_ICONERROR, MB_OK,
    MB_SETFOREGROUND, MB_TOPMOST, MessageBoxW, PostMessageW, SW_SHOWNORMAL, WM_CLOSE,
};
use windows::core::{BOOL, HSTRING, PCWSTR, w};

use crate::window_filter::{WindowInfo, is_closable};
use crate::{Platform, PlatformError, cmdline};

#[derive(Debug, Default, Clone, Copy)]
pub struct WindowsPlatform;

impl Platform for WindowsPlatform {
    fn launch(&self, command: &str, args: &[String], admin: bool) -> Result<(), PlatformError> {
        // "runas" meminta elevasi (UAC); KOHA sendiri tidak ber-elevasi.
        let verb = if admin { w!("runas") } else { w!("open") };
        let params = cmdline::join_args(args);
        let params = (!params.is_empty()).then_some(params);
        shell_execute("launch", verb, command, params.as_deref())
    }

    fn open_url(&self, url: &str) -> Result<(), PlatformError> {
        // Divalidasi lagi di sini agar pemanggil langsung (tanpa `execute`)
        // tidak bisa menyuruh ShellExecute menjalankan jalur berkas.
        let url = cmdline::validate_url(url)?;
        shell_execute("open_url", w!("open"), url, None)
    }

    fn lock(&self) -> Result<(), PlatformError> {
        // SAFETY: `LockWorkStation` tidak punya parameter dan tidak menyentuh
        // memori milik kita.
        unsafe { LockWorkStation() }.map_err(|error| failed("lock", &error))
    }

    fn sleep(&self) -> Result<(), PlatformError> {
        // Sama dengan AHK: DllCall("PowrProf\SetSuspendState", 0, 0, 0)
        // = tidur biasa (bukan hibernasi), tanpa paksa, wake event aktif.
        //
        // Keterbatasan: API ini dirancang untuk tidur klasik (S3) dan hibernasi.
        // Di PC yang hanya mendukung Modern Standby dan hibernasinya mati (lihat
        // `powercfg /a`), pemanggilan ini tidak menidurkan apa pun. Terbukti di
        // satu laptop (juga untuk `rundll32 powrprof.dll,SetSuspendState`), jadi
        // item "sleep" tidak disertakan di config contoh.
        // SAFETY: semua argumen adalah nilai bool biasa.
        let accepted = unsafe { SetSuspendState(false, false, false) };
        if accepted {
            Ok(())
        } else {
            Err(failed("sleep", &windows::core::Error::from_thread()))
        }
    }

    fn close_all_windows(&self) -> Result<(), PlatformError> {
        for window in closable_windows() {
            // `PostMessage` hanya menaruh WM_CLOSE di antrean jendela dan
            // langsung kembali. Aplikasi yang menampilkan "simpan perubahan?"
            // tidak membuat KOHA macet, dan tidak ada yang ditutup paksa.
            let hwnd = HWND(window.hwnd as *mut c_void);
            // SAFETY: `hwnd` berasal dari EnumWindows barusan; handle yang sudah
            // tidak valid hanya membuat PostMessageW mengembalikan error, yang
            // sengaja diabaikan (jendela itu memang sudah hilang).
            let _ = unsafe { PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)) };
        }
        Ok(())
    }
}

/// Jendela yang akan ditutup oleh `close_all_windows`, tanpa menutup apa pun.
/// Dipakai contoh `list_windows` sebagai mode kering.
pub fn closable_windows() -> Vec<WindowInfo> {
    // SAFETY: tanpa argumen, hanya membaca ID proses sendiri.
    let own_pid = unsafe { GetCurrentProcessId() };
    enumerate_windows()
        .into_iter()
        .filter(|window| is_closable(window, own_pid))
        .collect()
}

/// Semua jendela tingkat atas, beserta data yang dibutuhkan filter.
fn enumerate_windows() -> Vec<WindowInfo> {
    let mut windows: Vec<WindowInfo> = Vec::new();
    // SAFETY: `collect` hanya dipanggil selama EnumWindows berjalan, dan
    // `lparam` menunjuk `windows` yang hidup sampai EnumWindows kembali.
    let _ = unsafe {
        EnumWindows(
            Some(collect),
            LPARAM(&mut windows as *mut Vec<WindowInfo> as isize),
        )
    };
    windows
}

/// Callback C untuk `EnumWindows`: mengisi `Vec<WindowInfo>` lewat `lparam`.
unsafe extern "system" fn collect(hwnd: HWND, lparam: LPARAM) -> BOOL {
    // SAFETY: lihat `enumerate_windows`: `lparam` adalah pointer ke Vec yang valid
    // dan hanya diakses dari thread ini selama callback.
    let windows = unsafe { &mut *(lparam.0 as *mut Vec<WindowInfo>) };
    windows.push(inspect(hwnd));
    BOOL(1) // lanjut ke jendela berikutnya
}

fn inspect(hwnd: HWND) -> WindowInfo {
    // SAFETY: semua fungsi di bawah hanya membaca properti `hwnd`; handle yang
    // sudah tidak valid membuatnya mengembalikan nilai kosong/error, bukan crash.
    unsafe {
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));

        let mut class = [0u16; 256];
        let class_len = GetClassNameW(hwnd, &mut class).max(0) as usize;

        let mut cloaked = 0u32;
        let cloaked_known = DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            &mut cloaked as *mut u32 as *mut c_void,
            size_of::<u32>() as u32,
        )
        .is_ok();

        WindowInfo {
            hwnd: hwnd.0 as isize,
            visible: IsWindowVisible(hwnd).as_bool(),
            has_owner: GetWindow(hwnd, GW_OWNER).is_ok_and(|owner| !owner.0.is_null()),
            ex_style: GetWindowLongW(hwnd, GWL_EXSTYLE) as u32,
            title_len: GetWindowTextLengthW(hwnd),
            cloaked: cloaked_known && cloaked != 0,
            pid,
            class: String::from_utf16_lossy(&class[..class_len]),
        }
    }
}

/// `ShellExecuteExW` dengan opsi yang kita butuhkan:
/// - `NOASYNC`: selesaikan peluncuran sebelum kembali (KOHA langsung keluar sesudahnya).
/// - `DOENVSUBST`: perluas `%LOCALAPPDATA%` dan sejenisnya di `file`.
/// - `FLAG_NO_UI`: tidak ada kotak pesan error dari shell; error kita laporkan sendiri.
fn shell_execute(
    operation: &'static str,
    verb: PCWSTR,
    file: &str,
    parameters: Option<&str>,
) -> Result<(), PlatformError> {
    init_com();
    // `HSTRING` memiliki buffer UTF-16 berakhiran nol yang harus tetap hidup
    // selama pemanggilan di bawah.
    let file = HSTRING::from(file);
    let parameters = parameters.map(HSTRING::from);

    let mut info = SHELLEXECUTEINFOW {
        cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOASYNC | SEE_MASK_DOENVSUBST | SEE_MASK_FLAG_NO_UI,
        lpVerb: verb,
        lpFile: PCWSTR(file.as_ptr()),
        lpParameters: parameters
            .as_ref()
            .map_or(PCWSTR::null(), |p| PCWSTR(p.as_ptr())),
        nShow: SW_SHOWNORMAL.0,
        ..Default::default()
    };
    // SAFETY: `info` diinisialisasi penuh dengan `cbSize` yang benar, dan
    // `file`/`parameters`/`verb` hidup sampai pemanggilan selesai.
    unsafe { ShellExecuteExW(&mut info) }.map_err(|error| {
        // Menolak prompt UAC menghasilkan ERROR_CANCELLED: itu pilihan pengguna,
        // bukan kegagalan.
        if error.code() == windows::core::HRESULT::from_win32(ERROR_CANCELLED.0) {
            PlatformError::Cancelled
        } else {
            failed(operation, &error)
        }
    })
}

/// Dokumentasi Microsoft meminta COM diinisialisasi sebelum `ShellExecuteEx`,
/// karena shell bisa mendelegasikan ke ekstensi yang memakai COM. Hasil
/// "sudah diinisialisasi" (atau mode berbeda) tidak masalah, jadi diabaikan.
fn init_com() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        // SAFETY: `None` untuk parameter cadangan, flag adalah konstanta resmi.
        let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) };
    });
}

/// Menempel ke konsol milik proses induk bila ada. Mengembalikan `true` bila
/// sekarang ada konsol yang bisa dibaca pengguna.
///
/// Binary rilis memakai subsistem GUI (tanpa jendela konsol), jadi bila dijalankan
/// dari terminal ia tidak punya stdout sendiri dan `--help` atau
/// `--print-config-path` tidak akan mencetak apa pun. Menempel ke konsol induk
/// memperbaikinya. Bila dijalankan dari tombol atau pintasan tidak ada konsol
/// induk, dan hasilnya `false`: pemanggil sebaiknya melaporkan error lewat dialog.
pub fn attach_parent_console() -> bool {
    // SAFETY: kedua fungsi tidak menerima pointer dari kita.
    unsafe {
        // Sudah punya konsol sendiri (build debug berkonsol): `AttachConsole`
        // justru akan gagal, jadi tidak perlu dan tidak boleh dipanggil.
        if !GetConsoleWindow().0.is_null() {
            return true;
        }
        if AttachConsole(ATTACH_PARENT_PROCESS).is_err() {
            return false;
        }
    }
    ensure_console_output();
    true
}

/// Setelah `AttachConsole`, handle stdout/stderr proses GUI bisa masih kosong.
/// Bila begitu, arahkan ke `CONOUT$`. Handle yang sudah valid (misalnya hasil
/// `koha --print-config-path > berkas.txt`) dibiarkan agar pengalihan tetap bekerja.
fn ensure_console_output() {
    for which in [STD_OUTPUT_HANDLE, STD_ERROR_HANDLE] {
        if !std_handle_is_usable(which) {
            // SAFETY: nama berakhiran nol dari `w!`; semua argumen lain adalah
            // konstanta atau `None`. Handle yang dihasilkan sengaja tidak ditutup:
            // ia dipakai sampai proses selesai.
            let console = unsafe {
                CreateFileW(
                    w!("CONOUT$"),
                    GENERIC_WRITE.0,
                    FILE_SHARE_READ | FILE_SHARE_WRITE,
                    None,
                    OPEN_EXISTING,
                    FILE_ATTRIBUTE_NORMAL,
                    None,
                )
            };
            if let Ok(console) = console {
                // SAFETY: `console` adalah handle valid yang baru dibuka.
                let _ = unsafe { SetStdHandle(which, console) };
            }
        }
    }
}

fn std_handle_is_usable(which: STD_HANDLE) -> bool {
    // SAFETY: hanya membaca handle standar proses.
    match unsafe { GetStdHandle(which) } {
        Ok(handle) => !handle.is_invalid() && !handle.0.is_null(),
        Err(_) => false,
    }
}

/// Menampilkan dialog error dan menunggu pengguna menekan OK.
///
/// Dipakai saat tidak ada konsol (dijalankan dari tombol atau pintasan), karena
/// pesan ke stderr tidak akan terlihat siapa pun. `MB_TOPMOST` dan
/// `MB_SETFOREGROUND` membuatnya muncul di depan walau KOHA dipanggil dari tombol.
pub fn show_error_dialog(title: &str, message: &str) {
    let title = HSTRING::from(title);
    let message = HSTRING::from(message);
    // SAFETY: `message` dan `title` hidup sampai pemanggilan kembali; tanpa jendela
    // pemilik (`None`).
    unsafe {
        MessageBoxW(
            None,
            &message,
            &title,
            MB_OK | MB_ICONERROR | MB_SETFOREGROUND | MB_TOPMOST,
        );
    }
}

fn failed(operation: &'static str, error: &windows::core::Error) -> PlatformError {
    PlatformError::Failed {
        operation,
        message: error.message(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launching_a_program_that_does_not_exist_is_an_error_not_a_panic() {
        let result = WindowsPlatform.launch("koha-definitely-not-a-real-program.exe", &[], false);
        assert!(
            matches!(
                result,
                Err(PlatformError::Failed {
                    operation: "launch",
                    ..
                })
            ),
            "{result:?}"
        );
    }

    #[test]
    fn open_url_rejects_a_file_path_without_calling_the_shell() {
        let result = WindowsPlatform.open_url(r"C:\Windows\System32\calc.exe");
        assert!(
            matches!(result, Err(PlatformError::InvalidUrl(_))),
            "{result:?}"
        );
    }

    #[test]
    fn enumerating_windows_does_not_panic_and_reports_sane_values() {
        for window in enumerate_windows() {
            assert!(window.title_len >= 0);
            assert!(window.class.len() <= 256 * 3);
        }
    }

    #[test]
    fn koha_own_process_is_never_in_the_closable_list() {
        // SAFETY: tanpa argumen.
        let own = unsafe { GetCurrentProcessId() };
        assert!(closable_windows().iter().all(|w| w.pid != own));
    }

    #[test]
    fn shell_windows_are_never_in_the_closable_list() {
        use crate::window_filter::SHELL_CLASSES;
        assert!(
            closable_windows()
                .iter()
                .all(|w| !SHELL_CLASSES.contains(&w.class.as_str()))
        );
    }
}
