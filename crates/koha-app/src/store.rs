//! Membaca dan menulis berkas config dan state. Satu-satunya tempat yang
//! menyentuh filesystem; `koha-core` hanya mengurai dan menghasilkan teks.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use koha_core::{Config, ConfigError, DEFAULT_CONFIG, State};
use thiserror::Error;

use crate::diagnostics;

/// Dari mana config yang dipakai berasal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigSource {
    File,
    /// Berkas tidak ada; config contoh bawaan dipakai dari memori.
    Builtin,
}

#[derive(Debug, Error)]
pub enum LoadError {
    #[error("tidak bisa membaca {}: {source}", .path.display())]
    Read { path: PathBuf, source: io::Error },
    #[error(
        "config tidak ditemukan: {}\n(lokasi ini ditentukan lewat --config atau KOHA_CONFIG)",
        .path.display()
    )]
    Missing { path: PathBuf },
    #[error("{}", diagnostics::render(.path, .text, .error))]
    Invalid {
        path: PathBuf,
        text: String,
        error: ConfigError,
    },
}

/// Membaca config dari `path`.
///
/// - Berkas tidak ada dan lokasinya bukan pilihan pengguna: pakai config contoh
///   bawaan, tanpa membuat berkas.
/// - Berkas tidak ada tetapi lokasinya dipilih pengguna (`explicit`): error.
/// - Berkas ada tetapi rusak: error yang menunjuk baris dan kolomnya. Tidak
///   pernah jatuh diam-diam ke bawaan, supaya menu tidak berubah tanpa sebab.
pub fn load_config(path: &Path, explicit: bool) -> Result<(Config, ConfigSource), LoadError> {
    match fs::read_to_string(path) {
        Ok(text) => {
            // Notepad dan beberapa editor menulis BOM UTF-8 di awal berkas.
            let text = text.strip_prefix('\u{feff}').unwrap_or(&text).to_owned();
            match Config::from_toml_str(&text) {
                Ok(config) => Ok((config, ConfigSource::File)),
                Err(error) => Err(LoadError::Invalid {
                    path: path.to_owned(),
                    text,
                    error,
                }),
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            if explicit {
                return Err(LoadError::Missing {
                    path: path.to_owned(),
                });
            }
            // DEFAULT_CONFIG dijamin valid oleh test di `koha-core`.
            let config =
                Config::from_toml_str(DEFAULT_CONFIG).map_err(|error| LoadError::Invalid {
                    path: path.to_owned(),
                    text: DEFAULT_CONFIG.to_owned(),
                    error,
                })?;
            Ok((config, ConfigSource::Builtin))
        }
        Err(source) => Err(LoadError::Read {
            path: path.to_owned(),
            source,
        }),
    }
}

/// Hasil membaca state: selalu ada state yang bisa dipakai.
#[derive(Debug, PartialEq, Eq)]
pub struct StateLoad {
    pub state: State,
    /// Terisi bila berkas ada tetapi tidak bisa dipakai (dan state default
    /// dipakai sebagai gantinya).
    pub warning: Option<String>,
}

/// State milik aplikasi, jadi kegagalan tidak boleh menghalangi KOHA berjalan:
/// berkas hilang berarti default, berkas rusak berarti default plus peringatan.
pub fn load_state(path: &Path) -> StateLoad {
    let fallback = |warning: String| StateLoad {
        state: State::default(),
        warning: Some(warning),
    };
    match fs::read_to_string(path) {
        Ok(text) => {
            let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
            match State::from_toml_str(text) {
                Ok(state) => StateLoad {
                    state,
                    warning: None,
                },
                Err(error) => fallback(format!(
                    "state di {} tidak bisa dipakai ({error}); memakai state default",
                    path.display()
                )),
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => StateLoad {
            state: State::default(),
            warning: None,
        },
        Err(error) => fallback(format!(
            "state di {} tidak bisa dibaca ({error}); memakai state default",
            path.display()
        )),
    }
}

/// Folder induk `path` bila itu folder sungguhan. `Path::new("state.toml").parent()`
/// adalah `Some("")` (bukan `None`), dan `create_dir_all("")` akan gagal, jadi
/// folder kosong disaring di sini.
fn real_parent(path: &Path) -> Option<&Path> {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
}

/// Menyimpan state secara atomik: tulis ke berkas sementara di folder yang
/// sama, lalu `rename` menimpa berkas lama. Berkas state tidak pernah
/// setengah tertulis, bahkan bila komputer mati di tengah penulisan.
pub fn save_state(path: &Path, state: &State) -> io::Result<()> {
    let text = state.to_toml_string().map_err(io::Error::other)?;
    if let Some(parent) = real_parent(path) {
        fs::create_dir_all(parent)?;
    }
    let mut temp = path.as_os_str().to_owned();
    temp.push(".tmp");
    let temp = PathBuf::from(temp);

    let result = fs::write(&temp, text).and_then(|()| fs::rename(&temp, path));
    if result.is_err() {
        let _ = fs::remove_file(&temp); // bersih-bersih, kegagalannya tidak penting
    }
    result
}

#[derive(Debug, PartialEq, Eq)]
pub enum InitOutcome {
    Created,
    AlreadyExists,
}

/// Menulis config contoh ke `path` bila belum ada. Tidak pernah menimpa.
///
/// `create_new(true)` membuat pengecekan "belum ada" dan pembuatan berkas
/// menjadi satu langkah atomik di OS, jadi tidak ada celah balapan (race)
/// antara memeriksa dan menulis.
pub fn init_config(path: &Path) -> io::Result<InitOutcome> {
    if let Some(parent) = real_parent(path) {
        fs::create_dir_all(parent)?;
    }
    match OpenOptions::new().write(true).create_new(true).open(path) {
        Ok(mut file) => {
            file.write_all(DEFAULT_CONFIG.as_bytes())?;
            Ok(InitOutcome::Created)
        }
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            Ok(InitOutcome::AlreadyExists)
        }
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn themed(name: &str) -> State {
        State {
            theme: Some(name.to_owned()),
            ..State::default()
        }
    }

    // ---- config ----

    #[test]
    fn missing_default_config_falls_back_to_the_builtin_without_creating_a_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let (config, source) = load_config(&path, false).unwrap();
        assert_eq!(source, ConfigSource::Builtin);
        assert_eq!(config, Config::from_toml_str(DEFAULT_CONFIG).unwrap());
        assert!(!path.exists());
    }

    #[test]
    fn missing_explicit_config_is_an_error() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nope.toml");
        let error = load_config(&path, true).unwrap_err();
        assert!(matches!(error, LoadError::Missing { .. }), "{error}");
        assert!(error.to_string().contains("nope.toml"));
    }

    #[test]
    fn valid_config_file_is_loaded() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(
            &path,
            "version = 1\n[[menu]]\nid = \"only\"\nlabel = \"Only\"\ntype = \"url\"\nurl = \"x\"\n",
        )
        .unwrap();
        let (config, source) = load_config(&path, false).unwrap();
        assert_eq!(source, ConfigSource::File);
        assert_eq!(config.menu.len(), 1);
        assert_eq!(config.menu[0].id, "only");
    }

    #[test]
    fn config_with_a_utf8_bom_is_loaded() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, format!("\u{feff}{DEFAULT_CONFIG}")).unwrap();
        assert!(load_config(&path, false).is_ok());
    }

    #[test]
    fn config_with_windows_line_endings_is_loaded() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, DEFAULT_CONFIG.replace('\n', "\r\n")).unwrap();
        assert!(load_config(&path, false).is_ok());
    }

    #[test]
    fn broken_config_is_an_error_naming_file_line_and_column() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, "version = 1\nbogus = 1\n").unwrap();
        let error = load_config(&path, false).unwrap_err();
        let message = error.to_string();
        assert!(matches!(error, LoadError::Invalid { .. }));
        assert!(message.contains("config.toml:2:1"), "{message}");
        assert!(message.contains("bogus = 1"), "{message}");
    }

    #[test]
    fn broken_config_never_falls_back_to_the_builtin() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, "not toml at all [[[").unwrap();
        assert!(load_config(&path, false).is_err());
    }

    #[test]
    fn unreadable_config_path_is_a_read_error() {
        // Sebuah folder bukan berkas: dibaca sebagai berkas akan gagal (bukan NotFound).
        let dir = tempdir().unwrap();
        let error = load_config(dir.path(), false).unwrap_err();
        assert!(matches!(error, LoadError::Read { .. }), "{error}");
    }

    // ---- state ----

    #[test]
    fn missing_state_is_the_default_without_a_warning() {
        let dir = tempdir().unwrap();
        let loaded = load_state(&dir.path().join("state.toml"));
        assert_eq!(loaded.state, State::default());
        assert_eq!(loaded.warning, None);
    }

    #[test]
    fn state_roundtrips_through_the_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("state.toml");
        let mut state = themed("gruvbox");
        state.hidden.insert("lock".to_owned());
        save_state(&path, &state).unwrap();
        let loaded = load_state(&path);
        assert_eq!(loaded.state, state);
        assert_eq!(loaded.warning, None);
    }

    #[test]
    fn broken_state_falls_back_to_the_default_with_a_warning() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("state.toml");
        fs::write(&path, "theme = [[[").unwrap();
        let loaded = load_state(&path);
        assert_eq!(loaded.state, State::default());
        let warning = loaded.warning.expect("harus ada peringatan");
        assert!(warning.contains("state.toml"), "{warning}");
    }

    #[test]
    fn unsupported_state_version_falls_back_with_a_warning() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("state.toml");
        fs::write(&path, "version = 99\n").unwrap();
        assert!(load_state(&path).warning.is_some());
    }

    #[test]
    fn saving_creates_missing_parent_folders() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("a").join("b").join("state.toml");
        save_state(&path, &themed("vague")).unwrap();
        assert_eq!(load_state(&path).state, themed("vague"));
    }

    #[test]
    fn saving_overwrites_and_leaves_no_temp_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("state.toml");
        save_state(&path, &themed("amber")).unwrap();
        save_state(&path, &themed("tokyonight")).unwrap();
        assert_eq!(load_state(&path).state, themed("tokyonight"));
        let names: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, ["state.toml"]);
    }

    #[test]
    fn real_parent_ignores_the_empty_parent_of_a_bare_filename() {
        assert_eq!(real_parent(Path::new("state.toml")), None);
        assert_eq!(real_parent(Path::new("")), None);
        assert_eq!(real_parent(Path::new("a/state.toml")), Some(Path::new("a")));
    }

    // ---- init ----

    #[test]
    fn init_writes_the_sample_config_and_it_loads() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("koha").join("config.toml");
        assert_eq!(init_config(&path).unwrap(), InitOutcome::Created);
        assert_eq!(fs::read_to_string(&path).unwrap(), DEFAULT_CONFIG);
        let (_, source) = load_config(&path, true).unwrap();
        assert_eq!(source, ConfigSource::File);
    }

    #[test]
    fn init_never_overwrites_an_existing_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, "# milik saya\n").unwrap();
        assert_eq!(init_config(&path).unwrap(), InitOutcome::AlreadyExists);
        assert_eq!(fs::read_to_string(&path).unwrap(), "# milik saya\n");
    }

    #[test]
    fn init_twice_is_safe() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        assert_eq!(init_config(&path).unwrap(), InitOutcome::Created);
        assert_eq!(init_config(&path).unwrap(), InitOutcome::AlreadyExists);
    }
}
