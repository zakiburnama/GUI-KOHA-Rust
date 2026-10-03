//! Kanvas piksel: membungkus buffer `&mut [u32]` (format `0x00RRGGBB`) dengan
//! ukurannya, dan menjamin semua penulisan dipotong ke batas buffer.

use koha_core::Rgb;

use crate::layout::Rect;

pub struct Canvas<'a> {
    buffer: &'a mut [u32],
    width: usize,
    height: usize,
}

impl<'a> Canvas<'a> {
    /// `width` dan `height` adalah ukuran yang diklaim pemanggil. Buffer yang
    /// lebih pendek dari itu tidak dipercaya: tinggi dipotong ke jumlah baris
    /// piksel yang benar-benar ada, jadi tidak ada penulisan di luar batas.
    pub fn new(buffer: &'a mut [u32], width: u32, height: u32) -> Self {
        let width = width as usize;
        let height = (height as usize).min(buffer.len().checked_div(width).unwrap_or(0));
        Self {
            buffer,
            width,
            height,
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    /// Mengisi seluruh kanvas.
    pub fn clear(&mut self, color: Rgb) {
        self.buffer.fill(color.to_u32());
    }

    /// Mengisi `rect` dengan `color`, dipotong ke kanvas.
    pub fn fill_rect(&mut self, rect: Rect, color: Rgb) {
        let x0 = (rect.x as usize).min(self.width);
        let x1 = (rect.x as usize + rect.w as usize).min(self.width);
        let y0 = (rect.y as usize).min(self.height);
        let y1 = (rect.y as usize + rect.h as usize).min(self.height);
        let value = color.to_u32();
        for y in y0..y1 {
            self.buffer[y * self.width + x0..y * self.width + x1].fill(value);
        }
    }

    /// Mencampur `color` ke piksel (x, y) dengan opasitas `alpha` (0 = tidak
    /// berubah, 255 = menimpa penuh). Di luar kanvas diabaikan.
    pub fn blend_pixel(&mut self, x: i64, y: i64, color: Rgb, alpha: u8) {
        if alpha == 0 || x < 0 || y < 0 {
            return;
        }
        let (x, y) = (x as usize, y as usize);
        if x >= self.width || y >= self.height {
            return;
        }
        let pixel = &mut self.buffer[y * self.width + x];
        *pixel = blend(*pixel, color, alpha);
    }
}

/// Campuran linear per kanal: `dst * (1 - a) + src * a`, dibulatkan.
fn blend(dst: u32, src: Rgb, alpha: u8) -> u32 {
    let a = u32::from(alpha);
    let channel = |d: u32, s: u8| (u32::from(s) * a + d * (255 - a) + 127) / 255;
    let r = channel((dst >> 16) & 0xFF, src.r);
    let g = channel((dst >> 8) & 0xFF, src.g);
    let b = channel(dst & 0xFF, src.b);
    (r << 16) | (g << 8) | b
}

#[cfg(test)]
mod tests {
    use super::*;

    const RED: Rgb = Rgb::new(255, 0, 0);

    #[test]
    fn blend_with_zero_alpha_keeps_destination() {
        assert_eq!(blend(0x0012_3456, RED, 0), 0x0012_3456);
    }

    #[test]
    fn blend_with_full_alpha_replaces_destination() {
        assert_eq!(blend(0x0012_3456, RED, 255), 0x00FF_0000);
    }

    #[test]
    fn blend_half_alpha_is_midway() {
        // 0 -> 255 di alpha 128: (255*128 + 0 + 127)/255 = 128
        assert_eq!(blend(0x0000_0000, RED, 128), 0x0080_0000);
    }

    #[test]
    fn blend_never_overflows_a_channel() {
        for alpha in 0..=255u8 {
            let out = blend(0x00FF_FFFF, Rgb::new(255, 255, 255), alpha);
            assert_eq!(out, 0x00FF_FFFF, "alpha {alpha}");
        }
    }

    #[test]
    fn fill_rect_is_clipped_to_the_canvas() {
        let mut buffer = vec![0u32; 4 * 3];
        let mut canvas = Canvas::new(&mut buffer, 4, 3);
        canvas.fill_rect(
            Rect {
                x: 2,
                y: 1,
                w: 100,
                h: 100,
            },
            RED,
        );
        assert_eq!(
            buffer,
            [
                0,
                0,
                0,
                0,
                0,
                0,
                RED.to_u32(),
                RED.to_u32(),
                0,
                0,
                RED.to_u32(),
                RED.to_u32()
            ]
        );
    }

    #[test]
    fn blend_pixel_outside_the_canvas_is_ignored() {
        let mut buffer = vec![7u32; 4];
        let mut canvas = Canvas::new(&mut buffer, 2, 2);
        for (x, y) in [
            (-1, 0),
            (0, -1),
            (2, 0),
            (0, 2),
            (i64::MAX, 0),
            (i64::MIN, i64::MIN),
        ] {
            canvas.blend_pixel(x, y, RED, 255);
        }
        assert_eq!(buffer, [7; 4]);
    }

    #[test]
    fn claimed_height_larger_than_buffer_is_trimmed() {
        let mut buffer = vec![0u32; 4 * 2];
        let canvas = Canvas::new(&mut buffer, 4, 500);
        assert_eq!(canvas.height(), 2);
    }

    #[test]
    fn zero_width_canvas_is_empty_and_safe() {
        let mut buffer: Vec<u32> = Vec::new();
        let mut canvas = Canvas::new(&mut buffer, 0, 0);
        canvas.fill_rect(
            Rect {
                x: 0,
                y: 0,
                w: 5,
                h: 5,
            },
            RED,
        );
        canvas.blend_pixel(0, 0, RED, 255);
        assert_eq!((canvas.width(), canvas.height()), (0, 0));
    }
}
