//! State machine navigasi menu. Murni: input masuk, [`Effect`] keluar, tanpa
//! I/O dan tanpa mengenal kode tombol atau jendela.

use crate::config::{Action, Builtin, ItemKind, MenuItem};

/// Input yang sudah diterjemahkan GUI dari tombol fisik.
///
/// Pemetaan tombol ada di GUI, bukan di sini: Tab dan Shift+Tab menjadi
/// `Down` dan `Up`, Enter dan NumpadEnter menjadi `Enter`, Esc menjadi `Back`,
/// sedangkan tombol lain, klik di luar, dan hilangnya fokus menjadi `Dismiss`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input {
    Up,
    Down,
    Enter,
    Back,
    Dismiss,
}

/// Yang harus dilakukan GUI setelah sebuah input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    /// Tidak ada yang berubah.
    None,
    /// Gambar ulang. Jumlah baris bisa berubah, jadi GUI membandingkan
    /// `rows().len()` dengan sebelumnya untuk menghitung ulang tinggi jendela.
    Redraw,
    /// Tutup jendela **dulu**, lalu jalankan aksi, lalu keluar.
    Run(Action),
    /// Tutup tanpa menjalankan apa pun.
    Close,
}

/// Satu baris yang ditampilkan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub label: String,
}

/// State navigasi menu.
///
/// State ini **memiliki** salinan pohon menu dan hanya menyimpan jalur indeks
/// submenu yang sedang dimasuki. Alternatifnya meminjam config
/// (`MenuState<'a>` dengan `&'a [MenuItem]`), tetapi itu memaksa pemanggil
/// menyimpan config dan state yang saling meminjam; di Rust struktur seperti
/// itu (self-referential) sulit dibuat. Satu kali clone pohon kecil saat start
/// adalah harga yang murah untuk menghindarinya.
#[derive(Debug, Clone)]
pub struct MenuState {
    root: Vec<MenuItem>,
    /// Indeks submenu yang dimasuki dari root, berurutan. Kosong = di root.
    path: Vec<usize>,
    selected: usize,
    closed: bool,
}

impl MenuState {
    pub fn new(menu: Vec<MenuItem>) -> Self {
        Self {
            root: menu,
            path: Vec::new(),
            selected: 0,
            closed: false,
        }
    }

    /// Baris di level yang sedang aktif.
    pub fn rows(&self) -> Vec<Row> {
        self.current_items()
            .iter()
            .map(|item| Row {
                label: item.label.clone(),
            })
            .collect()
    }

    /// Indeks baris terpilih di [`rows`](Self::rows).
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// Seberapa dalam submenu yang sedang dibuka (0 = menu utama).
    pub fn depth(&self) -> usize {
        self.path.len()
    }

    /// `true` setelah `Run` atau `Close` dikeluarkan.
    pub fn is_closed(&self) -> bool {
        self.closed
    }

    pub fn update(&mut self, input: Input) -> Effect {
        // Sesudah ditutup, semua input diabaikan. Menutup jendela sendiri bisa
        // memicu event "kehilangan fokus" yang menyusul; tanpa guard ini event
        // itu bisa membatalkan aksi yang baru saja diputuskan (bug klasik AHK).
        if self.closed {
            return Effect::None;
        }
        match input {
            Input::Up => self.move_up(),
            Input::Down => self.move_down(),
            Input::Enter => self.enter(),
            Input::Back => self.back(),
            Input::Dismiss => self.close(),
        }
    }

    /// Daftar item di level aktif, dicari dengan menelusuri `path` dari root.
    /// `&self` mengembalikan irisan `&[MenuItem]` yang meminjam dari `self`;
    /// lifetime-nya disimpulkan otomatis (elision) sama dengan `&self`.
    fn current_items(&self) -> &[MenuItem] {
        let mut items: &[MenuItem] = &self.root;
        for &index in &self.path {
            match items.get(index).map(|item| &item.kind) {
                Some(ItemKind::Submenu(children)) => items = children,
                // Tidak terjadi: `path` hanya diisi indeks submenu yang valid.
                _ => break,
            }
        }
        items
    }

    fn move_down(&mut self) -> Effect {
        let len = self.current_items().len();
        if len == 0 {
            return Effect::None;
        }
        self.selected = (self.selected + 1) % len;
        Effect::Redraw
    }

    fn move_up(&mut self) -> Effect {
        let len = self.current_items().len();
        if len == 0 {
            return Effect::None;
        }
        // `usize` tidak boleh negatif, jadi tambah `len` dulu sebelum mengurangi.
        self.selected = (self.selected + len - 1) % len;
        Effect::Redraw
    }

    fn enter(&mut self) -> Effect {
        let Some(item) = self.current_items().get(self.selected) else {
            return Effect::None;
        };
        match &item.kind {
            ItemKind::Submenu(_) => {
                self.path.push(self.selected);
                self.selected = 0;
                Effect::Redraw
            }
            // Dibuat hidup di sub-langkah 2d (pemilih tema di dalam menu).
            ItemKind::Action(Action::Builtin(Builtin::ThemePicker)) => Effect::None,
            ItemKind::Action(action) => {
                let action = action.clone();
                self.closed = true;
                Effect::Run(action)
            }
        }
    }

    fn back(&mut self) -> Effect {
        if self.path.pop().is_some() {
            // Seperti AHK: kembali ke baris pertama, bukan mengingat posisi.
            self.selected = 0;
            Effect::Redraw
        } else {
            self.close()
        }
    }

    fn close(&mut self) -> Effect {
        self.closed = true;
        Effect::Close
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    /// root: Docs, Tools >(Sleep, More >(Close All)), Terminal, Colors(theme_picker)
    const FIXTURE: &str = r#"
version = 1

[[menu]]
id = "docs"
label = "Docs"
type = "url"
url = "https://example.com"

[[menu]]
id = "tools"
label = "Tools"
type = "submenu"

  [[menu.items]]
  id = "sleep"
  label = "Sleep"
  type = "builtin"
  name = "sleep"

  [[menu.items]]
  id = "more"
  label = "More"
  type = "submenu"

    [[menu.items.items]]
    id = "closeall"
    label = "Close All"
    type = "builtin"
    name = "close_all_windows"

[[menu]]
id = "term"
label = "Terminal"
type = "exec"
command = "wt.exe"
admin = true

[[menu]]
id = "colors"
label = "Colors"
type = "builtin"
name = "theme_picker"
"#;

    fn state() -> MenuState {
        MenuState::new(Config::from_toml_str(FIXTURE).unwrap().menu)
    }

    fn labels(state: &MenuState) -> Vec<String> {
        state.rows().into_iter().map(|row| row.label).collect()
    }

    /// Mengirim beberapa input berturut-turut dan memastikan semuanya `Redraw`.
    fn send(state: &mut MenuState, inputs: &[Input]) {
        for &input in inputs {
            assert_eq!(state.update(input), Effect::Redraw, "{input:?}");
        }
    }

    #[test]
    fn starts_at_root_with_first_row_selected() {
        let state = state();
        assert_eq!(labels(&state), ["Docs", "Tools", "Terminal", "Colors"]);
        assert_eq!(state.selected(), 0);
        assert_eq!(state.depth(), 0);
        assert!(!state.is_closed());
    }

    #[test]
    fn down_moves_selection_and_wraps_to_first() {
        let mut state = state();
        send(&mut state, &[Input::Down]);
        assert_eq!(state.selected(), 1);
        send(&mut state, &[Input::Down, Input::Down, Input::Down]);
        assert_eq!(state.selected(), 0);
    }

    #[test]
    fn up_wraps_from_first_to_last() {
        let mut state = state();
        send(&mut state, &[Input::Up]);
        assert_eq!(state.selected(), 3);
        send(&mut state, &[Input::Up]);
        assert_eq!(state.selected(), 2);
    }

    #[test]
    fn enter_on_action_runs_it_and_closes() {
        let mut state = state();
        send(&mut state, &[Input::Down, Input::Down]);
        assert_eq!(
            state.update(Input::Enter),
            Effect::Run(Action::Exec {
                command: "wt.exe".to_owned(),
                args: vec![],
                admin: true,
            })
        );
        assert!(state.is_closed());
    }

    #[test]
    fn enter_on_submenu_opens_it() {
        let mut state = state();
        send(&mut state, &[Input::Down, Input::Enter]);
        assert_eq!(labels(&state), ["Sleep", "More"]);
        assert_eq!(state.selected(), 0);
        assert_eq!(state.depth(), 1);
        assert!(!state.is_closed());
    }

    #[test]
    fn enter_inside_submenu_runs_that_item() {
        let mut state = state();
        send(&mut state, &[Input::Down, Input::Enter]);
        assert_eq!(
            state.update(Input::Enter),
            Effect::Run(Action::Builtin(Builtin::Sleep))
        );
    }

    #[test]
    fn nested_submenu_goes_two_levels_deep_and_back() {
        let mut state = state();
        send(&mut state, &[Input::Down, Input::Enter]); // Tools
        send(&mut state, &[Input::Down, Input::Enter]); // More
        assert_eq!(labels(&state), ["Close All"]);
        assert_eq!(state.depth(), 2);

        send(&mut state, &[Input::Back]);
        assert_eq!(labels(&state), ["Sleep", "More"]);
        assert_eq!(state.depth(), 1);

        send(&mut state, &[Input::Back]);
        assert_eq!(labels(&state), ["Docs", "Tools", "Terminal", "Colors"]);
        assert_eq!(state.depth(), 0);
    }

    #[test]
    fn back_resets_selection_to_first_row_like_ahk() {
        let mut state = state();
        send(&mut state, &[Input::Down, Input::Enter, Input::Back]);
        // Tadi masuk dari baris "Tools" (indeks 1); kembalinya ke indeks 0.
        assert_eq!(state.selected(), 0);
    }

    #[test]
    fn back_at_root_closes() {
        let mut state = state();
        assert_eq!(state.update(Input::Back), Effect::Close);
        assert!(state.is_closed());
    }

    #[test]
    fn dismiss_closes_from_any_depth() {
        let mut state = state();
        send(&mut state, &[Input::Down, Input::Enter]);
        assert_eq!(state.update(Input::Dismiss), Effect::Close);
        assert!(state.is_closed());
    }

    #[test]
    fn input_after_run_is_ignored() {
        // Meniru focus-loss yang menyusul penutupan jendela sendiri.
        let mut state = state();
        assert!(matches!(state.update(Input::Enter), Effect::Run(_)));
        for input in [
            Input::Up,
            Input::Down,
            Input::Enter,
            Input::Back,
            Input::Dismiss,
        ] {
            assert_eq!(state.update(input), Effect::None, "{input:?}");
        }
    }

    #[test]
    fn input_after_close_is_ignored() {
        let mut state = state();
        assert_eq!(state.update(Input::Back), Effect::Close);
        assert_eq!(state.update(Input::Dismiss), Effect::None);
        assert_eq!(state.update(Input::Enter), Effect::None);
    }

    #[test]
    fn theme_picker_does_nothing_until_substep_2d() {
        let mut state = state();
        send(&mut state, &[Input::Up]); // wrap ke "Colors"
        assert_eq!(state.update(Input::Enter), Effect::None);
        assert!(!state.is_closed());
    }

    #[test]
    fn empty_menu_does_not_panic() {
        let mut state = MenuState::new(Vec::new());
        assert!(state.rows().is_empty());
        assert_eq!(state.update(Input::Down), Effect::None);
        assert_eq!(state.update(Input::Up), Effect::None);
        assert_eq!(state.update(Input::Enter), Effect::None);
        assert_eq!(state.update(Input::Back), Effect::Close);
    }
}
