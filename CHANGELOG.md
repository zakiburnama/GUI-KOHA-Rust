# Riwayat perubahan

Format mengikuti [Keep a Changelog](https://keepachangelog.com/id-ID/1.1.0/), dan versi
mengikuti [Semantic Versioning](https://semver.org/lang/id/). Selama versi masih `0.x`,
format config dan perilaku bisa berubah tanpa pemberitahuan sebelumnya.

## [Unreleased]

## [0.1.0] - belum dirilis

Rilis pertama: penulisan ulang KOHA (versi AutoHotkey) dalam Rust. **Hanya Windows.**

### Ditambahkan

- Popup menu tanpa border dan selalu di atas, ditengahkan di monitor utama, dengan navigasi
  keyboard (`↑ ↓ Tab Shift+Tab Enter Esc`) dan tertutup saat kehilangan fokus atau saat tombol
  lain ditekan. Mendukung skala DPI.
- Submenu bertingkat, dan baris **Menu Settings** otomatis untuk menyembunyikan atau
  menampilkan item menu utama (disimpan berdasarkan `id`).
- Tujuh tema bawaan (`game_boy`, `amber`, `green_term`, `catppuccin-mocha`, `gruvbox`,
  `vague`, `tokyonight`), pemilih tema yang berlaku langsung, dan tema buatan sendiri.
  Setiap tema membawa fontnya sendiri; tiga font berlisensi OFL (`Press Start 2P`, `VT323`,
  `IBM Plex Mono`) tertanam di binary.
- Config TOML dengan validasi dan pesan error yang menunjuk berkas, baris, dan kolom (atau
  jalur item untuk kesalahan aturan). State (tema aktif, item yang di-OFF) disimpan terpisah
  dan ditulis secara atomik.
- Aksi: `exec` (dengan `args`, `admin` lewat UAC, dan perluasan `%VAR%`), `url`, dan aksi bawaan
  `lock`, `sleep`, `close_all_windows`, serta `theme_picker`.
- Baris perintah: `--config`, `--state`, `--init`, `--print-config-path`, `--print-state-path`;
  variabel lingkungan `KOHA_CONFIG` dan `KOHA_STATE`. Config yang tidak ada digantikan menu
  contoh bawaan, sedangkan config yang rusak menampilkan error (tidak diam-diam jatuh ke
  bawaan).
- Build rilis tanpa jendela konsol, dengan dialog error saat dipanggil tanpa konsol dan
  penempelan ke konsol induk saat dijalankan dari terminal.
- Profil rilis yang disetel dari pengukuran (LTO, `codegen-units = 1`, `panic = "abort"`,
  `strip`): binary sekitar 1,5 MB.
- `tools/install-shortcut.ps1`: memasang `koha.exe` ke `%LOCALAPPDATA%\Programs\koha` dan membuat
  pintasan Start Menu (untuk Lenovo Vantage); menolak build debug dan menolak menimpa pintasan
  milik program lain.
- Alat pengembangan: `tools/bench-startup.ps1` dan fitur Cargo `startup-trace` untuk mengukur
  waktu start, serta `tools/gen-third-party.ps1` untuk `THIRD_PARTY_LICENSES.md`.
- CI (Windows wajib; Ubuntu dan macOS eksperimental), kebijakan `cargo deny`, dan dokumentasi
  (`README`, `CONTRIBUTING`, `docs/`).

### Keterbatasan yang diketahui

Lihat [README](README.md#keterbatasan-yang-diketahui). Yang paling penting: hanya Windows;
`sleep` tidak bekerja di PC Modern Standby tanpa hibernasi; peluncuran pertama berkas baru bisa
lambat; binary tidak ditandatangani; belum ada lisensi.

### Keamanan

- `ttf-parser` (lewat `fontdue`) ditandai tidak lagi dipelihara (RUSTSEC-2026-0192). Bukan
  kerentanan yang diketahui, dan hanya membaca font yang ditanam sendiri; pengecualian dan
  alasannya tercatat di [deny.toml](deny.toml).

[Unreleased]: https://github.com/zakiburnama/GUI-KOHA_Rust/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/zakiburnama/GUI-KOHA_Rust/releases/tag/v0.1.0
