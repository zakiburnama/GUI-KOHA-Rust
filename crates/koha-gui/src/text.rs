//! Rasterisasi dan penggambaran teks dengan `fontdue`.
//!
//! Hanya satu font yang dimuat pada satu waktu (font tema yang aktif), dan
//! glyph yang sudah dirasterisasi disimpan di cache sampai font atau ukuran
//! berganti.

use std::collections::HashMap;

use fontdue::{Font, FontSettings, Metrics};
use koha_core::Rgb;
use thiserror::Error;

use crate::canvas::Canvas;
use crate::fonts::{FontSpec, spec_for};
use crate::layout::Rect;

#[derive(Debug, Error)]
pub enum TextError {
    #[error("font \"{id}\" tidak bisa dibaca: {reason}")]
    Parse {
        id: &'static str,
        reason: &'static str,
    },
}

/// Satu glyph yang sudah dirasterisasi: metrik dan peta cakupan 0-255.
struct Glyph {
    metrics: Metrics,
    coverage: Vec<u8>,
}

pub struct TextRenderer {
    spec: &'static FontSpec,
    font: Font,
    scale: f64,
    px: f32,
    glyphs: HashMap<char, Glyph>,
}

fn load(spec: &'static FontSpec) -> Result<Font, TextError> {
    Font::from_bytes(spec.bytes, FontSettings::default()).map_err(|reason| TextError::Parse {
        id: spec.id,
        reason,
    })
}

impl TextRenderer {
    /// `font_id` yang tidak dikenal memakai font bawaan.
    pub fn new(font_id: &str, scale: f64) -> Result<Self, TextError> {
        let spec = spec_for(font_id);
        Ok(Self {
            spec,
            font: load(spec)?,
            scale,
            px: spec.pixel_size(scale),
            glyphs: HashMap::new(),
        })
    }

    /// Mengganti font (misalnya saat tema berganti). Tidak melakukan apa-apa
    /// bila font yang diminta sudah aktif, sehingga cache tetap terpakai.
    pub fn set_font(&mut self, font_id: &str) -> Result<(), TextError> {
        let spec = spec_for(font_id);
        if std::ptr::eq(spec, self.spec) {
            return Ok(());
        }
        self.font = load(spec)?;
        self.spec = spec;
        self.px = spec.pixel_size(self.scale);
        self.glyphs.clear();
        Ok(())
    }

    /// Mengubah skala DPI. Cache dibuang hanya bila ukuran piksel berubah.
    pub fn set_scale(&mut self, scale: f64) {
        self.scale = scale;
        let px = self.spec.pixel_size(scale);
        if px != self.px {
            self.px = px;
            self.glyphs.clear();
        }
    }

    pub fn font_id(&self) -> &'static str {
        self.spec.id
    }

    /// Ukuran huruf aktif dalam piksel fisik.
    pub fn pixel_size(&self) -> f32 {
        self.px
    }

    /// Lebar `text` dalam piksel (dibulatkan ke atas).
    pub fn measure(&mut self, text: &str) -> u32 {
        let total: f32 = text
            .chars()
            .map(|ch| self.glyph(ch).metrics.advance_width)
            .sum();
        total.ceil() as u32
    }

    /// Posisi garis dasar (baseline) agar teks berada di tengah vertikal `row`.
    pub fn baseline(&self, row: Rect) -> i32 {
        let (ascent, descent) = self.vertical_metrics();
        let text_height = ascent - descent;
        (row.y as f32 + (row.h as f32 - text_height) / 2.0 + ascent).round() as i32
    }

    /// Menggambar `text` mulai dari `x` dengan garis dasar di `baseline`,
    /// dipotong ke `clip` (dan ke kanvas).
    pub fn draw(
        &mut self,
        canvas: &mut Canvas,
        clip: Rect,
        x: i32,
        baseline: i32,
        text: &str,
        color: Rgb,
    ) {
        let clip_x = i64::from(clip.x)..i64::from(clip.x) + i64::from(clip.w);
        let clip_y = i64::from(clip.y)..i64::from(clip.y) + i64::from(clip.h);

        let mut pen = x as f32;
        for ch in text.chars() {
            let glyph = self.glyph(ch);
            let metrics = glyph.metrics;
            // `ymin` = jarak dasar glyph di bawah (negatif) atau di atas garis dasar.
            let left = pen.round() as i64 + i64::from(metrics.xmin);
            let top = i64::from(baseline) - i64::from(metrics.ymin) - metrics.height as i64;

            for row in 0..metrics.height {
                let y = top + row as i64;
                if !clip_y.contains(&y) {
                    continue;
                }
                for col in 0..metrics.width {
                    let px = left + col as i64;
                    if clip_x.contains(&px) {
                        let alpha = glyph.coverage[row * metrics.width + col];
                        canvas.blend_pixel(px, y, color, alpha);
                    }
                }
            }
            pen += metrics.advance_width;
        }
    }

    /// (ascent, descent); descent bernilai negatif atau nol.
    fn vertical_metrics(&self) -> (f32, f32) {
        match self.font.horizontal_line_metrics(self.px) {
            Some(lines) => (lines.ascent, lines.descent),
            None => (self.px, 0.0),
        }
    }

    /// Glyph dari cache, dirasterisasi dulu bila belum ada. Karakter kontrol
    /// (misalnya baris baru dalam label) diperlakukan sebagai spasi.
    fn glyph(&mut self, ch: char) -> &Glyph {
        let ch = if ch.is_control() { ' ' } else { ch };
        // `self.font` dan `self.glyphs` adalah field berbeda, jadi closure boleh
        // meminjam yang satu sementara `entry` meminjam yang lain.
        self.glyphs.entry(ch).or_insert_with(|| {
            let (metrics, coverage) = self.font.rasterize(ch, self.px);
            Glyph { metrics, coverage }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fonts::FONTS;
    use koha_core::theme::{FONT_IBM_PLEX_MONO, FONT_PRESS_START_2P, FONT_VT323};

    const FG: Rgb = Rgb::new(255, 255, 255);
    const BG: u32 = 0x0000_0000;

    fn renderer(id: &str, scale: f64) -> TextRenderer {
        TextRenderer::new(id, scale).unwrap()
    }

    /// Menggambar `text` ke kanvas `w` x `h` dengan clip seluas kanvas.
    fn draw_full(
        renderer: &mut TextRenderer,
        w: u32,
        h: u32,
        x: i32,
        baseline: i32,
        text: &str,
    ) -> Vec<u32> {
        let mut buffer = vec![BG; (w * h) as usize];
        let mut canvas = Canvas::new(&mut buffer, w, h);
        let clip = Rect { x: 0, y: 0, w, h };
        renderer.draw(&mut canvas, clip, x, baseline, text, FG);
        buffer
    }

    fn painted(buffer: &[u32]) -> usize {
        buffer.iter().filter(|&&p| p != BG).count()
    }

    #[test]
    fn creates_a_renderer_for_every_bundled_font() {
        for spec in &FONTS {
            let renderer = renderer(spec.id, 1.0);
            assert_eq!(renderer.font_id(), spec.id);
        }
    }

    #[test]
    fn unknown_font_uses_the_default() {
        assert_eq!(renderer("nope", 1.0).font_id(), FONT_IBM_PLEX_MONO);
    }

    #[test]
    fn empty_text_has_zero_width() {
        assert_eq!(renderer(FONT_VT323, 1.0).measure(""), 0);
    }

    #[test]
    fn press_start_2p_is_16px_wide_per_character_at_100_percent() {
        let mut r = renderer(FONT_PRESS_START_2P, 1.0);
        assert_eq!(r.pixel_size(), 16.0);
        assert_eq!(r.measure("abc"), 48);
    }

    #[test]
    fn vt323_advance_is_whole_pixels_at_its_target_sizes() {
        let mut r = renderer(FONT_VT323, 1.0);
        assert_eq!(r.measure("abcd"), 32); // 20 px -> 8 px per karakter
        let mut r = renderer(FONT_VT323, 1.25);
        assert_eq!(r.measure("abcd"), 40); // 25 px -> 10 px per karakter
    }

    #[test]
    fn width_grows_with_text_length() {
        let mut r = renderer(FONT_IBM_PLEX_MONO, 1.0);
        assert!(r.measure("ab") < r.measure("abc"));
    }

    #[test]
    fn monospace_fonts_measure_all_characters_equally() {
        for id in [FONT_PRESS_START_2P, FONT_VT323, FONT_IBM_PLEX_MONO] {
            let mut r = renderer(id, 1.0);
            assert_eq!(r.measure("iiii"), r.measure("WWWW"), "{id}");
        }
    }

    #[test]
    fn control_characters_measure_like_spaces() {
        let mut r = renderer(FONT_IBM_PLEX_MONO, 1.0);
        assert_eq!(r.measure("\n\t"), r.measure("  "));
    }

    #[test]
    fn missing_glyph_does_not_panic() {
        let mut r = renderer(FONT_PRESS_START_2P, 1.0);
        let _ = r.measure("漢字🙂");
        let buffer = draw_full(&mut r, 200, 40, 0, 30, "漢字🙂");
        assert_eq!(buffer.len(), 200 * 40);
    }

    #[test]
    fn drawing_paints_pixels_and_press_start_uses_the_exact_color() {
        let mut r = renderer(FONT_PRESS_START_2P, 1.0);
        let buffer = draw_full(&mut r, 100, 30, 2, 24, "Lock");
        assert!(painted(&buffer) > 20);
        // Font pixel yang tajam: setiap piksel yang tergambar bernilai persis FG.
        assert!(buffer.iter().all(|&p| p == BG || p == FG.to_u32()));
    }

    #[test]
    fn smooth_fonts_produce_intermediate_shades() {
        let mut r = renderer(FONT_IBM_PLEX_MONO, 1.0);
        let buffer = draw_full(&mut r, 100, 30, 2, 22, "Lock");
        assert!(buffer.iter().any(|&p| p != BG && p != FG.to_u32()));
    }

    #[test]
    fn spaces_paint_nothing() {
        let mut r = renderer(FONT_VT323, 1.0);
        assert_eq!(painted(&draw_full(&mut r, 100, 30, 2, 22, "     ")), 0);
    }

    #[test]
    fn nothing_is_painted_outside_the_clip_rect() {
        let mut r = renderer(FONT_PRESS_START_2P, 1.0);
        let (w, h) = (200u32, 40u32);
        let mut buffer = vec![BG; (w * h) as usize];
        let clip = Rect {
            x: 20,
            y: 10,
            w: 60,
            h: 20,
        };
        let mut canvas = Canvas::new(&mut buffer, w, h);
        r.draw(&mut canvas, clip, 0, 28, "Close All Windows", FG);
        assert!(painted(&buffer) > 0);
        for (i, &p) in buffer.iter().enumerate() {
            if p != BG {
                let (x, y) = (i as u32 % w, i as u32 / w);
                assert!(
                    x >= clip.x && x < clip.x + clip.w && y >= clip.y && y < clip.y + clip.h,
                    "piksel di luar clip: ({x}, {y})"
                );
            }
        }
    }

    #[test]
    fn extreme_positions_and_tiny_canvases_do_not_panic() {
        let mut r = renderer(FONT_IBM_PLEX_MONO, 1.0);
        for (x, baseline) in [
            (i32::MIN / 2, 0),
            (i32::MAX / 2, 0),
            (-50, -50),
            (0, i32::MAX / 2),
        ] {
            let mut buffer = vec![BG; 10];
            let mut canvas = Canvas::new(&mut buffer, 5, 2);
            let clip = Rect {
                x: 0,
                y: 0,
                w: u32::MAX / 4,
                h: u32::MAX / 4,
            };
            r.draw(&mut canvas, clip, x, baseline, "Hello", FG);
        }
        let mut empty: Vec<u32> = Vec::new();
        let mut canvas = Canvas::new(&mut empty, 0, 0);
        r.draw(
            &mut canvas,
            Rect {
                x: 0,
                y: 0,
                w: 10,
                h: 10,
            },
            0,
            5,
            "x",
            FG,
        );
    }

    #[test]
    fn set_font_switches_metrics() {
        let mut r = renderer(FONT_PRESS_START_2P, 1.0);
        let before = r.measure("abcd");
        r.set_font(FONT_VT323).unwrap();
        assert_eq!(r.font_id(), FONT_VT323);
        assert_ne!(r.measure("abcd"), before);
    }

    #[test]
    fn set_font_to_the_same_font_keeps_the_cache() {
        let mut r = renderer(FONT_VT323, 1.0);
        r.measure("abc");
        let cached = r.glyphs.len();
        assert!(cached > 0);
        r.set_font(FONT_VT323).unwrap();
        assert_eq!(r.glyphs.len(), cached);
    }

    #[test]
    fn set_scale_clears_cache_only_when_size_changes() {
        let mut r = renderer(FONT_PRESS_START_2P, 1.0);
        r.measure("abc");
        r.set_scale(1.25); // tetap 16 px (kelipatan 8)
        assert!(!r.glyphs.is_empty());
        r.set_scale(1.5); // 24 px
        assert!(r.glyphs.is_empty());
        assert_eq!(r.pixel_size(), 24.0);
    }

    #[test]
    fn text_fits_inside_a_row_at_every_scale() {
        // Teks dengan huruf naik (H, l) dan turun (g, j, p, q) tidak boleh
        // terpotong oleh tinggi baris: menggambar dengan clip seluas baris harus
        // menghasilkan piksel yang sama persis dengan clip seluas kanvas.
        let label = "> Hgjpq|()[]";
        for id in [FONT_PRESS_START_2P, FONT_VT323, FONT_IBM_PLEX_MONO] {
            for scale in [1.0, 1.25, 1.5, 2.0] {
                let layout = crate::layout::Layout::new(3, scale);
                let row = layout.rows[1];
                let r = renderer(id, scale);
                let baseline = r.baseline(row);

                let draw = |clip: Rect| {
                    let mut buffer = vec![BG; (layout.width * layout.height) as usize];
                    let mut canvas = Canvas::new(&mut buffer, layout.width, layout.height);
                    let mut r = renderer(id, scale);
                    r.draw(&mut canvas, clip, row.x as i32 + 4, baseline, label, FG);
                    buffer
                };
                let whole = Rect {
                    x: 0,
                    y: 0,
                    w: layout.width,
                    h: layout.height,
                };
                let unclipped = draw(whole);
                let in_row = draw(row);
                assert!(painted(&unclipped) > 0, "{id} @ {scale}");
                // Kotak batas piksel yang tergambar, untuk pesan gagal yang berguna.
                let (mut top, mut bottom, mut left, mut right) = (u32::MAX, 0, u32::MAX, 0);
                for (i, &p) in unclipped.iter().enumerate() {
                    if p != BG {
                        let (x, y) = (i as u32 % layout.width, i as u32 / layout.width);
                        (top, bottom) = (top.min(y), bottom.max(y));
                        (left, right) = (left.min(x), right.max(x));
                    }
                }
                assert!(
                    unclipped == in_row,
                    "{id} @ {scale}: teks keluar dari baris. piksel y={top}..={bottom} x={left}..={right},                      baris y={}..{} x={}..{}, baseline={baseline}, px={}",
                    row.y,
                    row.y + row.h,
                    row.x,
                    row.x + row.w,
                    r.pixel_size()
                );
            }
        }
    }

    // ---- snapshot ASCII ----

    /// Menggambar `text` putih di atas hitam pada skala 1.0, memangkas ke kotak
    /// batas tinta, lalu mengubahnya menjadi seni ASCII ('#' = cakupan >= 50%).
    fn ascii_art(font_id: &str, text: &str) -> String {
        let mut r = renderer(font_id, 1.0);
        let (w, h) = (r.measure(text) + 8, (r.pixel_size() * 2.0).ceil() as u32);
        let baseline = r.baseline(Rect { x: 0, y: 0, w, h });
        let buffer = draw_full(&mut r, w, h, 4, baseline, text);

        let ink = |x: u32, y: u32| buffer[(y * w + x) as usize] & 0xFF >= 0x80;
        let any = |x: u32, y: u32| buffer[(y * w + x) as usize] != BG;
        let rows: Vec<u32> = (0..h).filter(|&y| (0..w).any(|x| any(x, y))).collect();
        let cols: Vec<u32> = (0..w).filter(|&x| (0..h).any(|y| any(x, y))).collect();
        let (top, bottom) = (rows[0], *rows.last().unwrap());
        let (left, right) = (cols[0], *cols.last().unwrap());
        (top..=bottom)
            .map(|y| {
                (left..=right)
                    .map(|x| if ink(x, y) { '#' } else { '.' })
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Membandingkan dengan berkas di `tests/snapshots/`. Jalankan dengan
    /// `UPDATE_SNAPSHOTS=1` untuk menulis ulang berkas acuan setelah perubahan
    /// yang disengaja (misalnya `fontdue` atau font berganti versi).
    fn assert_snapshot(name: &str, actual: &str) {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/snapshots");
        let path = format!("{dir}/{name}.txt");
        if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
            std::fs::create_dir_all(dir).unwrap();
            std::fs::write(&path, format!("{actual}\n")).unwrap();
            return;
        }
        let expected = std::fs::read_to_string(&path).unwrap_or_else(|_| {
            panic!("snapshot {path} belum ada; jalankan sekali dengan UPDATE_SNAPSHOTS=1")
        });
        // Git di Windows bisa mengubah akhir baris menjadi CRLF.
        assert_eq!(
            expected.replace("\r\n", "\n").trim_end(),
            actual.trim_end(),
            "snapshot {name} berubah"
        );
    }

    #[test]
    fn snapshot_press_start_2p() {
        assert_snapshot(
            "press-start-2p",
            &ascii_art(FONT_PRESS_START_2P, "> Sleep gy"),
        );
    }

    #[test]
    fn snapshot_vt323() {
        assert_snapshot("vt323", &ascii_art(FONT_VT323, "> Sleep gy"));
    }

    #[test]
    fn snapshot_ibm_plex_mono() {
        assert_snapshot(
            "ibm-plex-mono",
            &ascii_art(FONT_IBM_PLEX_MONO, "> Sleep gy"),
        );
    }

    #[test]
    fn baseline_is_inside_the_row() {
        for id in [FONT_PRESS_START_2P, FONT_VT323, FONT_IBM_PLEX_MONO] {
            for scale in [1.0, 1.25, 1.5, 2.0] {
                let layout = crate::layout::Layout::new(2, scale);
                let row = layout.rows[0];
                let baseline = renderer(id, scale).baseline(row);
                assert!(
                    baseline > row.y as i32 && baseline < (row.y + row.h) as i32,
                    "{id} @ {scale}"
                );
            }
        }
    }
}
