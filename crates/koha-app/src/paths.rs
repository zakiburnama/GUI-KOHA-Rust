//! Menentukan lokasi config dan state.
//!
//! Environment dan folder dasar sistem **disuntikkan** (dioper sebagai nilai,
//! bukan dibaca langsung di dalam fungsi), sehingga aturannya bisa dites tanpa
//! menyentuh variabel lingkungan atau folder pengguna yang sebenarnya.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::cli::Cli;

/// Nilai variabel lingkungan yang relevan. Nilai kosong dianggap tidak diset.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Env {
    koha_config: Option<OsString>,
    koha_state: Option<OsString>,
}

impl Env {
    pub fn new(koha_config: Option<OsString>, koha_state: Option<OsString>) -> Self {
        let non_empty = |value: Option<OsString>| value.filter(|v| !v.is_empty());
        Self {
            koha_config: non_empty(koha_config),
            koha_state: non_empty(koha_state),
        }
    }

    pub fn from_process() -> Self {
        Self::new(
            std::env::var_os("KOHA_CONFIG"),
            std::env::var_os("KOHA_STATE"),
        )
    }
}

/// Folder standar tempat KOHA menyimpan berkasnya (sudah termasuk `koha`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dirs {
    pub config_dir: PathBuf,
    pub state_dir: PathBuf,
}

impl Dirs {
    /// Windows: config di `%APPDATA%\koha` (Roaming), state di `%LOCALAPPDATA%\koha`
    /// (khusus mesin ini). Linux: `~/.config/koha` dan `~/.local/state/koha`.
    /// macOS: `~/Library/Application Support/koha` untuk keduanya.
    ///
    /// `BaseDirs` dipakai, bukan `ProjectDirs`, karena `ProjectDirs` menambah
    /// subfolder `config` di Windows. `None` bila folder rumah tidak bisa
    /// ditentukan.
    pub fn system() -> Option<Self> {
        let base = directories::BaseDirs::new()?;
        // `state_dir()` hanya ada di Linux; selain itu pakai folder data lokal.
        let state = base.state_dir().unwrap_or_else(|| base.data_local_dir());
        Some(Self {
            config_dir: base.config_dir().join("koha"),
            state_dir: state.join("koha"),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paths {
    pub config: PathBuf,
    /// `true` bila lokasi config ditentukan pengguna (`--config`/`KOHA_CONFIG`).
    /// Berkas yang tidak ada lalu dianggap kesalahan, bukan jatuh ke bawaan.
    pub config_explicit: bool,
    pub state: PathBuf,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PathError {
    #[error("tidak bisa menentukan folder rumah pengguna; berikan --config dan --state")]
    NoHomeDir,
}

/// Prioritas: flag, lalu variabel lingkungan, lalu lokasi standar.
///
/// Bila config ditentukan pengguna dan state tidak, state diletakkan **di
/// sebelah config itu** (`state.toml`), sehingga mencoba config alternatif tidak
/// menimpa state harian dan setup portabel tetap konsisten.
pub fn resolve(cli: &Cli, env: &Env, dirs: Option<&Dirs>) -> Result<Paths, PathError> {
    let explicit_config = cli
        .config
        .clone()
        .or_else(|| env.koha_config.clone().map(PathBuf::from));
    let explicit_state = cli
        .state
        .clone()
        .or_else(|| env.koha_state.clone().map(PathBuf::from));

    let config_explicit = explicit_config.is_some();
    let config = match explicit_config {
        Some(path) => path,
        None => dirs
            .ok_or(PathError::NoHomeDir)?
            .config_dir
            .join("config.toml"),
    };
    let state = match explicit_state {
        Some(path) => path,
        None if config_explicit => config
            .parent()
            .unwrap_or_else(|| Path::new(""))
            .join("state.toml"),
        None => dirs
            .ok_or(PathError::NoHomeDir)?
            .state_dir
            .join("state.toml"),
    };

    Ok(Paths {
        config,
        config_explicit,
        state,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn cli(args: &[&str]) -> Cli {
        Cli::try_parse_from(std::iter::once("koha").chain(args.iter().copied())).unwrap()
    }

    fn dirs() -> Dirs {
        Dirs {
            config_dir: PathBuf::from("/home/u/cfg"),
            state_dir: PathBuf::from("/home/u/state"),
        }
    }

    fn env(config: Option<&str>, state: Option<&str>) -> Env {
        Env::new(config.map(OsString::from), state.map(OsString::from))
    }

    #[test]
    fn defaults_use_the_standard_folders() {
        let paths = resolve(&cli(&[]), &Env::default(), Some(&dirs())).unwrap();
        assert_eq!(
            paths.config,
            PathBuf::from("/home/u/cfg").join("config.toml")
        );
        assert_eq!(
            paths.state,
            PathBuf::from("/home/u/state").join("state.toml")
        );
        assert!(!paths.config_explicit);
    }

    #[test]
    fn environment_overrides_the_default() {
        let paths = resolve(
            &cli(&[]),
            &env(Some("/e/c.toml"), Some("/e/s.toml")),
            Some(&dirs()),
        )
        .unwrap();
        assert_eq!(paths.config, PathBuf::from("/e/c.toml"));
        assert_eq!(paths.state, PathBuf::from("/e/s.toml"));
        assert!(paths.config_explicit);
    }

    #[test]
    fn flag_overrides_the_environment() {
        let paths = resolve(
            &cli(&["--config", "/f/c.toml", "--state", "/f/s.toml"]),
            &env(Some("/e/c.toml"), Some("/e/s.toml")),
            Some(&dirs()),
        )
        .unwrap();
        assert_eq!(paths.config, PathBuf::from("/f/c.toml"));
        assert_eq!(paths.state, PathBuf::from("/f/s.toml"));
    }

    #[test]
    fn explicit_config_puts_state_next_to_it() {
        let paths = resolve(
            &cli(&["--config", "/work/koha/c.toml"]),
            &Env::default(),
            Some(&dirs()),
        )
        .unwrap();
        assert_eq!(paths.state, PathBuf::from("/work/koha").join("state.toml"));
    }

    #[test]
    fn config_from_environment_also_puts_state_next_to_it() {
        let paths = resolve(
            &cli(&[]),
            &env(Some("/work/koha/c.toml"), None),
            Some(&dirs()),
        )
        .unwrap();
        assert_eq!(paths.state, PathBuf::from("/work/koha").join("state.toml"));
    }

    #[test]
    fn explicit_state_wins_over_next_to_config() {
        let paths = resolve(
            &cli(&["--config", "/work/c.toml", "--state", "/elsewhere/s.toml"]),
            &Env::default(),
            Some(&dirs()),
        )
        .unwrap();
        assert_eq!(paths.state, PathBuf::from("/elsewhere/s.toml"));
    }

    #[test]
    fn explicit_state_alone_keeps_the_default_config() {
        let paths = resolve(
            &cli(&["--state", "/s/s.toml"]),
            &Env::default(),
            Some(&dirs()),
        )
        .unwrap();
        assert_eq!(
            paths.config,
            PathBuf::from("/home/u/cfg").join("config.toml")
        );
        assert!(!paths.config_explicit);
        assert_eq!(paths.state, PathBuf::from("/s/s.toml"));
    }

    #[test]
    fn relative_config_name_puts_state_in_the_working_directory() {
        let paths = resolve(&cli(&["--config", "c.toml"]), &Env::default(), None).unwrap();
        assert_eq!(paths.state, PathBuf::from("state.toml"));
    }

    #[test]
    fn empty_environment_values_are_ignored() {
        let paths = resolve(&cli(&[]), &env(Some(""), Some("")), Some(&dirs())).unwrap();
        assert!(!paths.config_explicit);
        assert_eq!(
            paths.config,
            PathBuf::from("/home/u/cfg").join("config.toml")
        );
    }

    #[test]
    fn works_without_home_dir_when_both_are_explicit() {
        let paths = resolve(
            &cli(&["--config", "/c.toml", "--state", "/s.toml"]),
            &Env::default(),
            None,
        )
        .unwrap();
        assert_eq!(paths.config, PathBuf::from("/c.toml"));
    }

    #[test]
    fn missing_home_dir_is_an_error_when_a_default_is_needed() {
        assert_eq!(
            resolve(&cli(&[]), &Env::default(), None),
            Err(PathError::NoHomeDir)
        );
        assert_eq!(
            resolve(&cli(&["--state", "/s.toml"]), &Env::default(), None),
            Err(PathError::NoHomeDir)
        );
    }

    #[test]
    fn system_dirs_end_in_koha() {
        // Bergantung pada mesin, jadi hanya bentuknya yang diperiksa.
        if let Some(dirs) = Dirs::system() {
            assert_eq!(dirs.config_dir.file_name().unwrap(), "koha");
            assert_eq!(dirs.state_dir.file_name().unwrap(), "koha");
        }
    }
}
