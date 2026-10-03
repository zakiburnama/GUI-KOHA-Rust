use anyhow::{Context, Result};
use koha_core::{Config, DEFAULT_CONFIG, MenuState, State};

fn main() -> Result<()> {
    // Sementara: config contoh bawaan dan state kosong. Membaca file config
    // dan state menyusul di langkah 5.
    let config = Config::from_toml_str(DEFAULT_CONFIG).context("config contoh tidak valid")?;
    let menu = MenuState::new(config, State::default());

    let mut on_state = |state: &State| eprintln!("state berubah: {state:?}");
    if let Some(action) = koha_gui::run(menu, &mut on_state)? {
        // Menjalankan aksi sungguhan menyusul di langkah 6.
        println!("aksi dipilih: {action:?}");
    }
    Ok(())
}
