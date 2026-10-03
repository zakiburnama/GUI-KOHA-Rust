//! Model tema dan kumpulan tema bawaan.

use crate::rgb::Rgb;

/// Nama tema yang dipakai bila tema yang diminta tidak ada.
pub const DEFAULT_THEME: &str = "amber";

/// Identitas font bergaya retro (pixel/bitmap).
pub const FONT_RETRO: &str = "retro";
/// Identitas font monospace biasa.
pub const FONT_MONO: &str = "mono";

/// Satu tema. Setiap tema membawa font-nya sendiri.
///
/// `font` hanya nama identitas; file font yang sebenarnya dipilih dan dimuat
/// oleh `koha-gui` (langkah 5). Core tidak tahu apa-apa soal berkas font.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Theme {
    pub name: String,
    /// Latar dan teks normal.
    pub bg: Rgb,
    pub fg: Rgb,
    /// Latar dan teks item terpilih (reverse-video).
    pub sel_bg: Rgb,
    pub sel_fg: Rgb,
    /// Warna jendela di sekeliling daftar item.
    pub bezel: Rgb,
    pub font: String,
}

/// Kumpulan tema, urutannya dipertahankan (dipakai untuk urutan di pemilih tema).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeSet {
    themes: Vec<Theme>,
}

/// Bentuk ringkas satu baris tabel tema bawaan.
/// `&'static str` = teks yang hidup selama program (literal di dalam biner).
struct Builtin {
    name: &'static str,
    bg: u32,
    fg: u32,
    sel_bg: u32,
    sel_fg: u32,
    bezel: u32,
    font: &'static str,
}

/// Warna dipindahkan 1:1 dari `THEMES` di `koha.ahk`.
const BUILTIN: [Builtin; 7] = [
    Builtin {
        name: "game_boy",
        bg: 0x9BBC0F,
        fg: 0x0F380F,
        sel_bg: 0x0F380F,
        sel_fg: 0x9BBC0F,
        bezel: 0x0F380F,
        font: FONT_RETRO,
    },
    Builtin {
        name: "amber",
        bg: 0x1A0F00,
        fg: 0xFFB000,
        sel_bg: 0xFFB000,
        sel_fg: 0x1A0F00,
        bezel: 0xFFB000,
        font: FONT_RETRO,
    },
    Builtin {
        name: "green_term",
        bg: 0x0A0A0A,
        fg: 0x33FF33,
        sel_bg: 0x33FF33,
        sel_fg: 0x0A0A0A,
        bezel: 0x33FF33,
        font: FONT_RETRO,
    },
    Builtin {
        name: "catppuccin-mocha",
        bg: 0x1E1E2E,
        fg: 0xCDD6F4,
        sel_bg: 0xCBA6F7,
        sel_fg: 0x1E1E2E,
        bezel: 0x11111B,
        font: FONT_MONO,
    },
    Builtin {
        name: "gruvbox",
        bg: 0x282828,
        fg: 0xEBDBB2,
        sel_bg: 0xFE8019,
        sel_fg: 0x282828,
        bezel: 0x1D2021,
        font: FONT_MONO,
    },
    Builtin {
        name: "vague",
        bg: 0x141415,
        fg: 0xCDCDCD,
        sel_bg: 0x6E94B2,
        sel_fg: 0x141415,
        bezel: 0x1C1C24,
        font: FONT_MONO,
    },
    Builtin {
        name: "tokyonight",
        bg: 0x1A1B26,
        fg: 0xC0CAF5,
        sel_bg: 0x7AA2F7,
        sel_fg: 0x1A1B26,
        bezel: 0x0C0E14,
        font: FONT_MONO,
    },
];

impl From<&Builtin> for Theme {
    fn from(b: &Builtin) -> Self {
        Self {
            name: b.name.to_owned(),
            bg: Rgb::from_u32(b.bg),
            fg: Rgb::from_u32(b.fg),
            sel_bg: Rgb::from_u32(b.sel_bg),
            sel_fg: Rgb::from_u32(b.sel_fg),
            bezel: Rgb::from_u32(b.bezel),
            font: b.font.to_owned(),
        }
    }
}

impl ThemeSet {
    /// Tujuh tema bawaan, urutan sama dengan `THEME_NAMES` di AHK.
    pub fn builtin() -> Self {
        Self {
            themes: BUILTIN.iter().map(Theme::from).collect(),
        }
    }

    /// Menambah tema, atau mengganti tema bernama sama di posisinya semula.
    /// Dipakai nanti saat config menambah/menimpa tema bawaan.
    pub fn add_or_replace(&mut self, theme: Theme) {
        match self.themes.iter_mut().find(|t| t.name == theme.name) {
            Some(existing) => *existing = theme,
            None => self.themes.push(theme),
        }
    }

    /// Mencari tema menurut nama. Mengembalikan `Option<&Theme>`: pinjaman
    /// (referensi) ke tema di dalam set, tanpa menyalinnya.
    pub fn get(&self, name: &str) -> Option<&Theme> {
        self.themes.iter().find(|t| t.name == name)
    }

    /// Seperti [`get`](Self::get) tetapi jatuh ke [`DEFAULT_THEME`] bila nama
    /// tidak dikenal (perilaku AHK untuk tema yang tersimpan tapi sudah hilang).
    ///
    /// Jika set ini tidak memuat tema bawaan sama sekali (tidak bisa terjadi
    /// lewat `builtin()`), tema pertama dipakai.
    pub fn get_or_default(&self, name: &str) -> &Theme {
        self.get(name)
            .or_else(|| self.get(DEFAULT_THEME))
            .unwrap_or(&self.themes[0])
    }

    /// Semua tema sesuai urutan.
    pub fn iter(&self) -> impl Iterator<Item = &Theme> {
        self.themes.iter()
    }

    pub fn len(&self) -> usize {
        self.themes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.themes.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_has_seven_themes_in_ahk_order() {
        let names: Vec<_> = ThemeSet::builtin().iter().map(|t| t.name.clone()).collect();
        assert_eq!(
            names,
            [
                "game_boy",
                "amber",
                "green_term",
                "catppuccin-mocha",
                "gruvbox",
                "vague",
                "tokyonight"
            ]
        );
    }

    #[test]
    fn builtin_names_are_unique() {
        let set = ThemeSet::builtin();
        for theme in set.iter() {
            assert_eq!(set.iter().filter(|t| t.name == theme.name).count(), 1);
        }
    }

    #[test]
    fn default_theme_exists_in_builtin() {
        assert!(ThemeSet::builtin().get(DEFAULT_THEME).is_some());
    }

    #[test]
    fn amber_colors_match_ahk() {
        let set = ThemeSet::builtin();
        let amber = set.get("amber").unwrap();
        assert_eq!(amber.bg, Rgb::new(0x1A, 0x0F, 0x00));
        assert_eq!(amber.fg, Rgb::new(0xFF, 0xB0, 0x00));
        assert_eq!(amber.sel_bg, amber.fg);
        assert_eq!(amber.sel_fg, amber.bg);
        assert_eq!(amber.bezel, Rgb::new(0xFF, 0xB0, 0x00));
    }

    #[test]
    fn retro_themes_use_retro_font() {
        let set = ThemeSet::builtin();
        for name in ["game_boy", "amber", "green_term"] {
            assert_eq!(set.get(name).unwrap().font, FONT_RETRO, "{name}");
        }
        assert_eq!(set.get("gruvbox").unwrap().font, FONT_MONO);
    }

    #[test]
    fn unknown_theme_falls_back_to_default() {
        let set = ThemeSet::builtin();
        assert!(set.get("nope").is_none());
        assert_eq!(set.get_or_default("nope").name, DEFAULT_THEME);
        assert_eq!(set.get_or_default("gruvbox").name, "gruvbox");
    }

    #[test]
    fn add_or_replace_appends_new_theme() {
        let mut set = ThemeSet::builtin();
        let mut custom = set.get("amber").unwrap().clone();
        custom.name = "custom".to_owned();
        set.add_or_replace(custom);
        assert_eq!(set.len(), 8);
        assert_eq!(set.iter().last().unwrap().name, "custom");
    }

    #[test]
    fn add_or_replace_overrides_in_place() {
        let mut set = ThemeSet::builtin();
        let mut changed = set.get("gruvbox").unwrap().clone();
        changed.fg = Rgb::new(1, 2, 3);
        set.add_or_replace(changed);
        assert_eq!(set.len(), 7);
        assert_eq!(set.get("gruvbox").unwrap().fg, Rgb::new(1, 2, 3));
        // posisi tidak berubah: gruvbox tetap urutan ke-5
        assert_eq!(set.iter().nth(4).unwrap().name, "gruvbox");
    }
}
