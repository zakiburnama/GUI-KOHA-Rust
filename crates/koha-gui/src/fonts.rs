//! Font yang dibundel dan ukuran tampilnya.
//!
//! Semua font berlisensi SIL Open Font License; teks lisensinya ada di
//! `assets/fonts/<nama>/OFL.txt` dan harus tetap menyertai distribusi.
//! File font ditanam ke dalam biner dengan `include_bytes!`, jadi tidak ada
//! berkas yang dibaca saat program jalan.

use koha_core::theme::{DEFAULT_FONT, FONT_IBM_PLEX_MONO, FONT_PRESS_START_2P, FONT_VT323};

pub struct FontSpec {
    /// Sama dengan `Theme::font` di `koha-core`.
    pub id: &'static str,
    /// `&'static [u8]`: irisan byte yang hidup selama program (ada di dalam biner).
    pub bytes: &'static [u8],
    /// Ukuran huruf yang dituju, dalam piksel logis (sebelum dikali skala DPI).
    pub logical_px: f32,
    /// Font pixel hanya tajam pada kelipatan bulat ukuran grid aslinya (dalam
    /// piksel). `None` untuk font outline biasa.
    pub grid: Option<u32>,
}

pub const FONTS: [FontSpec; 3] = [
    FontSpec {
        id: FONT_PRESS_START_2P,
        bytes: include_bytes!("../assets/fonts/press-start-2p/PressStart2P-Regular.ttf"),
        // Tajam sempurna pada kelipatan 8 px (diukur: 0% piksel setengah-transparan).
        logical_px: 16.0,
        grid: Some(8),
    },
    FontSpec {
        id: FONT_VT323,
        bytes: include_bytes!("../assets/fonts/vt323/VT323-Regular.ttf"),
        // Bukan bitmap sejati: tepinya halus di semua ukuran. 20 px membuat
        // lebar karakter tepat 8,0 px (0,4 em), dan 25 px tepat 10,0 px.
        logical_px: 20.0,
        grid: None,
    },
    FontSpec {
        id: FONT_IBM_PLEX_MONO,
        bytes: include_bytes!("../assets/fonts/ibm-plex-mono/IBMPlexMono-Regular.ttf"),
        logical_px: 15.0,
        grid: None,
    },
];

/// Mencari font menurut id. Id yang tidak dikenal jatuh ke [`DEFAULT_FONT`].
pub fn spec_for(id: &str) -> &'static FontSpec {
    FONTS
        .iter()
        .find(|spec| spec.id == id)
        .or_else(|| FONTS.iter().find(|spec| spec.id == DEFAULT_FONT))
        .unwrap_or(&FONTS[0])
}

impl FontSpec {
    /// Ukuran huruf dalam piksel fisik untuk skala DPI `scale`.
    ///
    /// Font pixel dipaku ke kelipatan bulat (dibulatkan ke bawah, minimal 1x)
    /// dari grid-nya agar tetap tajam; font lain dibulatkan ke piksel terdekat.
    pub fn pixel_size(&self, scale: f64) -> f32 {
        let scale = if scale.is_finite() && scale > 0.0 {
            scale
        } else {
            1.0
        };
        let target = f64::from(self.logical_px) * scale;
        match self.grid {
            Some(grid) => {
                let grid = f64::from(grid);
                ((target / grid).floor().max(1.0) * grid) as f32
            }
            None => target.round() as f32,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fontdue::{Font, FontSettings};
    use koha_core::ThemeSet;

    #[test]
    fn every_bundled_font_parses() {
        for spec in &FONTS {
            Font::from_bytes(spec.bytes, FontSettings::default())
                .unwrap_or_else(|e| panic!("{}: {e}", spec.id));
        }
    }

    #[test]
    fn font_ids_are_unique_and_include_the_default() {
        for spec in &FONTS {
            assert_eq!(FONTS.iter().filter(|s| s.id == spec.id).count(), 1);
        }
        assert!(FONTS.iter().any(|s| s.id == DEFAULT_FONT));
    }

    #[test]
    fn every_builtin_theme_font_is_bundled() {
        for theme in ThemeSet::builtin().iter() {
            assert_eq!(spec_for(&theme.font).id, theme.font, "{}", theme.name);
        }
    }

    #[test]
    fn unknown_font_falls_back_to_default() {
        assert_eq!(spec_for("comic-sans").id, DEFAULT_FONT);
        assert_eq!(spec_for("").id, DEFAULT_FONT);
    }

    #[test]
    fn pixel_font_snaps_to_multiples_of_its_grid() {
        let spec = spec_for(FONT_PRESS_START_2P);
        assert_eq!(spec.pixel_size(1.0), 16.0);
        assert_eq!(spec.pixel_size(1.25), 16.0); // 20 -> kelipatan 8 ke bawah
        assert_eq!(spec.pixel_size(1.5), 24.0);
        assert_eq!(spec.pixel_size(2.0), 32.0);
        assert_eq!(spec.pixel_size(0.25), 8.0); // minimal 1x
    }

    #[test]
    fn outline_font_rounds_to_nearest_pixel() {
        assert_eq!(spec_for(FONT_IBM_PLEX_MONO).pixel_size(1.0), 15.0);
        assert_eq!(spec_for(FONT_IBM_PLEX_MONO).pixel_size(1.25), 19.0); // 18.75
        assert_eq!(spec_for(FONT_VT323).pixel_size(1.25), 25.0);
    }

    #[test]
    fn invalid_scale_behaves_like_100_percent() {
        let spec = spec_for(FONT_IBM_PLEX_MONO);
        for scale in [0.0, -2.0, f64::NAN, f64::INFINITY] {
            assert_eq!(spec.pixel_size(scale), spec.pixel_size(1.0));
        }
    }

    #[test]
    fn press_start_2p_is_crisp_at_every_snapped_size() {
        // Menjaga hasil pengukuran: pada kelipatan 8 px tidak ada piksel
        // setengah-transparan, jadi teks tampil seperti bitmap sungguhan.
        let spec = spec_for(FONT_PRESS_START_2P);
        let font = Font::from_bytes(spec.bytes, FontSettings::default()).unwrap();
        for scale in [1.0, 1.5, 2.0, 3.0] {
            let px = spec.pixel_size(scale);
            for ch in "HEgAm0&> Close All Windows".chars() {
                let (_, bitmap) = font.rasterize(ch, px);
                assert!(
                    bitmap.iter().all(|&c| c == 0 || c == 255),
                    "{ch:?} @ {px}px tidak tajam"
                );
            }
        }
    }
}
