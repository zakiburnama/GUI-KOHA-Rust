//! Mode kering untuk "Close All Windows": mencetak jendela yang AKAN ditutup,
//! tanpa menutup apa pun. Jalankan dengan:
//!
//!     cargo run -p koha-platform --example list_windows

#[cfg(windows)]
fn main() {
    let windows = koha_platform::closable_windows();
    println!(
        "{} jendela akan ditutup oleh Close All Windows:\n",
        windows.len()
    );
    for window in &windows {
        println!(
            "  pid {:>6}  kelas {:<28}  hwnd 0x{:x}",
            window.pid, window.class, window.hwnd
        );
    }
    println!("\n(Tidak ada yang ditutup; ini hanya daftar.)");
}

#[cfg(not(windows))]
fn main() {
    println!("Contoh ini hanya berarti di Windows.");
}
