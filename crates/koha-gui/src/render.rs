//! Render menu ke buffer piksel. Tidak tahu apa pun soal jendela: menerima
//! `&mut [u32]` (format `0x00RRGGBB`, sama dengan yang dipakai `softbuffer`),
//! sehingga bisa dites hanya dengan memeriksa isi buffer.

use koha_core::{MenuState, Row};

use crate::canvas::Canvas;
use crate::layout::Layout;
use crate::text::TextRenderer;

/// Teks sebuah baris: kursor `> ` untuk baris terpilih, dua spasi untuk yang
/// lain (seperti menu AHK), sehingga semua label sejajar.
pub fn row_text(label: &str, selected: bool) -> String {
    let prefix = if selected { "> " } else { "  " };
    format!("{prefix}{label}")
}

/// Lebar teks terpanjang di antara `rows` (piksel). Awalan `> ` dan dua spasi
/// sama lebar pada font monospace, jadi cukup mengukur salah satunya.
pub fn widest_row(text: &mut TextRenderer, rows: &[Row]) -> u32 {
    rows.iter()
        .map(|row| text.measure(&row_text(&row.label, true)))
        .max()
        .unwrap_or(0)
}

/// Menggambar menu.
///
/// `width` dan `height` adalah ukuran **buffer** yang sebenarnya. Ini bisa
/// berbeda sesaat dari ukuran `layout` ketika jendela sedang di-resize; semua
/// gambar dipotong (clip) ke buffer, jadi hasilnya tidak pernah menulis di luar
/// batas dan tidak panic.
///
/// `text` harus sudah memakai font tema aktif (`menu.theme().font`); pemanggil
/// yang mengaturnya, karena ukuran jendela juga bergantung pada font itu.
pub fn render(
    buffer: &mut [u32],
    width: u32,
    height: u32,
    layout: &Layout,
    menu: &MenuState,
    text: &mut TextRenderer,
) {
    let theme = menu.theme();
    let mut canvas = Canvas::new(buffer, width, height);

    // Bezel: seluruh area dulu, baris digambar di atasnya.
    canvas.clear(theme.bezel);

    for (index, (rect, row)) in layout.rows.iter().zip(menu.rows()).enumerate() {
        let selected = index == menu.selected();
        let (background, foreground) = if selected {
            (theme.sel_bg, theme.sel_fg)
        } else {
            (theme.bg, theme.fg)
        };
        canvas.fill_rect(*rect, background);
        text.draw(
            &mut canvas,
            *rect,
            rect.x as i32 + layout.text_pad as i32,
            text.baseline(*rect),
            &row_text(&row.label, selected),
            foreground,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use koha_core::{Config, DEFAULT_CONFIG, Effect, Input, Rgb, State};

    fn menu() -> MenuState {
        MenuState::new(
            Config::from_toml_str(DEFAULT_CONFIG).unwrap(),
            State::default(),
        )
    }

    /// Config contoh + tema "probe" berfont pixel yang tajam dan berwarna
    /// beda-beda, supaya warna teks bisa dicocokkan persis.
    fn probe_menu() -> MenuState {
        let source = format!(
            "{DEFAULT_CONFIG}\n[[themes]]\nname = \"probe\"\nbg = \"#101010\"\nfg = \"#FFFFFF\"\n\
             sel_bg = \"#0000FF\"\nsel_fg = \"#FFFF00\"\nbezel = \"#800000\"\nfont = \"press-start-2p\"\n"
        );
        let state = State {
            theme: Some("probe".to_owned()),
            ..State::default()
        };
        MenuState::new(Config::from_toml_str(&source).unwrap(), state)
    }

    /// Menggambar pada skala 1.0 dengan font tema aktif.
    fn drawn(menu: MenuState) -> (Vec<u32>, Layout, MenuState) {
        let mut text = TextRenderer::new(&menu.theme().font, 1.0).unwrap();
        let rows = menu.rows();
        let layout = Layout::fitting(rows.len(), 1.0, widest_row(&mut text, &rows), u32::MAX);
        let mut buffer = vec![0xDEAD_BEEF; (layout.width * layout.height) as usize];
        render(
            &mut buffer,
            layout.width,
            layout.height,
            &layout,
            &menu,
            &mut text,
        );
        (buffer, layout, menu)
    }

    fn pixel(buffer: &[u32], layout: &Layout, x: u32, y: u32) -> u32 {
        buffer[(y * layout.width + x) as usize]
    }

    /// Piksel di ujung kanan baris: jauh dari teks, jadi pasti warna latar baris.
    fn row_background(buffer: &[u32], layout: &Layout, row: usize) -> u32 {
        let r = layout.rows[row];
        pixel(buffer, layout, r.x + r.w - 2, r.y + r.h / 2)
    }

    fn count_in_row(buffer: &[u32], layout: &Layout, row: usize, color: Rgb) -> usize {
        let r = layout.rows[row];
        let mut count = 0;
        for y in r.y..r.y + r.h {
            for x in r.x..r.x + r.w {
                if pixel(buffer, layout, x, y) == color.to_u32() {
                    count += 1;
                }
            }
        }
        count
    }

    #[test]
    fn row_text_marks_only_the_selected_row() {
        assert_eq!(row_text("Lock", true), "> Lock");
        assert_eq!(row_text("Lock", false), "  Lock");
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
    fn first_row_background_is_selected_color_and_others_are_normal() {
        let (buffer, layout, menu) = drawn(menu());
        let theme = menu.theme();
        assert_eq!(row_background(&buffer, &layout, 0), theme.sel_bg.to_u32());
        assert_eq!(row_background(&buffer, &layout, 1), theme.bg.to_u32());
    }

    #[test]
    fn selection_follows_navigation() {
        let mut menu = menu();
        menu.update(Input::Down);
        let (buffer, layout, menu) = drawn(menu);
        let theme = menu.theme();
        assert_eq!(row_background(&buffer, &layout, 0), theme.bg.to_u32());
        assert_eq!(row_background(&buffer, &layout, 1), theme.sel_bg.to_u32());
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
    fn selected_row_text_uses_selection_colors_and_others_use_normal_colors() {
        let (buffer, layout, menu) = drawn(probe_menu());
        let theme = menu.theme();
        assert_eq!(theme.name, "probe");

        // Baris 0 terpilih: teks kuning, tidak ada teks putih.
        assert!(count_in_row(&buffer, &layout, 0, theme.sel_fg) > 20);
        assert_eq!(count_in_row(&buffer, &layout, 0, theme.fg), 0);
        // Baris 1 biasa: teks putih, tidak ada teks kuning.
        assert!(count_in_row(&buffer, &layout, 1, theme.fg) > 20);
        assert_eq!(count_in_row(&buffer, &layout, 1, theme.sel_fg), 0);
    }

    #[test]
    fn only_the_selected_row_has_ink_in_the_prefix_cell() {
        // Sel awalan "> " adalah dua karakter 16 px: baris terpilih punya ">"
        // di sana, baris lain hanya spasi.
        let (buffer, layout, menu) = drawn(probe_menu());
        let theme = menu.theme();
        let ink_in_prefix = |row: usize, color: Rgb| {
            let r = layout.rows[row];
            let left = r.x + layout.text_pad;
            let mut ink = 0;
            for y in r.y..r.y + r.h {
                for x in left..left + 32 {
                    if pixel(&buffer, &layout, x, y) == color.to_u32() {
                        ink += 1;
                    }
                }
            }
            ink
        };
        assert!(
            ink_in_prefix(0, theme.sel_fg) > 5,
            "kursor > tidak tergambar"
        );
        assert_eq!(ink_in_prefix(1, theme.fg), 0);
    }

    #[test]
    fn window_widens_to_fit_the_widest_label() {
        let menu = probe_menu();
        let mut text = TextRenderer::new("press-start-2p", 1.0).unwrap();
        let rows = menu.rows();
        let text_width = widest_row(&mut text, &rows);
        let layout = Layout::fitting(rows.len(), 1.0, text_width, u32::MAX);
        // "> Close All Windows" = 19 x 16 px = 304 px: melebihi lebar minimum.
        assert!(text_width >= 304, "{text_width}");
        assert!(layout.width > 300);
        let row = layout.rows[0];
        assert!(text_width + 2 * layout.text_pad <= row.w);
    }

    #[test]
    fn theme_change_changes_colors() {
        let mut menu = menu();
        // Buka pemilih tema (baris pertama di config contoh), pilih game_boy.
        menu.update(Input::Enter);
        assert_eq!(menu.update(Input::Enter), Effect::StateChanged);
        assert_eq!(menu.theme().name, "game_boy");
        let (buffer, layout, menu) = drawn(menu);
        assert_eq!(pixel(&buffer, &layout, 0, 0), 0x000F_380F);
        assert_eq!(menu.theme().bezel.to_u32(), 0x000F_380F);
    }

    #[test]
    fn layout_larger_than_buffer_is_clipped_without_panicking() {
        let menu = menu();
        let mut text = TextRenderer::new(&menu.theme().font, 2.0).unwrap();
        let layout = Layout::new(menu.rows().len(), 2.0); // layout 2x
        let mut buffer = vec![0u32; 100 * 40]; // buffer jauh lebih kecil
        render(&mut buffer, 100, 40, &layout, &menu, &mut text);
        assert_eq!(buffer.len(), 4000);
    }

    #[test]
    fn buffer_shorter_than_claimed_height_does_not_panic() {
        let menu = menu();
        let mut text = TextRenderer::new(&menu.theme().font, 1.0).unwrap();
        let layout = Layout::new(menu.rows().len(), 1.0);
        let mut buffer = vec![0u32; 300 * 10];
        // Mengaku tinggi 500, padahal isinya hanya 10 baris piksel.
        render(&mut buffer, 300, 500, &layout, &menu, &mut text);
    }

    #[test]
    fn zero_sized_buffer_does_not_panic() {
        let menu = menu();
        let mut text = TextRenderer::new(&menu.theme().font, 1.0).unwrap();
        let layout = Layout::new(menu.rows().len(), 1.0);
        render(&mut [], 0, 0, &layout, &menu, &mut text);
    }
}
