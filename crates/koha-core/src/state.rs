//! State yang ditulis aplikasi (bukan diedit pengguna): tema aktif dan item
//! menu yang di-OFF. Dipisah dari `Config` supaya `config.toml` tetap bersih
//! dan tidak ditimpa aplikasi.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::config::ConfigError;

/// Versi format state yang dipahami kode ini.
pub const STATE_VERSION: u32 = 1;

fn default_version() -> u32 {
    STATE_VERSION
}

/// `#[serde(default)]` membuat field yang hilang di file memakai nilai default,
/// jadi file kosong atau file lama tetap terbaca. Field tak dikenal diabaikan
/// (tidak ada `deny_unknown_fields`) karena file ini ditulis aplikasi sendiri.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    #[serde(default = "default_version")]
    pub version: u32,
    /// Nama tema aktif. `None` = tema bawaan ([`crate::DEFAULT_THEME`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    /// `id` item menu utama yang di-OFF. `BTreeSet` menyimpan terurut, jadi isi
    /// file stabil dan mudah dibaca. Id yang sudah tidak ada di config sengaja
    /// tidak dibuang, supaya item yang dihapus lalu dikembalikan tidak kehilangan
    /// statusnya.
    #[serde(default)]
    pub hidden: BTreeSet<String>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            version: STATE_VERSION,
            theme: None,
            hidden: BTreeSet::new(),
        }
    }
}

impl State {
    /// File kosong menghasilkan state default. Jika file rusak, pemanggil
    /// (`koha-app`) memutuskan: umumnya jatuh ke default dan tetap jalan.
    pub fn from_toml_str(source: &str) -> Result<Self, ConfigError> {
        let state: Self = toml::from_str(source).map_err(|e| ConfigError::from_toml(&e, source))?;
        if state.version != STATE_VERSION {
            return Err(ConfigError::invalid(
                "version",
                format!(
                    "versi state {} tidak didukung (yang didukung: {STATE_VERSION})",
                    state.version
                ),
            ));
        }
        Ok(state)
    }

    pub fn to_toml_string(&self) -> Result<String, ConfigError> {
        toml::to_string(self).map_err(|e| ConfigError::Toml {
            message: e.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_file_gives_default_state() {
        assert_eq!(State::from_toml_str("").unwrap(), State::default());
    }

    #[test]
    fn roundtrips_theme_and_hidden() {
        let state = State {
            theme: Some("gruvbox".to_owned()),
            hidden: ["sleep", "lock"].map(String::from).into(),
            ..State::default()
        };

        let text = state.to_toml_string().unwrap();
        assert_eq!(State::from_toml_str(&text).unwrap(), state);
    }

    #[test]
    fn hidden_ids_are_written_sorted() {
        let mut state = State::default();
        state.hidden.insert("zeta".to_owned());
        state.hidden.insert("alpha".to_owned());
        let text = state.to_toml_string().unwrap();
        assert!(
            text.find("alpha").unwrap() < text.find("zeta").unwrap(),
            "{text}"
        );
    }

    #[test]
    fn unset_theme_is_not_written() {
        let text = State::default().to_toml_string().unwrap();
        assert!(!text.contains("theme"), "{text}");
    }

    #[test]
    fn missing_fields_use_defaults() {
        let state = State::from_toml_str("theme = \"vague\"\n").unwrap();
        assert_eq!(state.theme.as_deref(), Some("vague"));
        assert!(state.hidden.is_empty());
        assert_eq!(state.version, STATE_VERSION);
    }

    #[test]
    fn unknown_fields_are_ignored() {
        let state = State::from_toml_str("future_option = true\n").unwrap();
        assert_eq!(state, State::default());
    }

    #[test]
    fn unsupported_version_is_rejected() {
        let err = State::from_toml_str("version = 2\n").unwrap_err();
        assert!(matches!(err, ConfigError::Invalid { ref path, .. } if path == "version"));
    }

    #[test]
    fn broken_toml_reports_position() {
        let err = State::from_toml_str("theme = \"ok\"\nhidden = [\n").unwrap_err();
        assert!(matches!(err, ConfigError::Syntax { .. }), "{err:?}");
    }
}
