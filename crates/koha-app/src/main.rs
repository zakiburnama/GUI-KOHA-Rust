mod cli;
mod diagnostics;
mod paths;
mod store;

use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::Parser;
use koha_core::{MenuState, State};
use koha_platform::PlatformError;

use crate::cli::Cli;
use crate::paths::{Dirs, Env};
use crate::store::{ConfigSource, InitOutcome};

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<ExitCode> {
    let cli = Cli::parse();
    let paths = paths::resolve(&cli, &Env::from_process(), Dirs::system().as_ref())?;

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
    let menu = MenuState::new(config, loaded.state);

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
