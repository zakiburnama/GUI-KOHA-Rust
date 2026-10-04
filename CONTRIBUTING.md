# Berkontribusi

Terima kasih sudah tertarik. KOHA adalah proyek yang masih muda (0.1.0) dan juga proyek
belajar Rust, jadi kode dan komentarnya sengaja menjelaskan *mengapa*, bukan hanya *apa*.

> **Soal lisensi:** KOHA belum memiliki lisensi (lihat [README](README.md#lisensi)), sehingga
> belum ada syarat yang jelas untuk menerima kontribusi kode. Sampai penulis menentukannya,
> silakan membuka issue untuk berdiskusi atau melaporkan masalah dulu, dan tunggu kabar
> sebelum mengirim perubahan besar.

## Menyiapkan lingkungan

- [Rust](https://rustup.rs) stabil. Dikembangkan dengan Rust 1.99; edition 2024 membutuhkan
  1.85 atau lebih baru (versi minimum sebenarnya belum diuji).
- Windows: **Visual Studio Build Tools** dengan "Desktop development with C++" (MSVC dan
  Windows SDK). Hanya Windows yang dikerjakan saat ini; lihat
  [docs/platforms.md](docs/platforms.md).
- Opsional: `cargo install cargo-deny --locked` untuk pemeriksaan lisensi dan advisory.

```powershell
git clone https://github.com/zakiburnama/GUI-KOHA_Rust.git
cd GUI-KOHA_Rust
cargo build
cargo run          # build debug berkonsol, jadi pesan log terlihat
```

## Struktur dan aturan dependensi

| Crate | Tanggung jawab |
|---|---|
| `koha-core` | Logika murni: config (`serde`/TOML), menu (state machine), tema, state. **Tanpa panggilan OS dan tanpa GUI.** |
| `koha-platform` | Trait `Platform` dan implementasi per OS, dipilih dengan `cfg`. |
| `koha-gui` | Jendela (`winit` + `softbuffer`), tata letak, render teks, font. |
| `koha-app` | Binary `koha`: CLI, lokasi berkas, membaca dan menulis config/state, merangkai semuanya. |

Aturannya: `koha-core` tidak bergantung ke crate lain; `koha-platform` dan `koha-gui`
bergantung ke `koha-core`; hanya `koha-app` yang mengenal semuanya. Jangan menaruh kode khusus
OS di `koha-core`. Logika yang bisa dipisah dari OS dan jendela (pembentukan perintah,
tata letak, render ke buffer) ditulis sebagai fungsi murni agar bisa dites tanpa
membuka jendela.

Galat: `thiserror` di crate library, `anyhow` hanya di `koha-app`.

## Sebelum mengirim perubahan

Semua ini dijalankan CI juga, dan harus hijau:

```powershell
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo clippy --workspace --all-targets --features koha-app/startup-trace -- -D warnings
cargo test -p koha-gui --features startup-trace
cargo doc --workspace --no-deps
cargo deny check
.\tools\gen-third-party.ps1 -Check
```

- Peringatan `clippy` dianggap error (`-D warnings`). Perbaiki penyebabnya, jangan menambah
  `#[allow]` tanpa alasan tertulis.
- `cargo deny check` memeriksa lisensi, advisory keamanan, dan sumber dependensi
  (aturannya di [deny.toml](deny.toml)). Peringatan versi ganda wajar.
- `THIRD_PARTY_LICENSES.md` dihasilkan dari `Cargo.lock`. Bila kamu menambah atau mengubah
  dependensi atau font, jalankan `.\tools\gen-third-party.ps1` (tanpa `-Check`) dan sertakan
  hasilnya.

## Menulis tes

- Tes unit ada di modul `#[cfg(test)]` di berkas yang sama dengan kodenya. Sasarannya
  perilaku, termasuk kasus tepi dan jalur gagal, bukan sekadar jalur sukses.
- **Tes tidak boleh menyentuh config atau state asli pengguna.** Pakai folder sementara
  (`tempfile`) atau opsi `--state` / `--config` ke folder sementara.
- **Tes snapshot teks** ada di `crates/koha-gui/src/text.rs` (berkas acuan di
  `crates/koha-gui/tests/snapshots/`). Bila `fontdue` atau sebuah font berganti versi, atau kamu
  sengaja mengubah rendering teks, perbarui acuannya lalu periksa selisihnya di diff:

  ```powershell
  $env:UPDATE_SNAPSHOTS = 1; cargo test -p koha-gui snapshot; Remove-Item Env:UPDATE_SNAPSHOTS
  ```

- **Test harus tahan terhadap akhir baris LF maupun CRLF.** CI Windows mengambil kode dengan
  CRLF (`core.autocrlf`), sedangkan di banyak mesin berkasnya LF. Berkas teks yang
  ditanam dengan `include_str!` (misalnya `default_config.toml`) ikut berganti akhir baris.
  Test yang mengganti `\n` dengan `\r\n` pada teks seperti itu menghasilkan `\r\r\n`, dan
  pernah gagal hanya di CI. Normalkan dulu ke LF sebelum mengubah, atau bandingkan setelah
  menormalkan. Untuk mereproduksi di mesinmu:
  `git -c core.autocrlf=true clone --no-hardlinks . $env:TEMP\koha-crlf`, lalu
  `cargo test --workspace --no-fail-fast` di sana (`--no-fail-fast` penting: tanpanya
  Cargo berhenti di target test pertama yang gagal dan menyembunyikan yang lain).
- **Dokumentasi diuji.** Setiap blok `toml` di `README.md` dan `docs/configuration.md` harus
  berupa config yang valid (blok `toml state` harus berupa state yang valid), semua tautan
  relatif di berkas `.md` harus menunjuk berkas yang ada, dan dokumen konfigurasi harus
  menyebut semua aksi bawaan, tema, dan font. Bila tes ini gagal setelah kamu mengubah kode,
  dokumennya perlu diperbarui.
- Perilaku jendela yang sebenarnya (fokus, posisi, tampilan) tidak bisa dites unit. Jalankan
  `cargo run` dan lihat sendiri, dan tulis di deskripsi perubahan apa yang sudah kamu coba.

## Menambah sesuatu

| Yang ditambah | Mulai dari |
|---|---|
| Field atau tipe item menu baru | `koha-core/src/config.rs` (`RawItem`, `convert_item`, `reject_foreign_fields`, tipe `Action`), lalu `koha-platform/src/lib.rs` (`execute`) |
| Aksi bawaan (`builtin`) baru | `Builtin` dan `Builtin::NAMES` di `koha-core/src/config.rs`; operasi di trait `Platform` dan di semua implementasinya (termasuk `unsupported.rs` dan tiruan di tes); `execute` |
| Tema baru | Tabel `BUILTIN` di `koha-core/src/theme.rs`, daftar di tes tema, komentar di `default_config.toml`, dan [docs/configuration.md](docs/configuration.md) |
| Font baru | Berkas `.ttf` dan `OFL.txt` (atau lisensi yang cocok) di `koha-gui/assets/fonts/<nama>/`, `FONTS` di `koha-gui/src/fonts.rs`, konstanta id di `theme.rs`, lalu `gen-third-party.ps1` |
| Dukungan OS | [docs/platforms.md](docs/platforms.md) menjelaskan titik kerjanya |
| Operasi `Platform` baru | Trait di `koha-platform/src/lib.rs`, implementasi Windows di `win.rs`, `unsupported.rs`, tiruan di tes |

Satu aturan untuk menambah dependensi: **waktu start adalah fitur utama**. Periksa ukuran
binary sebelum dan sesudah, dan bila perubahannya menyentuh jalur start, ukur dengan
[tools/bench-startup.ps1](tools/README.md) (hasil dan metode saat ini ada di
[docs/performance.md](docs/performance.md)). Dependensi baru juga harus lolos `cargo deny check`.

## Gaya

- Komentar dan dokumentasi ditulis dalam bahasa Indonesia; pesan untuk pengguna juga
  berbahasa Indonesia. Berkas yang dilihat pengguna akhir lintas bahasa (config contoh,
  daftar lisensi) berbahasa Inggris.
- Komentar menjelaskan *mengapa*. Keputusan yang bisa terlihat salah (angka yang dipilih dari
  pengukuran, urutan yang tidak jelas, batasan dari OS) ditulis alasannya di tempatnya.
- Setiap blok `unsafe` sekecil mungkin dan diberi komentar `// SAFETY:` yang menjelaskan
  mengapa aman.
- Jangan mengklaim sesuatu terverifikasi bila belum diuji; sebutkan terang-terangan apa yang
  belum (itu juga berlaku untuk deskripsi perubahan).

## Merilis

Langkah membuat rilis ada di [docs/releasing.md](docs/releasing.md).

## Pesan commit

Satu baris ringkas berawalan area, seperti riwayat yang sudah ada:

```text
core: add State, theme picker and Menu Settings overlays
gui: render menu text with per-theme fonts; window grows to fit labels
platform: Windows launch/url/lock/sleep/close_all_windows via windows crate
app: load config/state from disk, add CLI
docs: add platform support notes
tools: startup benchmark script and startup-trace feature
build: tuned release profile from measurements
chore: third-party license notices, cargo-deny policy
```

## Melaporkan masalah

Sertakan: versi (`koha --version`), versi Windows, cara KOHA dipanggil (terminal, pintasan,
tombol Lenovo Vantage), langkah untuk mengulang, apa yang terjadi, dan apa yang kamu harapkan.
Untuk masalah config, sertakan config-nya (hapus bagian pribadi) dan pesan errornya; untuk
masalah waktu start, hasil `tools\bench-startup.ps1` beserta cara kamu menjalankannya.
