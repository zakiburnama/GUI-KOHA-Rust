//! Tata letak jendela: murni hitungan, tanpa jendela dan tanpa OS.
//!
//! Ukuran dasar diambil dari AHK dan dinyatakan dalam piksel logis; dikali
//! faktor skala DPI menjadi piksel fisik (yang dipakai buffer `softbuffer`).

/// Lebar jendela (piksel logis).
pub const WIDTH: f64 = 300.0;
/// Tinggi tiap baris (piksel logis).
pub const ROW_HEIGHT: f64 = 26.0;
/// Jarak bezel di sekeliling daftar baris (piksel logis).
pub const MARGIN: f64 = 6.0;

/// Persegi panjang dalam piksel fisik.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// Ukuran jendela dan kotak tiap baris, dalam piksel fisik.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    pub width: u32,
    pub height: u32,
    pub rows: Vec<Rect>,
}

impl Layout {
    /// Menghitung tata letak untuk `row_count` baris pada skala `scale`
    /// (1.0 = 100%, 1.5 = 150%).
    ///
    /// Setiap tepi dibulatkan sendiri-sendiri (bukan "posisi + tinggi"), supaya
    /// baris yang bersebelahan selalu rapat tanpa celah satu piksel pun pada
    /// skala pecahan.
    pub fn new(row_count: usize, scale: f64) -> Self {
        let scale = if scale.is_finite() && scale > 0.0 {
            scale
        } else {
            1.0
        };
        let px = |logical: f64| (logical * scale).round() as u32;

        let x = px(MARGIN);
        let w = px(WIDTH - MARGIN) - x;
        let rows = (0..row_count)
            .map(|i| {
                let top = px(MARGIN + i as f64 * ROW_HEIGHT);
                let bottom = px(MARGIN + (i + 1) as f64 * ROW_HEIGHT);
                Rect {
                    x,
                    y: top,
                    w,
                    h: bottom - top,
                }
            })
            .collect();

        Self {
            width: px(WIDTH),
            height: px(MARGIN * 2.0 + row_count as f64 * ROW_HEIGHT),
            rows,
        }
    }
}

/// Posisi pojok kiri-atas agar jendela berukuran `window` berada tepat di
/// tengah monitor yang berposisi `monitor_pos` dan berukuran `monitor_size`.
/// Semuanya piksel fisik. Dihitung dengan `i64` supaya tidak overflow.
pub fn centered_position(
    monitor_pos: (i32, i32),
    monitor_size: (u32, u32),
    window: (u32, u32),
) -> (i32, i32) {
    let center = |pos: i32, monitor: u32, window: u32| {
        (i64::from(pos) + (i64::from(monitor) - i64::from(window)) / 2) as i32
    };
    (
        center(monitor_pos.0, monitor_size.0, window.0),
        center(monitor_pos.1, monitor_size.1, window.1),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_at_100_percent_matches_ahk() {
        // AHK: w=300, tinggi = baris*26 + margin*2.
        let layout = Layout::new(5, 1.0);
        assert_eq!(layout.width, 300);
        assert_eq!(layout.height, 5 * 26 + 12);
        assert_eq!(layout.rows.len(), 5);
    }

    #[test]
    fn first_row_starts_at_margin() {
        let layout = Layout::new(3, 1.0);
        assert_eq!(
            layout.rows[0],
            Rect {
                x: 6,
                y: 6,
                w: 288,
                h: 26
            }
        );
        assert_eq!(layout.rows[1].y, 32);
    }

    #[test]
    fn scales_with_dpi() {
        let layout = Layout::new(4, 2.0);
        assert_eq!(layout.width, 600);
        assert_eq!(layout.height, (4 * 26 + 12) * 2);
        assert_eq!(layout.rows[0].h, 52);
    }

    #[test]
    fn rows_are_contiguous_and_never_overlap_at_fractional_scales() {
        for scale in [1.0, 1.25, 1.5, 1.75, 2.25] {
            let layout = Layout::new(12, scale);
            for pair in layout.rows.windows(2) {
                assert_eq!(pair[0].y + pair[0].h, pair[1].y, "scale {scale}");
            }
        }
    }

    #[test]
    fn rows_stay_inside_the_window() {
        for scale in [1.0, 1.25, 1.5, 2.0] {
            let layout = Layout::new(7, scale);
            for row in &layout.rows {
                assert!(row.x + row.w <= layout.width, "scale {scale}");
                assert!(row.y + row.h <= layout.height, "scale {scale}");
            }
            // Ada bezel di bawah baris terakhir.
            let last = layout.rows.last().unwrap();
            assert!(last.y + last.h < layout.height, "scale {scale}");
        }
    }

    #[test]
    fn invalid_scale_falls_back_to_100_percent() {
        let reference = Layout::new(3, 1.0);
        for scale in [0.0, -1.5, f64::NAN, f64::INFINITY] {
            assert_eq!(Layout::new(3, scale), reference, "{scale}");
        }
    }

    #[test]
    fn zero_rows_is_just_the_bezel() {
        let layout = Layout::new(0, 1.0);
        assert!(layout.rows.is_empty());
        assert_eq!(layout.height, 12);
    }

    #[test]
    fn centers_on_a_monitor_at_the_origin() {
        assert_eq!(
            centered_position((0, 0), (1920, 1080), (300, 142)),
            (810, 469)
        );
    }

    #[test]
    fn centers_on_a_secondary_monitor_with_offset_origin() {
        // Monitor kedua di kiri monitor utama (posisi negatif).
        assert_eq!(
            centered_position((-1920, 0), (1920, 1080), (300, 142)),
            (-1110, 469)
        );
    }

    #[test]
    fn window_larger_than_monitor_gets_negative_offset_not_overflow() {
        assert_eq!(
            centered_position((0, 0), (200, 100), (300, 142)),
            (-50, -21)
        );
    }
}
