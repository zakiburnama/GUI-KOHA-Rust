// Build rilis memakai subsistem GUI: tidak ada jendela konsol yang berkedip saat
// KOHA dipanggil dari tombol atau pintasan. Build debug tetap berkonsol supaya
// log terlihat saat `cargo run`.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cli;
mod diagnostics;
mod paths;
mod report;
mod store;

use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::Parser;
use koha_core::{MenuState, State};
use koha_gui::trace;
use koha_platform::PlatformError;

use crate::cli::Cli;
use crate::paths::{Dirs, Env};
use crate::store::{ConfigSource, InitOutcome};

fn main() -> ExitCode {
    trace::start(); // titik nol jejak waktu start (kosong tanpa fitur `startup-trace`)
    // Pertama-tama: menempel ke konsol induk (bila ada), supaya `--help`,
    // `--print-config-path`, dan pesan error terlihat saat dijalankan dari terminal.
    let has_console = koha_platform::attach_parent_console();
    trace::mark("console attached");
    match run(has_console) {
        Ok(code) => code,
        Err(error) => {
            report::error(has_console, &format!("{error:#}"));
            ExitCode::FAILURE
        }
    }
}

fn run(has_console: bool) -> Result<ExitCode> {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        // `--help` dan `--version` bukan kesalahan: cetak lalu selesai.
        Err(error) if !error.use_stderr() => {
            let _ = error.print();
            return Ok(ExitCode::SUCCESS);
        }
        Err(error) => {
            report::error(has_console, &error.render().to_string());
            return Ok(ExitCode::from(2));
        }
    };
    trace::mark("args parsed");
    let paths = paths::resolve(&cli, &Env::from_process(), Dirs::system().as_ref())?;
    trace::mark("paths resolved");

    if cli.print_config_path {
        println!("{}", paths.config.display());
        return Ok(ExitCode::SUCCESS);
    }
    if cli.print_state_path {
        println!("{}", paths.state.display());
        return Ok(ExitCode::SUCCESS);
    }
    if cli.init {
        let outcome = store::init_config(&paths.config)
            .with_context(|| format!("gagal menulis {}", paths.config.display()))?;
        match outcome {
            InitOutcome::Created => {
                println!("Config contoh ditulis ke {}", paths.config.display())
            }
            InitOutcome::AlreadyExists => {
                println!("{} sudah ada; tidak ditimpa.", paths.config.display())
            }
        }
        return Ok(ExitCode::SUCCESS);
    }

    let (config, source) = store::load_config(&paths.config, paths.config_explicit)?;
    trace::mark("config loaded");
    if source == ConfigSource::Builtin {
        eprintln!(
            "info: {} belum ada; memakai menu contoh bawaan (jalankan `koha --init` untuk membuatnya)",
            paths.config.display()
        );
    }

    let loaded = store::load_state(&paths.state);
    if let Some(warning) = &loaded.warning {
        eprintln!("peringatan: {warning}");
    }
    trace::mark("state loaded");
    let menu = MenuState::new(config, loaded.state);
    trace::mark("menu built");

    // State disimpan segera setiap kali berubah (tema diganti, item di-ON/OFF).
    // Kegagalan menyimpan hanya peringatan: menu tetap berjalan.
    let mut last_saved = menu.state().clone();
    let mut on_state = |state: &State| {
        if *state == last_saved {
            return;
        }
        match store::save_state(&paths.state, state) {
            Ok(()) => last_saved = state.clone(),
            Err(error) => eprintln!(
                "peringatan: gagal menyimpan state ke {}: {error}",
                paths.state.display()
            ),
        }
    };

    // `run` baru kembali setelah jendela dihancurkan, jadi aksi di bawah ini
    // dijalankan sesudah fokus kembali ke aplikasi sebelumnya.
    if let Some(action) = koha_gui::run(menu, &mut on_state)? {
        let platform = koha_platform::SystemPlatform::default();
        match koha_platform::execute(&platform, &action) {
            // Pengguna menolak prompt UAC: pilihannya sendiri, bukan error.
            Ok(()) | Err(PlatformError::Cancelled) => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(ExitCode::SUCCESS)
}
