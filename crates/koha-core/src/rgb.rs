//! Warna 24-bit (`Rgb`) dan parsernya.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer};
use thiserror::Error;

/// Warna RGB 8-bit per kanal.
///
/// `Copy` artinya nilai ini disalin (bukan dipindahkan) saat dioper atau
/// ditugaskan; wajar untuk tipe sekecil 3 byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// Penyebab gagalnya mengurai teks menjadi [`Rgb`].
///
/// `#[derive(Error)]` dari `thiserror` membuat `impl std::error::Error` dan
/// `Display` dari atribut `#[error("...")]`, tanpa boilerplate manual.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ParseRgbError {
    #[error("warna harus 6 digit hex (RRGGBB), dapat {0} karakter")]
    InvalidLength(usize),
    #[error("warna hanya boleh berisi digit hex 0-9 dan A-F")]
    InvalidDigit,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Dari angka `0xRRGGBB`. `const fn` supaya bisa dipakai di tabel `const`.
    pub const fn from_u32(rgb: u32) -> Self {
        Self::new((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
    }

    /// Ke angka `0x00RRGGBB`, format piksel yang dipakai `softbuffer`.
    pub const fn to_u32(self) -> u32 {
        ((self.r as u32) << 16) | ((self.g as u32) << 8) | self.b as u32
    }
}

/// Mengimplementasi `FromStr` membuat `"#9BBC0F".parse::<Rgb>()` bekerja.
impl FromStr for Rgb {
    type Err = ParseRgbError;

    /// Menerima `RRGGBB` atau `#RRGGBB`, huruf besar atau kecil.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let hex = s.strip_prefix('#').unwrap_or(s);
        if hex.len() != 6 {
            return Err(ParseRgbError::InvalidLength(hex.chars().count()));
        }
        // `from_str_radix` menerima tanda "+", jadi digit dicek manual dulu.
        // Cek ini juga menjamin string ASCII, sehingga slicing per byte di
        // bawah tidak bisa memotong karakter multibyte (yang akan panic).
        if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(ParseRgbError::InvalidDigit);
        }
        let channel = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16);
        match (channel(0), channel(2), channel(4)) {
            (Ok(r), Ok(g), Ok(b)) => Ok(Self::new(r, g, b)),
            _ => Err(ParseRgbError::InvalidDigit),
        }
    }
}

/// Di config, warna ditulis sebagai string (`bg = "#1A0F00"`). Dengan
/// `Deserialize` ini kesalahan penulisan warna ikut mendapat posisi baris dan
/// kolom dari parser TOML, sama seperti kesalahan sintaks lainnya.
impl<'de> Deserialize<'de> for Rgb {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

impl fmt::Display for Rgb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_with_and_without_hash() {
        assert_eq!("9BBC0F".parse(), Ok(Rgb::new(0x9B, 0xBC, 0x0F)));
        assert_eq!("#9BBC0F".parse(), Ok(Rgb::new(0x9B, 0xBC, 0x0F)));
    }

    #[test]
    fn parses_lowercase() {
        assert_eq!("#ffb000".parse(), Ok(Rgb::new(0xFF, 0xB0, 0x00)));
    }

    #[test]
    fn rejects_wrong_length() {
        assert_eq!("".parse::<Rgb>(), Err(ParseRgbError::InvalidLength(0)));
        assert_eq!("FFF".parse::<Rgb>(), Err(ParseRgbError::InvalidLength(3)));
        assert_eq!(
            "#FFFFFFF".parse::<Rgb>(),
            Err(ParseRgbError::InvalidLength(7))
        );
    }

    #[test]
    fn rejects_non_hex_digits() {
        assert_eq!("GGGGGG".parse::<Rgb>(), Err(ParseRgbError::InvalidDigit));
    }

    #[test]
    fn rejects_plus_sign_that_from_str_radix_would_accept() {
        assert_eq!("+1+2+3".parse::<Rgb>(), Err(ParseRgbError::InvalidDigit));
    }

    #[test]
    fn rejects_multibyte_characters_without_panicking() {
        // "ééé" = 6 byte tetapi 3 karakter non-ASCII: lolos cek panjang, harus
        // ditolak oleh cek digit (bukan panic saat slicing di tengah karakter).
        assert_eq!("ééé".parse::<Rgb>(), Err(ParseRgbError::InvalidDigit));
        // "éé€" = 7 byte: ditolak oleh cek panjang, dilaporkan dalam karakter.
        assert_eq!("éé€".parse::<Rgb>(), Err(ParseRgbError::InvalidLength(3)));
    }

    #[test]
    fn display_roundtrips() {
        let color = Rgb::new(0x1A, 0x0F, 0x00);
        assert_eq!(color.to_string(), "#1A0F00");
        assert_eq!(color.to_string().parse(), Ok(color));
    }

    #[test]
    fn u32_conversion_roundtrips() {
        let color = Rgb::from_u32(0x33FF33);
        assert_eq!(color, Rgb::new(0x33, 0xFF, 0x33));
        assert_eq!(color.to_u32(), 0x0033_FF33);
    }
}
