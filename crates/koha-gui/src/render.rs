//! Render menu ke buffer piksel. Tidak tahu apa pun soal jendela: menerima
//! `&mut [u32]` (format `0x00RRGGBB`, sama dengan yang dipakai `softbuffer`),
//! sehingga bisa dites hanya dengan memeriksa isi buffer.
//!
//! Langkah ini baru menggambar blok warna; teks menyusul di langkah berikutnya.

use koha_core::{MenuState, Rgb};

use crate::layout::{Layout, Rect};

/// Menggambar menu.
///
/// `width` dan `height` adalah ukuran **buffer** yang sebenarnya. Ini bisa
/// berbeda sesaat dari ukuran `layout` ketika jendela sedang di-resize; semua
/// gambar dipotong (clip) ke buffer, jadi hasilnya tidak pernah menulis di luar
/// batas dan tidak panic.
pub fn render(buffer: &mut [u32], width: u32, height: u32, layout: &Layout, menu: &MenuState) {
    let theme = menu.theme();

    // Bezel: seluruh area dulu, baris digambar di atasnya.
    buffer.fill(theme.bezel.to_u32());

    for (index, rect) in layout.rows.iter().enumerate() {
        let color = if index == menu.selected() {
            theme.sel_bg
        } else {
            theme.bg
        };
        fill_rect(buffer, width, height, *rect, color);
    }
}

/// Mengisi `rect` dengan `color`, dipotong ke ukuran buffer `width` x `height`.
fn fill_rect(buffer: &mut [u32], width: u32, height: u32, rect: Rect, color: Rgb) {
    let width = width as usize;
    if width == 0 {
        return;
    }
    // Buffer yang lebih pendek dari `height` x `width` tidak dipercaya begitu saja.
    let height = (height as usize).min(buffer.len() / width);

    let x0 = (rect.x as usize).min(width);
    let x1 = (rect.x as usize + rect.w as usize).min(width);
    let y0 = (rect.y as usize).min(height);
    let y1 = (rect.y as usize + rect.h as usize).min(height);

    let value = color.to_u32();
    for y in y0..y1 {
        buffer[y * width + x0..y * width + x1].fill(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use koha_core::{Config, DEFAULT_CONFIG, Input, State};

    fn menu() -> MenuState {
        MenuState::new(
            Config::from_toml_str(DEFAULT_CONFIG).unwrap(),
            State::default(),
        )
    }

    /// Menggambar pada skala 1.0 dan mengembalikan (buffer, layout, menu).
    fn drawn(menu: MenuState) -> (Vec<u32>, Layout, MenuState) {
        let layout = Layout::new(menu.rows().len(), 1.0);
        let mut buffer = vec![0xDEAD_BEEF; (layout.width * layout.height) as usize];
        render(&mut buffer, layout.width, layout.height, &layout, &menu);
        (buffer, layout, menu)
    }

    fn pixel(buffer: &[u32], layout: &Layout, x: u32, y: u32) -> u32 {
        buffer[(y * layout.width + x) as usize]
    }

    #[test]
    fn corners_are_bezel_colored() {
        let (buffer, layout, menu) = drawn(menu());
        let bezel = menu.theme().bezel.to_u32();
        assert_eq!(pixel(&buffer, &layout, 0, 0), bezel);
        assert_eq!(
            pixel(&buffer, &layout, layout.width - 1, layout.height - 1),
            bezel
        );
    }

    #[test]
    fn first_row_is_selected_color_and_others_are_normal() {
        let (buffer, layout, menu) = drawn(menu());
        let theme = menu.theme();
        let center = |row: usize| {
            let r = layout.rows[row];
            pixel(&buffer, &layout, r.x + r.w / 2, r.y + r.h / 2)
        };
        assert_eq!(center(0), theme.sel_bg.to_u32());
        assert_eq!(center(1), theme.bg.to_u32());
    }

    #[test]
    fn selection_follows_navigation() {
        let mut menu = menu();
        menu.update(Input::Down);
        let (buffer, layout, menu) = drawn(menu);
        let theme = menu.theme();
        let r0 = layout.rows[0];
        let r1 = layout.rows[1];
        assert_eq!(
            pixel(&buffer, &layout, r0.x + 1, r0.y + 1),
            theme.bg.to_u32()
        );
        assert_eq!(
            pixel(&buffer, &layout, r1.x + 1, r1.y + 1),
            theme.sel_bg.to_u32()
        );
    }

    #[test]
    fn margin_between_window_edge_and_rows_stays_bezel() {
        let (buffer, layout, menu) = drawn(menu());
        let bezel = menu.theme().bezel.to_u32();
        let first = layout.rows[0];
        assert_eq!(pixel(&buffer, &layout, first.x - 1, first.y + 3), bezel);
        assert_eq!(pixel(&buffer, &layout, first.x + 3, first.y - 1), bezel);
    }

    #[test]
    fn every_pixel_is_overwritten() {
        // Nilai awal 0xDEADBEEF tidak boleh tersisa.
        let (buffer, _layout, _menu) = drawn(menu());
        assert!(buffer.iter().all(|&p| p != 0xDEAD_BEEF));
    }

    #[test]
    fn theme_change_changes_colors() {
        let mut menu = menu();
        // Buka pemilih tema (baris pertama di config contoh), pilih game_boy.
        menu.update(Input::Enter);
        assert_eq!(menu.update(Input::Enter), koha_core::Effect::StateChanged);
        assert_eq!(menu.theme().name, "game_boy");
        let (buffer, layout, menu) = drawn(menu);
        assert_eq!(pixel(&buffer, &layout, 0, 0), 0x000F_380F);
        assert_eq!(menu.theme().bezel.to_u32(), 0x000F_380F);
    }

    #[test]
    fn layout_larger_than_buffer_is_clipped_without_panicking() {
        let menu = menu();
        let layout = Layout::new(menu.rows().len(), 2.0); // layout 2x
        let mut buffer = vec![0u32; 100 * 40]; // buffer jauh lebih kecil
        render(&mut buffer, 100, 40, &layout, &menu);
        assert_eq!(buffer.len(), 4000);
    }

    #[test]
    fn buffer_shorter_than_claimed_height_does_not_panic() {
        let menu = menu();
        let layout = Layout::new(menu.rows().len(), 1.0);
        let mut buffer = vec![0u32; 300 * 10];
        // Mengaku tinggi 500, padahal isinya hanya 10 baris piksel.
        render(&mut buffer, 300, 500, &layout, &menu);
    }

    #[test]
    fn zero_sized_buffer_does_not_panic() {
        let menu = menu();
        let layout = Layout::new(menu.rows().len(), 1.0);
        render(&mut [], 0, 0, &layout, &menu);
    }
}
