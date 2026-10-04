//! Model config dan parser TOML.
//!
//! Alurnya dua tahap:
//! 1. TOML diurai ke tipe `Raw*` (bentuk persis seperti file). Kesalahan di tahap
//!    ini (sintaks, tipe salah, field tak dikenal, warna rusak) membawa posisi
//!    baris dan kolom dari parser TOML.
//! 2. `Raw*` divalidasi dan diubah ke tipe domain ([`Config`], [`MenuItem`], ...).
//!    Kesalahan semantik (id ganda, label kosong, field wajib hilang) menunjuk
//!    lewat jalur item, misalnya `menu[2].items[0]`, karena `serde` tidak
//!    menyimpan posisi sumber setelah nilai dibaca.

use std::collections::HashSet;

use serde::Deserialize;
use thiserror::Error;

use crate::rgb::Rgb;
use crate::theme::{DEFAULT_FONT, Theme, ThemeSet};

/// Versi format config yang dipahami kode ini.
pub const CONFIG_VERSION: u32 = 1;

/// Config contoh yang ditulis oleh `koha --init`. `include_str!` menyalin isi
/// file ke dalam binary saat kompilasi, jadi tidak ada file yang dibaca saat
/// jalan. Test di bawah menjamin isinya selalu valid.
pub const DEFAULT_CONFIG: &str = include_str!("default_config.toml");

/// Config yang sudah lolos validasi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// Menu utama (belum termasuk baris "Menu Settings" yang ditambahkan
    /// otomatis oleh state machine menu).
    pub menu: Vec<MenuItem>,
    /// Tema bawaan, ditambah/ditimpa oleh tema di config.
    pub themes: ThemeSet,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuItem {
    /// Pengenal stabil, dipakai untuk menyimpan status ON/OFF. Unik di seluruh
    /// config, tidak berubah walau `label` diganti.
    pub id: String,
    pub label: String,
    pub kind: ItemKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemKind {
    Action(Action),
    /// `Vec<MenuItem>` di dalam `MenuItem` itu sah: `Vec` menyimpan elemennya di
    /// heap, jadi ukuran tipe rekursif ini tetap diketahui saat kompilasi.
    Submenu(Vec<MenuItem>),
}

/// Aksi yang dijalankan saat item dipilih.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Exec {
        command: String,
        args: Vec<String>,
        /// Jalankan dengan elevasi (UAC di Windows).
        admin: bool,
    },
    Url(String),
    Builtin(Builtin),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Builtin {
    Lock,
    Sleep,
    CloseAllWindows,
    /// Bukan perintah OS: membuka pemilih tema di dalam menu. Ditangani oleh
    /// state machine menu dan tidak pernah sampai ke `Platform`.
    ThemePicker,
}

impl Builtin {
    /// Nama yang dipakai di config.
    pub const NAMES: [&'static str; 4] = ["lock", "sleep", "close_all_windows", "theme_picker"];

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "lock" => Some(Self::Lock),
            "sleep" => Some(Self::Sleep),
            "close_all_windows" => Some(Self::CloseAllWindows),
            "theme_picker" => Some(Self::ThemePicker),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ConfigError {
    /// Kesalahan dari parser TOML yang punya posisi (1-based).
    #[error("baris {line}, kolom {column}: {message}")]
    Syntax {
        line: usize,
        column: usize,
        message: String,
    },
    /// Kesalahan dari parser TOML tanpa posisi.
    #[error("{message}")]
    Toml { message: String },
    /// Kesalahan semantik; `path` menunjuk item, misalnya `menu[2].items[0]`.
    #[error("{path}: {message}")]
    Invalid { path: String, message: String },
}

impl ConfigError {
    pub(crate) fn invalid(path: &str, message: impl Into<String>) -> Self {
        Self::Invalid {
            path: path.to_owned(),
            message: message.into(),
        }
    }

    pub(crate) fn from_toml(err: &toml::de::Error, source: &str) -> Self {
        let message = err.message().to_owned();
        match err.span() {
            Some(span) => {
                let (line, column) = line_col(source, span.start);
                Self::Syntax {
                    line,
                    column,
                    message,
                }
            }
            None => Self::Toml { message },
        }
    }
}

/// Mengubah offset byte menjadi (baris, kolom), keduanya mulai dari 1.
/// Kolom dihitung dalam karakter, bukan byte.
fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut column = 1;
    for (index, ch) in source.char_indices() {
        if index >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    (line, column)
}

// ---- Tahap 1: bentuk file ------------------------------------------------

/// Hanya untuk membaca `version`; field lain diabaikan. Dengan begitu file dari
/// versi masa depan (yang field-nya mungkin berbeda) tetap menghasilkan pesan
/// "versi tidak didukung" yang jelas, bukan error field tak dikenal.
#[derive(Deserialize)]
struct VersionProbe {
    version: Option<u32>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    #[serde(rename = "version")]
    _version: u32,
    #[serde(default)]
    menu: Vec<RawItem>,
    #[serde(default)]
    themes: Vec<RawTheme>,
}

/// Satu bentuk datar untuk semua jenis item. Field yang tidak relevan untuk
/// sebuah `type` ditolak saat validasi dengan pesan yang menyebut nama field-nya.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawItem {
    id: String,
    label: String,
    #[serde(rename = "type")]
    kind: String,
    command: Option<String>,
    args: Option<Vec<String>>,
    admin: Option<bool>,
    url: Option<String>,
    name: Option<String>,
    items: Option<Vec<RawItem>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTheme {
    name: String,
    bg: Rgb,
    fg: Rgb,
    sel_bg: Rgb,
    sel_fg: Rgb,
    bezel: Rgb,
    font: Option<String>,
}

// ---- Tahap 2: validasi dan konversi --------------------------------------

impl Config {
    /// Mengurai dan memvalidasi isi `config.toml`.
    pub fn from_toml_str(source: &str) -> Result<Self, ConfigError> {
        let probe: VersionProbe =
            toml::from_str(source).map_err(|e| ConfigError::from_toml(&e, source))?;
        match probe.version {
            None => return Err(ConfigError::invalid("version", "field wajib ada")),
            Some(CONFIG_VERSION) => {}
            Some(other) => {
                return Err(ConfigError::invalid(
                    "version",
                    format!("versi {other} tidak didukung (yang didukung: {CONFIG_VERSION})"),
                ));
            }
        }

        let raw: RawConfig =
            toml::from_str(source).map_err(|e| ConfigError::from_toml(&e, source))?;

        if raw.menu.is_empty() {
            return Err(ConfigError::invalid("menu", "menu tidak boleh kosong"));
        }
        let mut ids = HashSet::new();
        let menu = convert_items(&raw.menu, "menu", &mut ids)?;
        let themes = convert_themes(&raw.themes)?;
        Ok(Self { menu, themes })
    }
}

fn convert_items(
    raw: &[RawItem],
    path: &str,
    ids: &mut HashSet<String>,
) -> Result<Vec<MenuItem>, ConfigError> {
    // `collect()` ke `Result<Vec<_>, _>` berhenti di error pertama.
    raw.iter()
        .enumerate()
        .map(|(index, item)| convert_item(item, &format!("{path}[{index}]"), ids))
        .collect()
}

fn convert_item(
    raw: &RawItem,
    path: &str,
    ids: &mut HashSet<String>,
) -> Result<MenuItem, ConfigError> {
    validate_id(&raw.id).map_err(|message| ConfigError::invalid(path, message))?;
    if !ids.insert(raw.id.clone()) {
        return Err(ConfigError::invalid(
            path,
            format!("id \"{}\" dipakai lebih dari sekali", raw.id),
        ));
    }
    if raw.label.trim().is_empty() {
        return Err(ConfigError::invalid(path, "label tidak boleh kosong"));
    }

    let kind = match raw.kind.as_str() {
        "exec" => {
            reject_foreign_fields(raw, "exec", &["command", "args", "admin"], path)?;
            let command = required_text(&raw.command, "exec", "command", path)?;
            ItemKind::Action(Action::Exec {
                command,
                args: raw.args.clone().unwrap_or_default(),
                admin: raw.admin.unwrap_or(false),
            })
        }
        "url" => {
            reject_foreign_fields(raw, "url", &["url"], path)?;
            ItemKind::Action(Action::Url(required_text(&raw.url, "url", "url", path)?))
        }
        "builtin" => {
            reject_foreign_fields(raw, "builtin", &["name"], path)?;
            let name = required_text(&raw.name, "builtin", "name", path)?;
            let builtin = Builtin::from_name(&name).ok_or_else(|| {
                ConfigError::invalid(
                    path,
                    format!(
                        "builtin \"{name}\" tidak dikenal (pilihan: {})",
                        Builtin::NAMES.join(", ")
                    ),
                )
            })?;
            ItemKind::Action(Action::Builtin(builtin))
        }
        "submenu" => {
            reject_foreign_fields(raw, "submenu", &["items"], path)?;
            let items = raw.items.as_deref().unwrap_or_default();
            if items.is_empty() {
                return Err(ConfigError::invalid(
                    path,
                    "submenu wajib punya minimal satu item (field \"items\")",
                ));
            }
            ItemKind::Submenu(convert_items(items, &format!("{path}.items"), ids)?)
        }
        other => {
            return Err(ConfigError::invalid(
                path,
                format!("type \"{other}\" tidak dikenal (pilihan: exec, url, builtin, submenu)"),
            ));
        }
    };

    Ok(MenuItem {
        id: raw.id.clone(),
        label: raw.label.clone(),
        kind,
    })
}

fn validate_id(id: &str) -> Result<(), String> {
    if id.is_empty() {
        return Err("id tidak boleh kosong".to_owned());
    }
    let valid = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-';
    if !id.chars().all(valid) {
        return Err(format!(
            "id \"{id}\" hanya boleh huruf kecil a-z, angka, \"_\" dan \"-\""
        ));
    }
    Ok(())
}

/// Menolak field yang tidak berlaku untuk `type` ini (misalnya `url` pada `exec`).
fn reject_foreign_fields(
    raw: &RawItem,
    kind: &str,
    allowed: &[&str],
    path: &str,
) -> Result<(), ConfigError> {
    let present = [
        ("command", raw.command.is_some()),
        ("args", raw.args.is_some()),
        ("admin", raw.admin.is_some()),
        ("url", raw.url.is_some()),
        ("name", raw.name.is_some()),
        ("items", raw.items.is_some()),
    ];
    for (field, is_present) in present {
        if is_present && !allowed.contains(&field) {
            return Err(ConfigError::invalid(
                path,
                format!("field \"{field}\" tidak berlaku untuk type \"{kind}\""),
            ));
        }
    }
    Ok(())
}

/// Field teks wajib: harus ada dan tidak kosong/hanya spasi.
fn required_text(
    value: &Option<String>,
    kind: &str,
    field: &str,
    path: &str,
) -> Result<String, ConfigError> {
    match value.as_deref().map(str::trim) {
        Some(text) if !text.is_empty() => Ok(text.to_owned()),
        _ => Err(ConfigError::invalid(
            path,
            format!("type \"{kind}\" wajib punya field \"{field}\" yang tidak kosong"),
        )),
    }
}

fn convert_themes(raw: &[RawTheme]) -> Result<ThemeSet, ConfigError> {
    let mut themes = ThemeSet::builtin();
    let mut seen = HashSet::new();
    for (index, theme) in raw.iter().enumerate() {
        let path = format!("themes[{index}]");
        let name = theme.name.trim();
        if name.is_empty() {
            return Err(ConfigError::invalid(&path, "name tidak boleh kosong"));
        }
        if !seen.insert(name.to_owned()) {
            return Err(ConfigError::invalid(
                &path,
                format!("tema \"{name}\" didefinisikan lebih dari sekali"),
            ));
        }
        let font = match theme.font.as_deref().map(str::trim) {
            None => DEFAULT_FONT.to_owned(),
            Some("") => return Err(ConfigError::invalid(&path, "font tidak boleh kosong")),
            Some(font) => font.to_owned(),
        };
        themes.add_or_replace(Theme {
            name: name.to_owned(),
            bg: theme.bg,
            fg: theme.fg,
            sel_bg: theme.sel_bg,
            sel_fg: theme.sel_fg,
            bezel: theme.bezel,
            font,
        });
    }
    Ok(themes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> Result<Config, ConfigError> {
        Config::from_toml_str(source)
    }

    /// Mengambil pesan `Invalid` dan mengembalikan (path, message).
    fn invalid(source: &str) -> (String, String) {
        match parse(source) {
            Err(ConfigError::Invalid { path, message }) => (path, message),
            other => panic!("diharapkan ConfigError::Invalid, dapat {other:?}"),
        }
    }

    fn syntax(source: &str) -> (usize, usize, String) {
        match parse(source) {
            Err(ConfigError::Syntax {
                line,
                column,
                message,
            }) => (line, column, message),
            other => panic!("diharapkan ConfigError::Syntax, dapat {other:?}"),
        }
    }

    const MINIMAL: &str = r#"
version = 1

[[menu]]
id = "lock"
label = "Lock PC"
type = "builtin"
name = "lock"
"#;

    #[test]
    fn parses_minimal_config() {
        let config = parse(MINIMAL).unwrap();
        assert_eq!(config.menu.len(), 1);
        assert_eq!(config.menu[0].id, "lock");
        assert_eq!(config.menu[0].label, "Lock PC");
        assert_eq!(
            config.menu[0].kind,
            ItemKind::Action(Action::Builtin(Builtin::Lock))
        );
        assert_eq!(config.themes, ThemeSet::builtin());
    }

    #[test]
    fn parses_every_item_type_with_nested_submenu() {
        let config = parse(
            r#"
version = 1

[[menu]]
id = "term"
label = "Terminal (Admin)"
type = "exec"
command = "wt.exe"
args = ["-d", "C:\\"]
admin = true

[[menu]]
id = "docs"
label = "Docs"
type = "url"
url = "https://example.com"

[[menu]]
id = "colors"
label = "Color Scheme"
type = "builtin"
name = "theme_picker"

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
    label = "Close All Windows"
    type = "builtin"
    name = "close_all_windows"
"#,
        )
        .unwrap();

        assert_eq!(config.menu.len(), 4);
        assert_eq!(
            config.menu[0].kind,
            ItemKind::Action(Action::Exec {
                command: "wt.exe".to_owned(),
                args: vec!["-d".to_owned(), "C:\\".to_owned()],
                admin: true,
            })
        );
        assert_eq!(
            config.menu[1].kind,
            ItemKind::Action(Action::Url("https://example.com".to_owned()))
        );
        assert_eq!(
            config.menu[2].kind,
            ItemKind::Action(Action::Builtin(Builtin::ThemePicker))
        );
        let ItemKind::Submenu(tools) = &config.menu[3].kind else {
            panic!("tools harus submenu");
        };
        assert_eq!(tools.len(), 2);
        let ItemKind::Submenu(more) = &tools[1].kind else {
            panic!("more harus submenu");
        };
        assert_eq!(more[0].id, "closeall");
    }

    #[test]
    fn exec_defaults_to_no_args_and_no_admin() {
        let config = parse(
            r#"
version = 1
[[menu]]
id = "t"
label = "T"
type = "exec"
command = "notepad"
"#,
        )
        .unwrap();
        assert_eq!(
            config.menu[0].kind,
            ItemKind::Action(Action::Exec {
                command: "notepad".to_owned(),
                args: vec![],
                admin: false,
            })
        );
    }

    #[test]
    fn toml_syntax_error_reports_line_and_column() {
        let (line, _column, _message) = syntax("version = 1\n[[menu\n");
        assert_eq!(line, 2);
    }

    #[test]
    fn unknown_field_reports_its_position() {
        let source = "version = 1\n[[menu]]\nid = \"a\"\nlabel = \"A\"\ntype = \"url\"\nurl = \"x\"\nbogus = 1\n";
        let (line, column, message) = syntax(source);
        assert_eq!((line, column), (7, 1));
        assert!(message.contains("bogus"), "{message}");
    }

    #[test]
    fn wrong_value_type_reports_its_position() {
        let source = "version = 1\n[[menu]]\nid = \"a\"\nlabel = 5\ntype = \"url\"\nurl = \"x\"\n";
        let (line, column, _message) = syntax(source);
        assert_eq!((line, column), (4, 9));
    }

    #[test]
    fn invalid_theme_color_reports_its_position() {
        // r##"..."## karena isinya mengandung `"#` yang akan mengakhiri r#"..."#.
        let source = r##"
version = 1
[[menu]]
id = "a"
label = "A"
type = "url"
url = "x"

[[themes]]
name = "x"
bg = "red"
fg = "#FFFFFF"
sel_bg = "#000000"
sel_fg = "#FFFFFF"
bezel = "#000000"
"##;
        let (line, _column, message) = syntax(source);
        assert_eq!(line, 11);
        assert!(message.contains("6 digit hex"), "{message}");
    }

    #[test]
    fn column_counts_characters_not_bytes() {
        assert_eq!(line_col("héllo = 1", 7), (1, 7));
        assert_eq!(line_col("a\nbc", 3), (2, 2));
    }

    #[test]
    fn missing_version_is_rejected() {
        let (path, _message) = invalid("[[menu]]\nid = \"a\"\n");
        assert_eq!(path, "version");
    }

    #[test]
    fn unsupported_version_is_reported_before_unknown_fields() {
        // Field "from_the_future" tidak dikenal, tetapi yang dilaporkan harus versinya.
        let (path, message) = invalid("version = 2\nfrom_the_future = true\n");
        assert_eq!(path, "version");
        assert!(message.contains('2'), "{message}");
    }

    #[test]
    fn empty_menu_is_rejected() {
        let (path, _message) = invalid("version = 1\n");
        assert_eq!(path, "menu");
    }

    #[test]
    fn duplicate_id_is_rejected_across_nesting_levels() {
        let (path, message) = invalid(
            r#"
version = 1
[[menu]]
id = "same"
label = "A"
type = "url"
url = "x"

[[menu]]
id = "sub"
label = "Sub"
type = "submenu"
  [[menu.items]]
  id = "same"
  label = "B"
  type = "url"
  url = "y"
"#,
        );
        assert_eq!(path, "menu[1].items[0]");
        assert!(message.contains("same"), "{message}");
    }

    #[test]
    fn id_must_be_lowercase_slug() {
        for bad in ["", "Has Space", "UPPER", "a.b", "é"] {
            let source = format!(
                "version = 1\n[[menu]]\nid = \"{bad}\"\nlabel = \"A\"\ntype = \"url\"\nurl = \"x\"\n"
            );
            let (path, _message) = invalid(&source);
            assert_eq!(path, "menu[0]", "id {bad:?}");
        }
    }

    #[test]
    fn empty_label_is_rejected() {
        let (path, message) = invalid(
            "version = 1\n[[menu]]\nid = \"a\"\nlabel = \"   \"\ntype = \"url\"\nurl = \"x\"\n",
        );
        assert_eq!(path, "menu[0]");
        assert!(message.contains("label"), "{message}");
    }

    #[test]
    fn unknown_type_is_rejected() {
        let (_path, message) =
            invalid("version = 1\n[[menu]]\nid = \"a\"\nlabel = \"A\"\ntype = \"keys\"\n");
        assert!(message.contains("keys"), "{message}");
    }

    #[test]
    fn exec_requires_command() {
        let (path, message) =
            invalid("version = 1\n[[menu]]\nid = \"a\"\nlabel = \"A\"\ntype = \"exec\"\n");
        assert_eq!(path, "menu[0]");
        assert!(message.contains("command"), "{message}");
    }

    #[test]
    fn url_requires_non_blank_url() {
        let (_path, message) = invalid(
            "version = 1\n[[menu]]\nid = \"a\"\nlabel = \"A\"\ntype = \"url\"\nurl = \" \"\n",
        );
        assert!(message.contains("url"), "{message}");
    }

    #[test]
    fn unknown_builtin_lists_valid_names() {
        let (_path, message) = invalid(
            "version = 1\n[[menu]]\nid = \"a\"\nlabel = \"A\"\ntype = \"builtin\"\nname = \"reboot\"\n",
        );
        assert!(message.contains("reboot"), "{message}");
        assert!(message.contains("close_all_windows"), "{message}");
    }

    #[test]
    fn submenu_requires_items() {
        let (path, _message) =
            invalid("version = 1\n[[menu]]\nid = \"a\"\nlabel = \"A\"\ntype = \"submenu\"\n");
        assert_eq!(path, "menu[0]");
    }

    #[test]
    fn field_not_valid_for_type_is_rejected() {
        let (path, message) = invalid(
            "version = 1\n[[menu]]\nid = \"a\"\nlabel = \"A\"\ntype = \"exec\"\ncommand = \"x\"\nurl = \"y\"\n",
        );
        assert_eq!(path, "menu[0]");
        assert!(message.contains("\"url\""), "{message}");
        assert!(message.contains("exec"), "{message}");
    }

    // ---- config contoh ----

    #[test]
    fn default_config_is_valid() {
        let config = parse(DEFAULT_CONFIG).unwrap();
        assert_eq!(config.themes, ThemeSet::builtin());
        assert!(!config.menu.is_empty());
    }

    #[test]
    fn default_config_offers_theme_picker_lock_and_close_all_windows() {
        let config = parse(DEFAULT_CONFIG).unwrap();
        let builtins: Vec<Builtin> = config
            .menu
            .iter()
            .filter_map(|item| match item.kind {
                ItemKind::Action(Action::Builtin(builtin)) => Some(builtin),
                _ => None,
            })
            .collect();
        for expected in [
            Builtin::ThemePicker,
            Builtin::Lock,
            Builtin::CloseAllWindows,
        ] {
            assert!(builtins.contains(&expected), "{expected:?} hilang");
        }
    }

    #[test]
    fn default_config_does_not_ship_the_sleep_item_enabled() {
        // "sleep" tidak bekerja di PC yang hanya punya Modern Standby.
        let config = parse(DEFAULT_CONFIG).unwrap();
        assert!(config.menu.iter().all(|item| item.id != "sleep"));
    }

    #[test]
    fn default_config_has_no_personal_paths() {
        for needle in ["PERSONAL", "ThinkPad", "Users\\", "AppData", "Obsidian"] {
            assert!(!DEFAULT_CONFIG.contains(needle), "{needle}");
        }
    }

    #[test]
    fn default_config_commented_examples_are_valid_when_uncommented() {
        // Menghapus "# " di depan blok contoh harus menghasilkan config valid,
        // supaya contoh di komentar tidak membusuk.
        let uncommented: String = DEFAULT_CONFIG
            .lines()
            .map(|line| match line.strip_prefix("# ") {
                Some(rest) if rest.starts_with("[[") || rest.contains(" = ") => rest,
                _ => line,
            })
            .collect::<Vec<_>>()
            .join("\n");
        let config = parse(&uncommented).unwrap();
        assert!(config.menu.iter().any(|item| item.id == "terminal-admin"));
        assert!(config.menu.iter().any(|item| item.id == "sleep"));
        assert!(config.themes.get("my-theme").is_some());
    }

    const THEME_BODY: &str =
        "fg = \"#FFFFFF\"\nsel_bg = \"#000000\"\nsel_fg = \"#FFFFFF\"\nbezel = \"#000000\"\n";

    fn with_themes(themes: &str) -> String {
        format!("{MINIMAL}\n{themes}")
    }

    #[test]
    fn custom_theme_is_added_with_default_font() {
        let source = with_themes(&format!(
            "[[themes]]\nname = \"mine\"\nbg = \"#101010\"\n{THEME_BODY}"
        ));
        let config = parse(&source).unwrap();
        assert_eq!(config.themes.len(), 8);
        let mine = config.themes.get("mine").unwrap();
        assert_eq!(mine.bg, Rgb::new(0x10, 0x10, 0x10));
        assert_eq!(mine.font, DEFAULT_FONT);
    }

    #[test]
    fn custom_theme_can_override_builtin_in_place() {
        let source = with_themes(&format!(
            "[[themes]]\nname = \"gruvbox\"\nbg = \"#010203\"\nfont = \"vt323\"\n{THEME_BODY}"
        ));
        let config = parse(&source).unwrap();
        assert_eq!(config.themes.len(), 7);
        let gruvbox = config.themes.get("gruvbox").unwrap();
        assert_eq!(gruvbox.bg, Rgb::new(1, 2, 3));
        assert_eq!(gruvbox.font, "vt323");
        assert_eq!(config.themes.iter().nth(4).unwrap().name, "gruvbox");
    }

    #[test]
    fn duplicate_theme_name_is_rejected() {
        let one = format!("[[themes]]\nname = \"mine\"\nbg = \"#101010\"\n{THEME_BODY}");
        let source = with_themes(&format!("{one}\n{one}"));
        let (path, _message) = invalid(&source);
        assert_eq!(path, "themes[1]");
    }
}
