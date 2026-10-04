//! Jejak waktu start, untuk mencari ke mana waktu habis antara `main` dan
//! gambar pertama.
//!
//! Hanya aktif dengan fitur Cargo `startup-trace` (mati secara bawaan). Tanpa
//! fitur itu [`start`] dan [`mark`] adalah fungsi kosong bertanda `#[inline(always)]`
//! sehingga kompiler menghapus pemanggilannya, dan biaya di rilis normal nol.
//!
//! Dengan fitur aktif, tiap [`mark`] mencetak satu baris ke stderr:
//!
//! ```text
//! [trace]    1.234 ms  config loaded
//! ```
//!
//! Waktu dihitung sejak [`start`] dipanggil (awal `main`), jadi tidak mencakup
//! waktu OS memuat proses sebelum `main`. Selisih itu terlihat dengan
//! membandingkannya dengan hasil `tools/bench-startup.ps1`.

#[cfg(feature = "startup-trace")]
mod enabled {
    use std::sync::OnceLock;
    use std::time::{Duration, Instant};

    static START: OnceLock<Instant> = OnceLock::new();

    /// Menetapkan titik nol. Hanya panggilan pertama yang berlaku.
    pub fn start() {
        let _ = START.set(Instant::now());
    }

    /// Mencatat tahap `stage`. Tidak mencetak apa pun bila [`start`] belum dipanggil.
    pub fn mark(stage: &str) {
        if let Some(start) = START.get() {
            eprintln!("{}", format_line(start.elapsed(), stage));
        }
    }

    /// Format baris jejak. Dipisah agar bisa dites, dan diurai oleh
    /// `tools/bench-startup.ps1` (awalan `[trace]`, angka milidetik, `ms`, tahap).
    pub(super) fn format_line(elapsed: Duration, stage: &str) -> String {
        format!(
            "[trace] {:>9.3} ms  {stage}",
            elapsed.as_secs_f64() * 1000.0
        )
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn lines_have_a_stable_parsable_format() {
            let line = format_line(Duration::from_micros(1234), "config loaded");
            assert_eq!(line, "[trace]     1.234 ms  config loaded");
        }

        #[test]
        fn large_values_keep_three_decimals() {
            let line = format_line(Duration::from_millis(250), "x");
            assert_eq!(line, "[trace]   250.000 ms  x");
        }

        #[test]
        fn mark_before_start_prints_nothing_and_does_not_panic() {
            // `START` bersifat global untuk seluruh proses test, jadi hanya
            // memastikan tidak panic bila belum diatur.
            mark("sebelum start");
        }
    }
}

#[cfg(feature = "startup-trace")]
pub use enabled::{mark, start};

/// Fitur mati: fungsi kosong yang dihapus kompiler.
#[cfg(not(feature = "startup-trace"))]
#[inline(always)]
pub fn start() {}

/// Fitur mati: fungsi kosong yang dihapus kompiler.
#[cfg(not(feature = "startup-trace"))]
#[inline(always)]
pub fn mark(_stage: &str) {}

#[cfg(all(test, not(feature = "startup-trace")))]
mod tests {
    use super::*;

    #[test]
    fn disabled_trace_is_callable_and_silent() {
        start();
        mark("apa pun");
    }
}
