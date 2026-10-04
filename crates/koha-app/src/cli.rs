//! Argumen baris perintah.

use std::path::PathBuf;

use clap::{ArgGroup, Parser};

// `#[derive(Parser)]` membuat parser argumen dari struct ini, termasuk `--help`
// dan `--version`. Teks bantuan diambil dari `about` dan komentar `///` di tiap
// field, jadi catatan untuk developer ditulis dengan `//` biasa seperti ini.
// Satu `ArgGroup` memastikan paling banyak satu dari `--init`,
// `--print-config-path`, dan `--print-state-path` dipakai sekaligus.
#[derive(Debug, Parser, PartialEq, Eq)]
#[command(
    name = "koha",
    version,
    about = "KOHA (Kwik One-Hotkey Access): popup menu seperti rofi. Dipanggil, memilih satu aksi, lalu selesai; tidak ada proses yang tertinggal di background.",
    group(ArgGroup::new("mode").args(["init", "print_config_path", "print_state_path"]))
)]
pub struct Cli {
    /// Pakai berkas config ini (menimpa KOHA_CONFIG dan lokasi standar)
    #[arg(long, value_name = "PATH")]
    pub config: Option<PathBuf>,

    /// Pakai berkas state ini (menimpa KOHA_STATE dan lokasi standar)
    #[arg(long, value_name = "PATH")]
    pub state: Option<PathBuf>,

    /// Tulis config contoh bila belum ada (tidak pernah menimpa)
    #[arg(long)]
    pub init: bool,

    /// Cetak lokasi config yang dipakai, lalu keluar
    #[arg(long)]
    pub print_config_path: bool,

    /// Cetak lokasi state yang dipakai, lalu keluar
    #[arg(long)]
    pub print_state_path: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
        Cli::try_parse_from(std::iter::once("koha").chain(args.iter().copied()))
    }

    #[test]
    fn no_arguments_means_run_the_menu() {
        let cli = parse(&[]).unwrap();
        assert_eq!(cli.config, None);
        assert_eq!(cli.state, None);
        assert!(!cli.init && !cli.print_config_path && !cli.print_state_path);
    }

    #[test]
    fn parses_config_and_state_paths() {
        let cli = parse(&["--config", "a.toml", "--state", "b.toml"]).unwrap();
        assert_eq!(cli.config, Some(PathBuf::from("a.toml")));
        assert_eq!(cli.state, Some(PathBuf::from("b.toml")));
    }

    #[test]
    fn parses_each_mode_flag() {
        assert!(parse(&["--init"]).unwrap().init);
        assert!(parse(&["--print-config-path"]).unwrap().print_config_path);
        assert!(parse(&["--print-state-path"]).unwrap().print_state_path);
    }

    #[test]
    fn mode_flags_are_mutually_exclusive() {
        assert!(parse(&["--init", "--print-config-path"]).is_err());
        assert!(parse(&["--print-config-path", "--print-state-path"]).is_err());
    }

    #[test]
    fn mode_flag_can_be_combined_with_a_custom_config() {
        let cli = parse(&["--print-config-path", "--config", "x.toml"]).unwrap();
        assert!(cli.print_config_path);
        assert_eq!(cli.config, Some(PathBuf::from("x.toml")));
    }

    #[test]
    fn unknown_flag_is_rejected() {
        assert!(parse(&["--nope"]).is_err());
    }

    #[test]
    fn config_requires_a_value() {
        assert!(parse(&["--config"]).is_err());
    }
}
