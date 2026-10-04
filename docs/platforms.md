# Dukungan platform dan cara melanjutkan untuk OS lain

KOHA dirancang lintas OS, tetapi **baru Windows yang dikerjakan dan diuji**.
Dokumen ini menjelaskan keadaan sebenarnya, dan di mana tepatnya pekerjaan untuk
Linux dan macOS harus dilanjutkan.

## Keadaan saat ini

| Bagian | Windows | Linux / macOS |
|---|---|---|
| Logika (`koha-core`: config, menu, tema, state) | Berjalan, diuji | Tidak punya kode OS; terkompilasi dan tes unitnya lolos di CI |
| Tata letak, teks, dan render (`koha-gui`: `layout`, `render`, `text`, `canvas`) | Berjalan, diuji | Tidak punya kode OS; terkompilasi dan tes unitnya lolos di CI |
| Jendela popup (`koha-gui/src/window.rs`) | Berjalan, diuji | Terkompilasi di CI, **belum pernah dijalankan** (lihat "Jendela" di bawah) |
| Lokasi config dan state (`koha-app/src/paths.rs`) | Berjalan, diuji | Memakai crate `directories`; terkompilasi dan tesnya lolos di CI, tetapi lokasi sebenarnya (tabel bawah) belum diverifikasi di mesin sungguhan |
| Aksi (`koha-platform`) | Berjalan | Semua aksi mengembalikan `Unsupported` |

Artinya, di Linux atau macOS menu bisa ditampilkan (kalau jendelanya berjalan),
tetapi memilih aksi apa pun menghasilkan pesan `operasi "..." belum didukung di OS ini`.

### Yang sudah diuji di Windows 11

`exec` biasa dan dengan variabel lingkungan, `url`, `lock`, dan `close_all_windows`.
`sleep` tidak bekerja di laptop pengembang (Modern Standby tanpa hibernasi, lihat
komentar di `koha-platform/src/win.rs` dan di `default_config.toml`), jadi tidak
ada di config contoh. `exec` dengan `admin = true` (UAC) diuji manual oleh pemilik.

### Yang sudah dibuktikan CI, dan yang belum

CI GitHub Actions (`.github/workflows/ci.yml`) sudah dijalankan. Pada run untuk commit
`120e651`, job `check (ubuntu-latest)` dan `check (macos-latest)` **lolos semua langkahnya**:
`cargo fmt`, `cargo clippy -D warnings`, `cargo test --workspace` (termasuk tes
`UnsupportedPlatform` yang hanya dikompilasi di OS non-Windows), dan varian fitur
`startup-trace`. Jadi di Linux dan macOS, **kode terkompilasi dan tes unitnya lolos**.

Yang **belum** dibuktikan: bahwa jendelanya benar-benar tampil dan bekerja. Tidak ada tes
yang membuka jendela, dan tidak ada yang pernah menjalankan `koha` di Linux atau macOS.

Pada run yang sama job Windows gagal di `cargo test` karena satu tes yang tidak tahan
terhadap akhir baris CRLF di checkout CI (sudah diperbaiki; lihat
[releasing.md](releasing.md#pelajaran-dari-ci)).

## Di mana melanjutkan

Semua pekerjaan khusus-OS sengaja dipusatkan di dua crate. `koha-core` tidak
boleh menyentuh OS.

### 1. Aksi: `crates/koha-platform` (pekerjaan utama)

- `src/lib.rs`: trait `Platform` (lima operasi) dan pemilihan implementasi lewat
  `cfg`. Di sini ada alias `SystemPlatform`:
  - `#[cfg(windows)]` memakai `WindowsPlatform` (`src/win.rs`).
  - `#[cfg(not(windows))]` memakai `UnsupportedPlatform` (`src/unsupported.rs`).

  Untuk menambah OS, ganti cabang `not(windows)` menjadi `unix` (dan tetap sediakan
  cabang cadangan untuk target lain) lalu tambahkan modul implementasinya.
- `src/unsupported.rs`: perilaku sekarang untuk OS lain. Ini bisa dipakai sebagai
  kerangka implementasi baru.
- `src/win.rs`: acuan cara satu implementasi dibuat (urutan: validasi, panggilan
  OS, pemetaan error ke `PlatformError`, pembatalan oleh pengguna menjadi
  `Cancelled`).
- `src/cmdline.rs`: `validate_url` murni dan dites di semua OS; pakai lagi di
  implementasi Unix.
- `src/window_filter.rs`: khusus Windows (kriteria jendela untuk `close_all_windows`).

Rancangan yang sudah dipikirkan (belum dikerjakan):

1. **Pembentuk perintah yang murni**, mis. `unix_commands.rs`, dikompilasi di semua
   OS seperti `cmdline.rs`, sehingga bisa dites dari Windows:
   `open_url_command(os, url) -> CommandSpec { program, args }`
   (`xdg-open` di Linux, `open` di macOS) dan `launch_command(command, args)`.
2. **`unix.rs`** dengan `#[cfg(unix)]`, memakai `std::process::Command` tanpa shell
   (tidak ada risiko injeksi lewat spasi atau karakter khusus). Jalankan proses
   anak di process group baru (`CommandExt::process_group(0)`) dengan
   stdin/stdout/stderr ke `/dev/null`, agar tetap hidup setelah KOHA keluar dan tidak
   mengotori terminal.
3. **Tanpa crate `open`**: logikanya sekitar dua puluh baris, dan menghindari
   dependensi pada aplikasi yang waktu startnya adalah fitur utama.
4. **`admin = true` di Unix**: kembalikan `Unsupported` yang jelas, jangan
   menjalankan diam-diam tanpa hak yang diminta. `pkexec` (Linux) atau
   `osascript ... with administrator privileges` (macOS) belum diputuskan.
5. Program tidak ditemukan harus menghasilkan pesan yang bisa ditindaklanjuti
   (mis. "pasang paket xdg-utils" bila `xdg-open` tidak ada).
6. `lock`, `sleep`, dan `close_all_windows` boleh tetap `Unsupported` di versi
   pertama. Petunjuk awal (belum diverifikasi): Linux `loginctl lock-session`;
   macOS belum dipilih.

Test yang disarankan:

- Di semua OS: `open_url_command` per OS, argumen berspasi dipertahankan apa adanya,
  URL tidak valid ditolak sebelum sampai ke platform.
- Khusus Unix (jalan di CI Ubuntu dan macOS): `launch("true", ...)` berhasil, program
  yang tidak ada mengembalikan error, `admin = true` mengembalikan `Unsupported`.
  **Jangan** memanggil `xdg-open` atau `open` sungguhan dari test, supaya CI tidak
  membuka peramban.

### 2. Jendela: `crates/koha-gui/src/window.rs`

Tipis dan hampir seluruhnya lintas OS (`winit` + `softbuffer`). Satu-satunya cabang
khusus-OS sekarang adalah `#[cfg(windows)]` di `create_gfx`
(`with_skip_taskbar`). Hal yang perlu diperiksa di OS lain:

- **Wayland** tidak mengizinkan aplikasi menentukan posisi jendela
  (`set_outer_position`) atau menetapkan always-on-top. Menengahkan jendela dan
  `WindowLevel::AlwaysOnTop` kemungkinan tidak berlaku di sana. X11 tidak punya
  batasan ini.
- **Fokus**: perilaku merebut fokus saat dipanggil dari shortcut berbeda per OS dan
  per window manager. Dismiss-on-blur bergantung pada event `Focused`.
- **Event tombol sintetis**: `map_key` (`koha-gui/src/input.rs`) mengabaikan event
  `is_synthetic`. Ini diperlukan di Windows (tombol pemicu masih tertahan saat jendela
  muncul); perilaku `winit` di OS lain perlu dicek.
- **Monitor**: jendela ditengahkan di monitor *utama*. Memilih monitor di bawah
  kursor belum dikerjakan di OS mana pun.
- `softbuffer` membutuhkan fitur yang sesuai per backend; `Cargo.toml` memakai
  fitur bawaan.

### 3. Lokasi berkas: `crates/koha-app/src/paths.rs`

`Dirs::system()` memakai crate `directories` (`BaseDirs`) dan seharusnya sudah
bekerja di semua OS. Lokasi yang dihasilkan, sesuai dokumentasi crate (hanya Windows
yang diverifikasi di mesin sungguhan):

| | Config | State |
|---|---|---|
| Windows | `%APPDATA%\koha\config.toml` | `%LOCALAPPDATA%\koha\state.toml` |
| Linux | `~/.config/koha/config.toml` | `~/.local/state/koha/state.toml` |
| macOS | `~/Library/Application Support/koha/config.toml` | `~/Library/Application Support/koha/state.toml` |

### 4. Fitur Windows-saja yang butuh keputusan untuk OS lain

- **Perluasan variabel lingkungan** di `command` (`%LOCALAPPDATA%\...`) dikerjakan
  oleh `ShellExecuteEx` (`SEE_MASK_DOENVSUBST` di `win.rs`). Di Unix tidak ada padanan
  otomatis; perlu diputuskan apakah mau didukung (mis. `$HOME` atau `~`) dan dengan
  sintaks apa.
- **`sleep`**: lihat catatan Modern Standby di atas. Padanan Unix belum dirancang.

## Bagian yang tidak perlu disentuh

`koha-core` (config, menu, tema, state), pemformatan teks dan font (`fontdue`, font
dibundel lewat `include_bytes!`), dan `layout`/`render`/`canvas` tidak punya kode
khusus-OS.

## Cara memeriksa dari Windows tanpa perangkat lain

CI Ubuntu dan macOS sudah memeriksa kompilasi tiap kali kamu push. Perintah di bawah hanya
berguna untuk memeriksa lebih cepat sebelum push (belum pernah dijalankan di mesin
pengembangan). Kompilasi silang hanya memeriksa bahwa kode **terkompilasi**, bukan bahwa ia
**bekerja**:

```bash
rustup target add x86_64-unknown-linux-gnu aarch64-apple-darwin
cargo check --workspace --target x86_64-unknown-linux-gnu
cargo check --workspace --target aarch64-apple-darwin
```

Bagian yang murni (pembentuk perintah, validasi URL) bisa diuji sepenuhnya di
Windows. Menjalankan kode Unix yang sesungguhnya membutuhkan Linux atau macOS
(WSL, mesin virtual, atau runner CI).
