//! State machine navigasi menu. Murni: input masuk, [`Effect`] keluar, tanpa
//! I/O dan tanpa mengenal kode tombol atau jendela.

use crate::config::{Action, Builtin, Config, ItemKind, MenuItem};
use crate::state::State;
use crate::theme::{DEFAULT_THEME, Theme, ThemeSet};

/// Label baris yang selalu ada di akhir menu utama.
pub const MENU_SETTINGS_LABEL: &str = "Menu Settings";

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
    /// Gambar ulang, terapkan [`MenuState::theme`] (warna dan font bisa
    /// berubah), dan simpan [`MenuState::state`]. Menu tetap terbuka.
    StateChanged,
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

/// Tampilan khusus yang menumpuk di atas jalur submenu. Menekan `Back`
/// melepasnya dan kembali ke menu di bawahnya (menu utama atau submenu tempat
/// item `theme_picker` berada).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Overlay {
    ThemePicker,
    MenuSettings,
}

/// Apa yang terjadi saat sebuah baris dipilih dengan `Enter`. Dengan menyimpan
/// target di samping label, kita tidak pernah mencocokkan teks label (yang
/// dinamis: "(ON)", "(current)") untuk tahu baris mana yang dipilih.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Target {
    /// Indeks item di level aktif pada pohon asli (stabil walau ada yang
    /// disembunyikan, karena yang disembunyikan hanya di-skip, bukan dihapus).
    Item(usize),
    MenuSettings,
    Theme(String),
    /// Id item menu utama yang di-ON/OFF-kan.
    Toggle(String),
}

struct Entry {
    label: String,
    target: Target,
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
    themes: ThemeSet,
    state: State,
    /// Indeks submenu yang dimasuki dari root, berurutan. Kosong = di root.
    path: Vec<usize>,
    overlay: Option<Overlay>,
    selected: usize,
    closed: bool,
}

impl MenuState {
    pub fn new(config: Config, state: State) -> Self {
        Self {
            root: config.menu,
            themes: config.themes,
            state,
            path: Vec::new(),
            overlay: None,
            selected: 0,
            closed: false,
        }
    }

    /// Baris di tampilan yang sedang aktif.
    pub fn rows(&self) -> Vec<Row> {
        self.entries()
            .into_iter()
            .map(|entry| Row { label: entry.label })
            .collect()
    }

    /// Indeks baris terpilih di [`rows`](Self::rows).
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// Seberapa dalam tampilan yang sedang dibuka (0 = menu utama).
    pub fn depth(&self) -> usize {
        self.path.len() + usize::from(self.overlay.is_some())
    }

    /// `true` setelah `Run` atau `Close` dikeluarkan.
    pub fn is_closed(&self) -> bool {
        self.closed
    }

    /// Tema aktif. Jika nama tema tersimpan tidak dikenal (misalnya tema
    /// custom yang sudah dihapus dari config), jatuh ke tema bawaan.
    pub fn theme(&self) -> &Theme {
        let name = self.state.theme.as_deref().unwrap_or(DEFAULT_THEME);
        self.themes.get_or_default(name)
    }

    /// State yang perlu disimpan setelah [`Effect::StateChanged`].
    pub fn state(&self) -> &State {
        &self.state
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

    /// Daftar item pohon di level submenu aktif (tanpa filter), dicari dengan
    /// menelusuri `path` dari root. Lifetime irisan yang dikembalikan
    /// disimpulkan otomatis (elision) sama dengan `&self`.
    fn level_items(&self) -> &[MenuItem] {
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

    /// Baris beserta targetnya untuk tampilan aktif. `rows()` dan `enter()`
    /// sama-sama berangkat dari sini, jadi keduanya tidak mungkin tidak sinkron.
    fn entries(&self) -> Vec<Entry> {
        match self.overlay {
            Some(Overlay::ThemePicker) => {
                let active = self.theme().name.as_str();
                self.themes
                    .iter()
                    .map(|theme| Entry {
                        label: if theme.name == active {
                            format!("{} (current)", theme.name)
                        } else {
                            theme.name.clone()
                        },
                        target: Target::Theme(theme.name.clone()),
                    })
                    .collect()
            }
            Some(Overlay::MenuSettings) => self
                .root
                .iter()
                .map(|item| {
                    let status = if self.state.hidden.contains(&item.id) {
                        "(OFF)"
                    } else {
                        "(ON)"
                    };
                    Entry {
                        label: format!("{} {status}", item.label),
                        target: Target::Toggle(item.id.clone()),
                    }
                })
                .collect(),
            None => {
                // Hanya menu utama yang bisa di-OFF dan hanya menu utama yang
                // punya baris "Menu Settings".
                let at_root = self.path.is_empty();
                let mut entries: Vec<Entry> = self
                    .level_items()
                    .iter()
                    .enumerate()
                    .filter(|(_, item)| !(at_root && self.state.hidden.contains(&item.id)))
                    .map(|(index, item)| Entry {
                        label: item.label.clone(),
                        target: Target::Item(index),
                    })
                    .collect();
                if at_root {
                    entries.push(Entry {
                        label: MENU_SETTINGS_LABEL.to_owned(),
                        target: Target::MenuSettings,
                    });
                }
                entries
            }
        }
    }

    fn move_down(&mut self) -> Effect {
        let len = self.entries().len();
        if len == 0 {
            return Effect::None;
        }
        self.selected = (self.selected + 1) % len;
        Effect::Redraw
    }

    fn move_up(&mut self) -> Effect {
        let len = self.entries().len();
        if len == 0 {
            return Effect::None;
        }
        // `usize` tidak boleh negatif, jadi tambah `len` dulu sebelum mengurangi.
        self.selected = (self.selected + len - 1) % len;
        Effect::Redraw
    }

    fn enter(&mut self) -> Effect {
        // `into_iter().nth(..)` memindahkan (move) entri terpilih keluar, jadi
        // `target` adalah milik kita dan pinjaman ke `self` sudah lepas. Tanpa
        // itu, mutasi `self` di bawah ditolak borrow checker.
        let Some(entry) = self.entries().into_iter().nth(self.selected) else {
            return Effect::None;
        };
        match entry.target {
            Target::Item(index) => self.enter_item(index),
            Target::MenuSettings => self.open_overlay(Overlay::MenuSettings),
            Target::Theme(name) => {
                self.state.theme = Some(name);
                Effect::StateChanged
            }
            Target::Toggle(id) => {
                // `remove` mengembalikan false bila id belum ada -> jadi OFF.
                if !self.state.hidden.remove(&id) {
                    self.state.hidden.insert(id);
                }
                Effect::StateChanged
            }
        }
    }

    fn enter_item(&mut self, index: usize) -> Effect {
        let Some(item) = self.level_items().get(index) else {
            return Effect::None;
        };
        match &item.kind {
            ItemKind::Submenu(_) => {
                self.path.push(index);
                self.selected = 0;
                Effect::Redraw
            }
            ItemKind::Action(Action::Builtin(Builtin::ThemePicker)) => {
                self.open_overlay(Overlay::ThemePicker)
            }
            ItemKind::Action(action) => {
                let action = action.clone();
                self.closed = true;
                Effect::Run(action)
            }
        }
    }

    fn open_overlay(&mut self, overlay: Overlay) -> Effect {
        self.overlay = Some(overlay);
        self.selected = 0;
        Effect::Redraw
    }

    fn back(&mut self) -> Effect {
        // Seperti AHK: setelah mundur, seleksi kembali ke baris pertama.
        if self.overlay.take().is_some() || self.path.pop().is_some() {
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
    use crate::theme::FONT_IBM_PLEX_MONO;

    /// root: Docs, Tools >(Sleep, More >(Close All)), Terminal, Colors(theme_picker)
    /// + baris otomatis "Menu Settings".
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

    /// Pemilih tema berada di dalam submenu.
    const NESTED_PICKER: &str = r#"
version = 1

[[menu]]
id = "look"
label = "Look"
type = "submenu"

  [[menu.items]]
  id = "colors"
  label = "Colors"
  type = "builtin"
  name = "theme_picker"
"#;

    fn menu_with(source: &str, state: State) -> MenuState {
        MenuState::new(Config::from_toml_str(source).unwrap(), state)
    }

    fn menu() -> MenuState {
        menu_with(FIXTURE, State::default())
    }

    fn hidden(ids: &[&str]) -> State {
        State {
            hidden: ids.iter().map(|id| (*id).to_owned()).collect(),
            ..State::default()
        }
    }

    fn with_theme(name: &str) -> State {
        State {
            theme: Some(name.to_owned()),
            ..State::default()
        }
    }

    fn labels(menu: &MenuState) -> Vec<String> {
        menu.rows().into_iter().map(|row| row.label).collect()
    }

    /// Mengirim beberapa input berturut-turut dan memastikan semuanya `Redraw`.
    fn send(menu: &mut MenuState, inputs: &[Input]) {
        for &input in inputs {
            assert_eq!(menu.update(input), Effect::Redraw, "{input:?}");
        }
    }

    const ROOT: [&str; 5] = ["Docs", "Tools", "Terminal", "Colors", "Menu Settings"];

    // ---- navigasi dasar ----

    #[test]
    fn starts_at_root_with_first_row_selected() {
        let menu = menu();
        assert_eq!(labels(&menu), ROOT);
        assert_eq!(menu.selected(), 0);
        assert_eq!(menu.depth(), 0);
        assert!(!menu.is_closed());
    }

    #[test]
    fn down_moves_selection_and_wraps_to_first() {
        let mut menu = menu();
        send(&mut menu, &[Input::Down]);
        assert_eq!(menu.selected(), 1);
        send(&mut menu, &[Input::Down; 4]);
        assert_eq!(menu.selected(), 0);
    }

    #[test]
    fn up_wraps_from_first_to_last() {
        let mut menu = menu();
        send(&mut menu, &[Input::Up]);
        assert_eq!(menu.selected(), 4);
        send(&mut menu, &[Input::Up]);
        assert_eq!(menu.selected(), 3);
    }

    #[test]
    fn enter_on_action_runs_it_and_closes() {
        let mut menu = menu();
        send(&mut menu, &[Input::Down, Input::Down]);
        assert_eq!(
            menu.update(Input::Enter),
            Effect::Run(Action::Exec {
                command: "wt.exe".to_owned(),
                args: vec![],
                admin: true,
            })
        );
        assert!(menu.is_closed());
    }

    #[test]
    fn enter_on_submenu_opens_it() {
        let mut menu = menu();
        send(&mut menu, &[Input::Down, Input::Enter]);
        assert_eq!(labels(&menu), ["Sleep", "More"]);
        assert_eq!(menu.selected(), 0);
        assert_eq!(menu.depth(), 1);
        assert!(!menu.is_closed());
    }

    #[test]
    fn enter_inside_submenu_runs_that_item() {
        let mut menu = menu();
        send(&mut menu, &[Input::Down, Input::Enter]);
        assert_eq!(
            menu.update(Input::Enter),
            Effect::Run(Action::Builtin(Builtin::Sleep))
        );
    }

    #[test]
    fn nested_submenu_goes_two_levels_deep_and_back() {
        let mut menu = menu();
        send(&mut menu, &[Input::Down, Input::Enter]); // Tools
        send(&mut menu, &[Input::Down, Input::Enter]); // More
        assert_eq!(labels(&menu), ["Close All"]);
        assert_eq!(menu.depth(), 2);

        send(&mut menu, &[Input::Back]);
        assert_eq!(labels(&menu), ["Sleep", "More"]);
        assert_eq!(menu.depth(), 1);

        send(&mut menu, &[Input::Back]);
        assert_eq!(labels(&menu), ROOT);
        assert_eq!(menu.depth(), 0);
    }

    #[test]
    fn back_resets_selection_to_first_row_like_ahk() {
        let mut menu = menu();
        send(&mut menu, &[Input::Down, Input::Enter, Input::Back]);
        assert_eq!(menu.selected(), 0);
    }

    #[test]
    fn back_at_root_closes() {
        let mut menu = menu();
        assert_eq!(menu.update(Input::Back), Effect::Close);
        assert!(menu.is_closed());
    }

    #[test]
    fn dismiss_closes_from_any_depth() {
        let mut menu = menu();
        send(&mut menu, &[Input::Down, Input::Enter]);
        assert_eq!(menu.update(Input::Dismiss), Effect::Close);
        assert!(menu.is_closed());
    }

    #[test]
    fn input_after_run_is_ignored() {
        // Meniru focus-loss yang menyusul penutupan jendela sendiri.
        let mut menu = menu();
        assert!(matches!(menu.update(Input::Enter), Effect::Run(_)));
        for input in [
            Input::Up,
            Input::Down,
            Input::Enter,
            Input::Back,
            Input::Dismiss,
        ] {
            assert_eq!(menu.update(input), Effect::None, "{input:?}");
        }
    }

    #[test]
    fn input_after_close_is_ignored() {
        let mut menu = menu();
        assert_eq!(menu.update(Input::Back), Effect::Close);
        assert_eq!(menu.update(Input::Dismiss), Effect::None);
        assert_eq!(menu.update(Input::Enter), Effect::None);
    }

    #[test]
    fn empty_menu_only_has_menu_settings_and_does_not_panic() {
        let config = Config {
            menu: Vec::new(),
            themes: ThemeSet::builtin(),
        };
        let mut menu = MenuState::new(config, State::default());
        assert_eq!(labels(&menu), ["Menu Settings"]);
        send(&mut menu, &[Input::Down, Input::Up, Input::Enter]);
        assert!(labels(&menu).is_empty()); // Menu Settings tanpa item
        assert_eq!(menu.update(Input::Enter), Effect::None);
        send(&mut menu, &[Input::Back]);
        assert_eq!(menu.update(Input::Back), Effect::Close);
    }

    // ---- Menu Settings ----

    #[test]
    fn menu_settings_row_is_last_at_root_only() {
        let mut menu = menu();
        assert_eq!(labels(&menu).last().unwrap(), MENU_SETTINGS_LABEL);
        send(&mut menu, &[Input::Down, Input::Enter]); // Tools
        assert!(!labels(&menu).contains(&MENU_SETTINGS_LABEL.to_owned()));
    }

    #[test]
    fn menu_settings_lists_every_root_item_with_status() {
        let mut menu = menu();
        send(&mut menu, &[Input::Up, Input::Enter]); // wrap ke Menu Settings
        assert_eq!(
            labels(&menu),
            ["Docs (ON)", "Tools (ON)", "Terminal (ON)", "Colors (ON)"]
        );
        assert_eq!(menu.selected(), 0);
        assert_eq!(menu.depth(), 1);
    }

    #[test]
    fn toggling_an_item_hides_it_from_root_after_going_back() {
        let mut menu = menu();
        send(&mut menu, &[Input::Up, Input::Enter]); // Menu Settings, baris "Docs"
        assert_eq!(menu.update(Input::Enter), Effect::StateChanged);
        assert_eq!(labels(&menu)[0], "Docs (OFF)");
        assert!(menu.state().hidden.contains("docs"));
        assert!(!menu.is_closed());

        send(&mut menu, &[Input::Back]);
        assert_eq!(
            labels(&menu),
            ["Tools", "Terminal", "Colors", "Menu Settings"]
        );
    }

    #[test]
    fn toggling_twice_shows_the_item_again() {
        let mut menu = menu();
        send(&mut menu, &[Input::Up, Input::Enter]);
        assert_eq!(menu.update(Input::Enter), Effect::StateChanged);
        assert_eq!(menu.update(Input::Enter), Effect::StateChanged);
        assert!(menu.state().hidden.is_empty());
        send(&mut menu, &[Input::Back]);
        assert_eq!(labels(&menu), ROOT);
    }

    #[test]
    fn hidden_items_do_not_break_selection_mapping() {
        // "Docs" disembunyikan: baris pertama kini "Tools", dan Enter harus
        // membuka Tools (indeks asli 1), bukan item pertama pohon.
        let mut menu = menu_with(FIXTURE, hidden(&["docs"]));
        assert_eq!(
            labels(&menu),
            ["Tools", "Terminal", "Colors", "Menu Settings"]
        );
        send(&mut menu, &[Input::Enter]);
        assert_eq!(labels(&menu), ["Sleep", "More"]);
    }

    #[test]
    fn hidden_item_after_hidden_one_still_runs_the_right_action() {
        let mut menu = menu_with(FIXTURE, hidden(&["docs", "tools"]));
        assert_eq!(labels(&menu), ["Terminal", "Colors", "Menu Settings"]);
        assert!(matches!(
            menu.update(Input::Enter),
            Effect::Run(Action::Exec { .. })
        ));
    }

    #[test]
    fn hiding_everything_leaves_only_menu_settings() {
        let mut menu = menu_with(FIXTURE, hidden(&["docs", "tools", "term", "colors"]));
        assert_eq!(labels(&menu), ["Menu Settings"]);
        // Masih ada jalan balik.
        send(&mut menu, &[Input::Enter]);
        assert_eq!(labels(&menu).len(), 4);
    }

    #[test]
    fn hidden_flag_only_applies_to_root_items() {
        // "sleep" ada di dalam submenu Tools; id-nya di hidden tidak berpengaruh.
        let mut menu = menu_with(FIXTURE, hidden(&["sleep"]));
        send(&mut menu, &[Input::Down, Input::Enter]);
        assert_eq!(labels(&menu), ["Sleep", "More"]);
    }

    #[test]
    fn stale_hidden_ids_are_kept_when_toggling_others() {
        let mut menu = menu_with(FIXTURE, hidden(&["ghost"]));
        send(&mut menu, &[Input::Up, Input::Enter]);
        assert_eq!(menu.update(Input::Enter), Effect::StateChanged); // OFF-kan "docs"
        assert!(menu.state().hidden.contains("ghost"));
        assert!(menu.state().hidden.contains("docs"));
    }

    // ---- Pemilih tema ----

    fn open_picker(menu: &mut MenuState) {
        // Colors ada di indeks 3 root.
        send(menu, &[Input::Down, Input::Down, Input::Down, Input::Enter]);
    }

    #[test]
    fn theme_picker_lists_all_themes_and_marks_current() {
        let mut menu = menu();
        open_picker(&mut menu);
        let rows = labels(&menu);
        assert_eq!(rows.len(), 7);
        assert_eq!(rows[0], "game_boy");
        assert_eq!(rows[1], "amber (current)");
        assert_eq!(menu.selected(), 0);
        assert_eq!(menu.depth(), 1);
    }

    #[test]
    fn choosing_a_theme_applies_it_and_stays_open() {
        let mut menu = menu();
        assert_eq!(menu.theme().name, "amber");
        open_picker(&mut menu);
        send(&mut menu, &[Input::Down; 4]); // gruvbox
        assert_eq!(menu.update(Input::Enter), Effect::StateChanged);

        assert_eq!(menu.theme().name, "gruvbox");
        assert_eq!(menu.theme().font, FONT_IBM_PLEX_MONO);
        assert_eq!(menu.state().theme.as_deref(), Some("gruvbox"));
        assert_eq!(labels(&menu)[4], "gruvbox (current)");
        assert_eq!(labels(&menu)[1], "amber");
        assert!(!menu.is_closed());
        assert_eq!(menu.depth(), 1);
    }

    #[test]
    fn unknown_stored_theme_falls_back_to_default_and_is_marked_current() {
        let mut menu = menu_with(FIXTURE, with_theme("nope"));
        assert_eq!(menu.theme().name, "amber");
        open_picker(&mut menu);
        assert_eq!(labels(&menu)[1], "amber (current)");
    }

    #[test]
    fn stored_theme_is_used_from_the_start() {
        assert_eq!(
            menu_with(FIXTURE, with_theme("vague")).theme().name,
            "vague"
        );
    }

    #[test]
    fn back_from_theme_picker_returns_to_root() {
        let mut menu = menu();
        open_picker(&mut menu);
        send(&mut menu, &[Input::Back]);
        assert_eq!(labels(&menu), ROOT);
        assert_eq!(menu.depth(), 0);
        assert_eq!(menu.selected(), 0);
    }

    #[test]
    fn back_from_theme_picker_inside_submenu_returns_to_that_submenu() {
        let mut menu = menu_with(NESTED_PICKER, State::default());
        send(&mut menu, &[Input::Enter]); // Look
        assert_eq!(labels(&menu), ["Colors"]);
        send(&mut menu, &[Input::Enter]); // pemilih tema
        assert_eq!(labels(&menu).len(), 7);
        assert_eq!(menu.depth(), 2);

        send(&mut menu, &[Input::Back]);
        assert_eq!(labels(&menu), ["Colors"]);
        send(&mut menu, &[Input::Back]);
        assert_eq!(labels(&menu), ["Look", "Menu Settings"]);
    }

    #[test]
    fn dismiss_inside_overlay_closes() {
        let mut menu = menu();
        open_picker(&mut menu);
        assert_eq!(menu.update(Input::Dismiss), Effect::Close);
        assert!(menu.is_closed());
    }

    #[test]
    fn default_config_menu_is_navigable() {
        use crate::config::DEFAULT_CONFIG;
        let mut menu = menu_with(DEFAULT_CONFIG, State::default());
        assert_eq!(labels(&menu).first().unwrap(), "Color Scheme");
        assert_eq!(labels(&menu).last().unwrap(), MENU_SETTINGS_LABEL);
        send(&mut menu, &[Input::Enter]); // pemilih tema
        assert_eq!(labels(&menu).len(), 7);
        send(&mut menu, &[Input::Back, Input::Down, Input::Enter]); // Links
        assert_eq!(labels(&menu), ["GitHub"]);
    }

    #[test]
    fn custom_theme_from_config_appears_in_picker() {
        let source = format!(
            "{FIXTURE}\n[[themes]]\nname = \"mine\"\nbg = \"#101010\"\nfg = \"#FFFFFF\"\nsel_bg = \"#FFFFFF\"\nsel_fg = \"#101010\"\nbezel = \"#000000\"\nfont = \"vt323\"\n"
        );
        let mut menu = menu_with(&source, State::default());
        open_picker(&mut menu);
        let rows = labels(&menu);
        assert_eq!(rows.len(), 8);
        assert_eq!(rows[7], "mine");
    }
}
