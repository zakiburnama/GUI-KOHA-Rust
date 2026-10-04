//! Ke mana pesan error ditampilkan.
//!
//! Binary rilis tidak punya jendela konsol. Bila dijalankan dari terminal, pesan
//! ke stderr terlihat; bila dijalankan dari tombol atau pintasan (misalnya Lenovo
//! Vantage), tidak ada yang melihat stderr, jadi pesan ditampilkan sebagai dialog.

use std::io::Write;

/// Judul dialog error.
pub const TITLE: &str = "KOHA";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sink {
    Stderr,
    Dialog,
}

/// Ada konsol berarti ada yang membaca stderr; tanpa konsol, pakai dialog.
pub fn choose_sink(has_console: bool) -> Sink {
    if has_console {
        Sink::Stderr
    } else {
        Sink::Dialog
    }
}

/// Mengirim `message` ke tujuan yang dipilih. `stderr` dan `dialog` dioper
/// sebagai parameter supaya bisa dites tanpa menampilkan dialog sungguhan
/// (yang menunggu klik pengguna).
pub fn emit(sink: Sink, message: &str, stderr: &mut dyn Write, dialog: &mut dyn FnMut(&str, &str)) {
    let message = message.trim_end();
    match sink {
        // Gagal menulis ke stderr tidak bisa dilaporkan ke mana-mana lagi.
        Sink::Stderr => {
            let _ = writeln!(stderr, "{message}");
        }
        Sink::Dialog => dialog(TITLE, message),
    }
}

/// Melaporkan error fatal ke tujuan yang tepat untuk keadaan saat ini.
pub fn error(has_console: bool, message: &str) {
    emit(
        choose_sink(has_console),
        message,
        &mut std::io::stderr(),
        &mut |title, body| koha_platform::show_error_dialog(title, body),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_console_means_stderr_and_no_console_means_a_dialog() {
        assert_eq!(choose_sink(true), Sink::Stderr);
        assert_eq!(choose_sink(false), Sink::Dialog);
    }

    /// Menjalankan `emit` dan mengembalikan (teks stderr, panggilan dialog).
    fn run(sink: Sink, message: &str) -> (String, Vec<(String, String)>) {
        let mut stderr = Vec::new();
        let mut dialogs = Vec::new();
        emit(sink, message, &mut stderr, &mut |title, body| {
            dialogs.push((title.to_owned(), body.to_owned()));
        });
        (String::from_utf8(stderr).unwrap(), dialogs)
    }

    #[test]
    fn stderr_sink_writes_the_message_and_shows_no_dialog() {
        let (stderr, dialogs) = run(Sink::Stderr, "config rusak");
        assert_eq!(stderr, "config rusak\n");
        assert!(dialogs.is_empty());
    }

    #[test]
    fn dialog_sink_shows_a_titled_dialog_and_writes_nothing_to_stderr() {
        let (stderr, dialogs) = run(Sink::Dialog, "config rusak");
        assert_eq!(stderr, "");
        assert_eq!(dialogs, [("KOHA".to_owned(), "config rusak".to_owned())]);
    }

    #[test]
    fn trailing_newlines_are_trimmed_so_there_is_no_blank_line_in_the_dialog() {
        let (_, dialogs) = run(Sink::Dialog, "baris satu\nbaris dua\n\n");
        assert_eq!(dialogs[0].1, "baris satu\nbaris dua");
    }

    #[test]
    fn multi_line_messages_are_kept_intact() {
        let message = "error: x\n --> config.toml:2:1\n  |\n2 | bogus = 1\n  | ^";
        let (stderr, _) = run(Sink::Stderr, message);
        assert_eq!(stderr, format!("{message}\n"));
    }
}
