# Perjalanan membangun KOHA

Dokumen ini mencatat **bagaimana KOHA dibangun, tahap demi tahap**, dari workspace kosong
sampai versi `0.1.0`: apa yang dibuat dalam urutan apa, mengapa, konsep Rust apa yang muncul,
keputusan apa yang diambil, dan kesalahan apa yang terjadi di tengah jalan beserta
perbaikannya. Untuk *bagaimana hasil akhirnya bekerja* lihat [architecture.md](architecture.md).

KOHA ditulis ulang dari versi AutoHotkey, sekaligus sebagai **proyek belajar Rust**. Karena itu
cara kerjanya sengaja bertahap, dan dokumen ini mengikuti urutan itu.

## Cara kerja yang dipakai

- **Satu langkah kecil per giliran.** Tidak ada langkah yang menulis seluruh proyek sekaligus.
- **Rancangan dulu, kode sesudah disetujui.** Setiap langkah dimulai dengan rencana dan
  pertanyaan keputusan; kode baru ditulis setelah ada persetujuan.
- **Setiap langkah ditutup dengan** `cargo fmt`, `cargo clippy -- -D warnings`, dan
  `cargo test`, dilaporkan apa adanya (termasuk yang gagal).
- **Konsep Rust dijelaskan di tempat ia muncul**, di komentar kode dan di laporan langkah.
- **Biaya performa disebut** (ukuran binary, waktu start), lalu diukur sebelum dan sesudah
  menambah dependensi besar.
- **Git (commit, push) dilakukan pemilik repositori**; tiap langkah menyebut titik commit yang
  masuk akal. Itu sebabnya satu commit kadang mencakup beberapa langkah.
- **Jujur soal yang belum diuji.** Yang tidak bisa dibuktikan (tombol Lenovo Vantage, OS selain
  Windows) dicatat sebagai belum terverifikasi, bukan diasumsikan jalan.

Dikerjakan pada **3 sampai 4 Oktober 2026**, menghasilkan 19 commit.

## Ringkasan dalam angka

| | |
|---|---|
| Crate | 4 (`koha-core`, `koha-platform`, `koha-gui`, `koha-app`) |
| Kode Rust | sekitar 6.700 baris, termasuk tes |
| Tes | 262 (dari 3 di langkah pertama) |
| Ukuran `koha.exe` rilis | 132,5 KB (kosong) → 2.059 KB (profil bawaan) → **1.470 KB** (profil yang disetel) |
| Waktu sampai jendela terlihat | sekitar 34 ms (versi AHK: sekitar 68 ms; biaya minimum memulai proses di mesin itu: sekitar 16 ms) |

Angka waktu start berasal dari satu laptop dan dua sesi pengukuran; batasannya ada di
[performance.md](performance.md).

## Peta langkah, commit, dan jumlah tes

Jumlah tes di bawah dihitung dengan menjalankan `cargo test --workspace --no-fail-fast` pada
**setiap commit** di klon terpisah (kolom "Tes" = lolos / gagal). Pesan commit di riwayat kadang menggabungkan beberapa langkah.

| Commit | Langkah | Tes | Catatan |
|---|---|---|---|
| `37abd37` | 1. Workspace kosong, empat crate, CI dasar | 3 | |
| `1c905c4` | 2a. `Rgb`, `Theme`, `ThemeSet`; 2b. `Config`, parser TOML, validasi | 42 | |
| `9015920` | 2c. `MenuState` (navigasi) | 56 | |
| `4ac99fc` | 2d. `State`, pemilih tema, Menu Settings | 80 | |
| `880fe58` | 2e. Config contoh bawaan | 85 | Akhir `koha-core` tahap pertama |
| `80ddd14` | 3a. Tata letak, render piksel, pemetaan tombol | 114 | Belum ada jendela |
| `00a76e7` | 3b. Jendela `winit` + `softbuffer` | 114 | Blok warna tanpa teks |
| `4023244` | 4a. `Canvas`, font OFL, `TextRenderer` | 149 | |
| `ddbff80` | 4b. Teks di menu, jendela melebar mengikuti teks | 164 | |
| `a657f4f` | 5. CLI, config dan state di disk | 209 / 1 | Satu tes CRLF gagal mulai di sini (lihat tahap 9d) |
| `55a6d3b` | 6. Aksi Windows lewat `koha-platform` | 246 / 1 | Juga mencabut `sleep` dari config contoh |
| `d2d90b0` | 8a. Build tanpa konsol; 8b. Jejak dan alat ukur waktu start | 252 / 1 | |
| `c7e8d9b` | 8c. Profil rilis yang disetel, `performance.md` | 252 / 1 | |
| `86d0667` | 9a. Lisensi pihak ketiga, `cargo-deny`, CI | 252 / 1 | |
| `120e651` | 9b. README, referensi konfigurasi, CONTRIBUTING, CHANGELOG | 261 / 1 | Tes dokumentasi masuk |
| `02456b5` | 9c. Pengemasan rilis | 261 / 1 | |
| `5111e86` | Perbaikan tes CRLF | 262 | CI Windows hijau kembali |
| `1594bef` | 9d. Daftar periksa rilis | 262 | |
| `3089091` | Skrip pemasangan pintasan | 262 | |

Langkah 7 (Linux dan macOS) **ditunda** atas keputusan pemilik; titik lanjutnya ada di
[platforms.md](platforms.md).

---

## Tahap 0: menentukan apa yang dibangun

Sebelum ada kode, prompt awal dikoreksi karena salah memahami proyek:

- Yang dibangun adalah **GUI, bukan TUI**, dan temanya bukan "terminal": "terminal" hanya satu
  dari tujuh tema. Rujukannya lebih ke rofi di Linux.
- **Referensi perilaku utama adalah versi AutoHotkey** (`win-koha-ahk`), bukan prototipe Go.
  README dan `koha.ahk` dibaca penuh untuk memahami fiturnya, termasuk jebakan yang sudah
  ketahuan di sana (centering, dismiss-on-blur yang menutup sebelum aksi jalan, konsol yang
  berkedip).

Keputusan awal:

| Keputusan | Pilihan | Alasan |
|---|---|---|
| Cakupan | Inti dulu: menu, submenu, tema, aksi dasar | Aplikasi "buka aplikasi tertentu" akan digantikan Search Apps; fitur personal (pengeluaran, waktu sholat, ...) menjadi plugin di masa depan |
| OS | Windows dulu, arsitektur siap lintas OS | OS lain menyusul tanpa mengubah inti |
| Format config | TOML | Lazim di Rust, ada komentar, galat bisa menunjuk baris dan kolom |
| GUI | `winit` + `softbuffer` | Start cepat dan binary kecil; render CPU cukup untuk beberapa baris teks |
| Font | Berganti mengikuti tema | Setiap tema punya identitasnya |
| Nama | Repo `gui-koha-rust`, binary `koha` | |
| Lisensi | Belum ada | Diputuskan belakangan |

**Lingkungan.** Rust belum terpasang, sehingga pemilik memasangnya sendiri. Masalah pertama: `cargo build` gagal di tahap *link*
dengan `cannot open file 'msvcrt.lib'`. Rust sudah memilih Visual Studio 2026, tetapi komponen
"MSVC build tools x64/x86" belum terpasang penuh. Solusinya memasang workload "Desktop
development with C++". Ini kemudian ditulis sebagai prasyarat di README.

## Tahap 1: workspace kosong

**Tujuan:** kerangka yang bisa dikompilasi, dengan aturan dependensi yang sudah benar sejak
awal.

- `Cargo.toml` akar dengan `[workspace]`, versi bersama (`[workspace.package]`), dan dependensi
  internal bersama (`[workspace.dependencies]`).
- Empat crate: `koha-core`, `koha-platform`, `koha-gui` (library) dan `koha-app` (binary
  `koha`, lewat `[[bin]] name = "koha"`).
- `.gitignore`, `docs/README.md`, dan `.github/workflows/ci.yml` dasar.

**Konsep Rust:** workspace dan `Cargo.lock` bersama; crate vs package; `lib.rs` vs `main.rs`;
path dependency; mengapa edition 2024 dan `resolver = "3"` (Rust 1.99 mendukungnya).

**Hasil:** 3 tes (satu per crate library), `cargo run` mencetak satu baris.

## Tahap 2: `koha-core`, logika murni

Lima sub-langkah, masing-masing dengan rancangan dan persetujuan sendiri. Semuanya tanpa
panggilan OS.

### 2a. Warna dan tema

- `Rgb` (`#RRGGBB` ↔ tiga byte), `Theme`, dan `ThemeSet` dengan tujuh tema yang dipindahkan 1:1
  dari `THEMES` di `koha.ahk`.
- **Konsep Rust:** `Copy` vs `Clone`; `const fn`; `FromStr` sehingga `"#9BBC0F".parse::<Rgb>()`
  bekerja; enum galat dengan `thiserror`; lifetime `'static` untuk tabel tema.
- **Masalah:** `u8::from_str_radix` ternyata menerima tanda `+`, jadi `"+1+2+3"` lolos sebagai
  warna. Penjaganya ditulis sebagai pengecekan digit manual, plus tes. Satu tes saya sendiri
  salah menghitung byte (`"éé€"` itu 7 byte, bukan 6), dikoreksi dengan dua kasus terpisah.

### 2b. Config

- `serde` + `toml`. Alurnya dua tahap: teks TOML → tipe `Raw*` (bentuk persis berkas) →
  validasi → tipe domain (`Config`, `MenuItem`, `Action`, ...).
- Galat sintaks membawa **baris dan kolom** (dari rentang byte parser); galat aturan membawa
  **jalur item** (`menu[1].items[0]`).
- Versi dibaca lebih dulu dengan `VersionProbe`, supaya berkas versi masa depan menghasilkan
  "versi tidak didukung", bukan "field tidak dikenal".
- **`id` wajib dan unik** (keputusan pemilik: status ON/OFF disimpan dengan `id`, bukan `label`).
- **Konsep Rust:** pola Raw → domain; enum rekursif lewat `Vec`; `collect()` ke
  `Result<Vec<_>, _>`; atribut serde (`deny_unknown_fields`, `rename`, `default`); raw string
  `r##"..."##` (satu tes gagal kompilasi karena `"#FFFFFF"` mengakhiri `r#"..."#`).
- **Hasil:** 42 tes; asersi posisi galat (baris 7 kolom 1, baris 4 kolom 9) benar pada
  percobaan pertama.

### 2c. State machine navigasi

- `MenuState::update(Input) -> Effect`, murni. `Input`: `Up, Down, Enter, Back, Dismiss`.
  `Effect`: `None, Redraw, Run(Action), Close`.
- **Keputusan desain:** `MenuState` **memiliki** salinan pohon menu (bukan meminjam config),
  untuk menghindari struktur yang saling meminjam yang sulit di Rust; biayanya satu salinan
  kecil.
- **Setelah `Run` atau `Close`, semua input diabaikan.** Ini membawa pelajaran terbesar dari
  versi AHK (dismiss-on-blur yang membatalkan aksi) langsung ke dalam inti.
- **Konsep Rust:** memiliki vs meminjam; lifetime elision; `let ... else`; `match` yang harus
  menyeluruh; aritmetika `usize` tanpa underflow (`(i + len - 1) % len`).

### 2d. State, pemilih tema, Menu Settings

- `State { version, theme, hidden }` terpisah dari `Config`.
- Tampilan khusus (**overlay**) di atas jalur submenu: pemilih tema dan Menu Settings. Satu
  fungsi `entries()` menghasilkan pasangan `{label, target}`, dan `rows()` serta `Enter`
  sama-sama berangkat darinya, sehingga tidak ada pencocokan teks label yang dinamis.
- Baris "Menu Settings" otomatis di akhir menu utama; filter ON/OFF memakai indeks asli supaya
  menyembunyikan satu item tidak menggeser aksi item lain.
- `Effect::StateChanged` ditambahkan (gambar ulang + terapkan tema + simpan).
- **Konsep Rust:** enum dengan data; `BTreeSet`; `Option::take()`; memindahkan entri keluar
  dari `Vec` (`into_iter().nth()`) agar pinjaman ke `self` lepas sebelum memutasinya; struct
  update syntax (`..Default::default()`, setelah `clippy` memprotes pola lama).
- **Hasil:** 80 tes.

### 2e. Config contoh

- `default_config.toml` ditanam dengan `include_str!`, dan **diuji selalu valid**. Satu tes
  bahkan memeriksa bahwa blok contoh yang di-komentar tetap valid bila tanda `#` dihapus,
  supaya contoh di komentar tidak membusuk.

## Tahap 3: GUI tanpa teks

### 3a. Logika yang bisa dites tanpa jendela

- `layout.rs`: ukuran dari jumlah baris dan skala DPI. **Tiap tepi dibulatkan sendiri** agar
  baris yang bersebelahan tidak punya celah satu piksel pada skala 125/150%.
- `render.rs`: menulis ke `&mut [u32]` (format `0x00RRGGBB`, sama dengan `softbuffer`), dengan
  pemotongan ke batas buffer; buffer kosong atau lebih kecil dari yang diklaim tidak membuat panic.
- `input.rs`: `map_key` (tombol → `Input`) dan `FocusGate` (dismiss-on-blur hanya setelah
  jendela pernah mendapat fokus; menggantikan timer 200 ms di AHK dengan aturan deterministik).
- Versi pustaka: `winit` **0.30.13** (stabil), bukan 0.31 beta, dan `softbuffer` 0.4.8 yang diuji
  terhadapnya.

### 3b. Jendela pertama

- `window.rs`: `winit` untuk jendela dan event, `softbuffer` untuk menampilkan buffer.
  Jendela dibuat **tak terlihat**, ditengahkan, baru ditampilkan (pelajaran centering dari AHK).
  `run` baru kembali setelah jendela dihancurkan, sehingga aksi dijalankan sesudahnya.
- **Diverifikasi sungguhan:** program dijalankan, jendela ditangkap, dan hasilnya diperiksa:
  375×210 piksel pada skala 125%; pindah ke submenu mengecil ke 375×48 dan tetap di tengah
  vertikal; `Esc` kembali; `Esc` kedua menutup proses.
- **Masalah:** `SoftBufferError` berisi pointer mentah dan bukan `Send + Sync`, sehingga tidak
  bisa dibungkus `anyhow::Error`; `GuiError::Draw` hanya menyimpan pesan teksnya. Dan menjalankan
  program lewat `Start-Process` biasa membuatnya menutup sendiri dalam sekejap: konsol baru
  merebut fokus, lalu dismiss-on-blur bekerja sebagaimana mestinya.
- Ukuran `koha.exe` rilis naik dari 132,5 KB menjadi 1.000 KB (efek `winit`), dicatat sebelum
  dan sesudah.

## Tahap 4: teks dan font

### 4a. Mesin teks

- Tiga font berlisensi **OFL** diunduh (dengan izin): Press Start 2P (`game_boy`), VT323
  (`amber`, `green_term`), IBM Plex Mono (empat tema lain). Teks lisensinya ikut disimpan.
  JetBrains Mono dibatalkan karena hanya tersedia sebagai *variable font*, yang tidak diterapkan
  sumbunya oleh `fontdue`.
- **Diukur dulu, baru memutuskan ukuran.** Press Start 2P tajam sempurna hanya pada kelipatan
  8 px; VT323 **tidak tajam di ukuran mana pun** (bukan bitmap sejati), jadi dirender halus dan
  ukurannya dipilih agar lebar karakter bilangan bulat.
- `Canvas` (penulisan terpotong ke batas), `FontSpec`, `TextRenderer` (cache glyph, `measure`,
  `baseline`, `draw`).
- **Konsep Rust:** `include_bytes!` dan `&'static [u8]`; `HashMap::entry().or_insert_with()`
  dengan closure yang meminjam field lain; `std::ptr::eq`; alpha blending dengan aritmetika
  bilangan bulat.
- **Masalah:** satu tes gagal karena saya menggambar mulai dari x=8, padahal baris dimulai di
  x=12 pada skala 2,0; pesan gagalnya diubah agar menampilkan kotak batas piksel, bukan dump
  buffer.

### 4b. Teks di menu

- Baris `> Nama` (terpilih) dan dua spasi (lain), seperti AHK. Jendela **melebar** mengikuti
  teks terpanjang (minimum 300, maksimum 90% lebar monitor), karena Press Start 2P pada 2×
  membuat `> Close All Windows` selebar 304 px.
- Tes snapshot seni-ASCII per font, lalu uji visual: tema diganti dari dalam menu dan jendela
  ditangkap pada `amber` (VT323), `game_boy` (Press Start 2P), dan `gruvbox` (IBM Plex Mono);
  font ikut berganti.
- **Kecelakaan:** menyunting berkas dengan Python menulis ulang akhir baris dan merusak tiga
  string `\n` di `text.rs` menjadi baris baru sungguhan. Kompiler dan `clippy` menangkapnya; dua
  string tetap berfungsi karena menjadi literal multi-baris, lalu dikembalikan ke bentuk
  `\n`. Pelajaran: jangan menulis escape di dalam string lewat skrip penyuntingan.
- **Hasil:** 164 tes; `koha.exe` rilis 1.507 KB.

## Tahap 5: aplikasi, CLI, config dan state di disk

- `cli.rs` (`clap`): `--config`, `--state`, `--init`, `--print-config-path`, `--print-state-path`,
  dengan satu grup agar mode saling eksklusif.
- `paths.rs`: prioritas flag › variabel lingkungan › standar. Environment dan folder dasar
  **disuntikkan**, jadi aturannya dites tanpa menyentuh sistem. `BaseDirs` dipakai, bukan
  `ProjectDirs`, agar jalurnya bersih (`%APPDATA%\koha\config.toml`).
- `store.rs`: config yang rusak **tidak pernah** jatuh diam-diam ke bawaan; config yang tidak
  ada memakai menu bawaan, kecuali lokasinya dipilih pengguna; state **atomik** (tulis ke berkas
  sementara, lalu `rename`); `--init` memakai `create_new(true)` sehingga tidak pernah menimpa.
- `diagnostics.rs`: pesan galat dengan potongan baris dan tanda `^`.
- **Keputusan:** bila `--config` diberikan tanpa `--state`, state diletakkan di sebelah config,
  supaya mencoba config lain tidak menimpa state harian.
- **Bug nyata yang ditemukan lewat uji ujung-ke-ujung:** menu menutup sendiri dalam beberapa detik
  di setiap peluncuran. Log sementara menunjukkan tombol `AudioVolumeUp` tiba sebelum fokus.
  Penyebabnya perilaku `winit`: saat jendela mendapat fokus ia menerbitkan penekanan tombol
  **sintetis** untuk tombol yang dianggap sedang ditahan. Ini bukan masalah mesin saja: saat
  KOHA dipanggil dari sebuah tombol, tombol itu biasanya masih tertahan, sehingga menu akan
  menutup sendiri di pemakaian sebenarnya. `map_key` kini mengabaikan event sintetis.
- **Kesalahan saya:** skrip uji menjalankan perubahan tema di lokasi state standar, sehingga
  `state.toml` milik pengguna tertulis. File itu dihapus dan skrip uji diperbaiki agar memakai
  `--state` ke folder sementara.
- Ukuran: 2.033 KB (`clap`, `directories`, kode berkas).

## Tahap 6: aksi sungguhan di Windows

- Trait `Platform` (lima operasi) dan `execute(&dyn Platform, &Action)`, dites dengan platform
  tiruan; `PlatformError` dengan `Cancelled` (UAC ditolak) dibedakan dari kegagalan.
- `cmdline.rs`: pengutipan argumen sesuai aturan baris perintah Windows, dengan tes yang
  mengurai hasilnya kembali dan membuktikan argumennya sama; `validate_url` menolak teks tanpa
  skema (karena `ShellExecute` akan menjalankan jalur berkas).
- `win.rs` (crate `windows`): `ShellExecuteExW` (verb `runas` untuk admin, perluasan `%VAR%`),
  `LockWorkStation`, `SetSuspendState`, dan `close_all_windows` (`EnumWindows` + `WM_CLOSE`).
  Kode `unsafe` dibuat sekecil mungkin dengan komentar `// SAFETY:`.
- `window_filter.rs`: aturan jendela mana yang boleh ditutup, berbasis kriteria Alt+Tab, murni
  dan dites. **Mode kering** (`examples/list_windows.rs`) mencetak jendela yang *akan* ditutup
  tanpa menutup apa pun.
- **Pengujian yang aman:** `exec` diuji dengan meluncurkan `notepad.exe` (lalu hanya proses baru
  yang ditutup); `lock`, `sleep`, UAC, dan pembukaan URL **tidak** dijalankan sendiri oleh
  pengembang karena akan mengganggu mesin; pemilik mengujinya.
- **Hasil uji pemilik:** semuanya bekerja kecuali `sleep`. Penyebabnya: laptop itu hanya punya
  Modern Standby tanpa hibernasi, dan `SetSuspendState` dirancang untuk tidur klasik. `sleep`
  dicabut dari config contoh (kodenya tetap, dengan catatan).
- Ukuran: 2.059 KB.

## Tahap 7: OS lain, ditunda

Atas keputusan pemilik, Linux dan macOS tidak dikerjakan sekarang. Yang dilakukan hanya
**mendokumentasikan di mana melanjutkannya** ([platforms.md](platforms.md)): titik pemilihan
`cfg`, rancangan yang sudah dipikirkan (pembentuk perintah murni, `xdg-open`/`open`), dan
batasan seperti Wayland. Belakangan CI menunjukkan bahwa kode sudah terkompilasi dan tes
unitnya lolos di Ubuntu dan macOS, walau jendelanya belum pernah dijalankan di sana.

## Tahap 8: waktu start

### 8a. Perilaku binary rilis

- `windows_subsystem = "windows"` di build rilis: tidak ada kilatan konsol saat dipanggil dari
  tombol.
- Konsekuensinya: tanpa konsol, `--help` dan pesan galat tidak terlihat. Jawabannya
  `attach_parent_console()` (menempel ke konsol terminal induk) dan `show_error_dialog()`
  (`MessageBox` bila tidak ada konsol). Keputusan "stderr atau dialog" dibuat fungsi murni yang
  dites.
- Uji tanpa konsol sungguhan membutuhkan induk yang benar-benar tanpa konsol (`wscript.exe`);
  PowerShell alat uji ternyata punya konsol tersembunyi, sehingga `AttachConsole` berhasil dan
  dialog tidak muncul. Dialog lalu terbukti muncul untuk config rusak dan config yang tidak ada.

### 8b. Alat ukur

- Fitur Cargo `startup-trace` (mati secara bawaan; **terbukti ukuran binary tidak berubah**
  saat mati) mencetak waktu tiap tahap, dan `tools/bench-startup.ps1` mengukur sampai jendela
  terlihat, bergantian antar target, dengan pemanasan dan statistik.
- Bug di skrip sendiri: `$PSScriptRoot` kosong di nilai bawaan parameter pada Windows
  PowerShell 5.1.

### 8c. Mengukur dan memutuskan

- Tiga profil rilis dibangun di folder terpisah dan diukur dalam **dua sesi** penuh, dibandingkan
  dengan `KOHA.exe` AHK. Hasil dan keterbatasannya ada di [performance.md](performance.md).
- **Keputusan:** `opt-level 3` + `lto` + `codegen-units = 1` + `panic = "abort"` + `strip`
  (29% lebih kecil, tanpa menambah waktu start); `opt-level "z"` konsisten ~2 ms lebih lambat dan
  tidak dipilih. `clap` dipertahankan: biayanya sekitar 0,15 ms dan sekitar 241 KB, di bawah
  ambang yang disepakati.
- **Temuan jujur:** angka "terlihat" tidak sama dengan "tergambar" (jendela Rust terlihat sekitar
  15 ms sebelum gambar pertama selesai), jadi klaim "dua kali lebih cepat dari AHK" **tidak**
  boleh dipakai. Peluncuran pertama berkas yang baru di-link bisa memakan 0,6 sampai 5 detik
  (diduga pemindaian antivirus, belum dibuktikan).
- Rincian waktu menunjukkan ke mana ~34 ms habis; empat kandidat optimasi dicatat tetapi belum
  dikerjakan.
- Ukuran akhir: **1.470 KB**.

## Tahap 9: kualitas dan rilis

### 9a. Lisensi, keamanan dependensi, CI

- `tools/gen-third-party.ps1` menghasilkan `THIRD_PARTY_LICENSES.md` dari dependensi yang masuk
  ke binary Windows, ditambah tiga font; `-Check` memastikan berkas itu tidak basi.
- `cargo-deny` dengan `deny.toml`. Temuan: `ttf-parser` (lewat `fontdue`) ditandai **tidak lagi
  dipelihara** (RUSTSEC-2026-0192). Bukan kerentanan, dan hanya membaca font yang ditanam
  sendiri, jadi dibuat pengecualian spesifik dengan alasan dan syarat tinjau ulang.
- **Insiden lingkungan:** `cargo install cargo-deny` gagal di tahap link (`LNK1143`). Penyebabnya
  variabel `CC`/`CXX` di profil pengguna yang menunjuk ke pembungkus kompilator; dimatikan hanya
  di lingkup proses pemasangan, tanpa mengubah konfigurasi pengguna.
- CI diubah: Windows wajib, Ubuntu dan macOS eksperimental, ditambah job `deny`.

### 9b. Dokumentasi yang diuji

- README, [configuration.md](configuration.md), CONTRIBUTING, CHANGELOG, tangkapan layar tiga
  tema.
- **Dokumentasi diuji terhadap kode** (`docs_examples.rs`): blok TOML harus config valid,
  dokumen harus menyebut semua aksi/tema/font, tautan relatif harus menunjuk berkas yang ada.
  Pemeriksa itu sendiri diuji dengan **merusak dokumen secara sengaja** (empat cara) dan
  memastikan masing-masing tertangkap, lalu mengembalikannya.
- Klaim di dokumen diuji sungguhan: perintah `notepad (.\koha.exe --print-config-path)` ternyata
  menghasilkan string kosong (PowerShell tidak menunggu aplikasi GUI kecuali keluarannya
  dialirkan lewat pipa); dokumen diperbaiki.

### 9c. Pengemasan

- `tools/package-release.ps1` mengemas zip + SHA-256 dari **klon bersih** sebuah commit atau tag,
  dengan gerbang yang membuatnya menolak mengemas bila ada yang salah.
- **Temuan penting:** binary rilis menanam `C:\Users\<nama>\.cargo\registry\...` sebanyak 62
  kali (jalur sumber dependensi untuk pesan panic). `trim-paths` di Cargo belum stabil (butuh
  nightly), jadi skrip memakai `--remap-path-prefix` dan menjadikan "tidak memuat nama atau
  jalur lokal" sebagai gerbang. Hasil: nol kemunculan, dan dipastikan lagi dengan alat lain.
- README di dalam zip awalnya menaut ke berkas yang tidak ikut dikemas; kini berkasnya ikut
  dan ada gerbang tautan atas isi paket.
- Build **tidak byte-reproducible** (dua build bersih menghasilkan ukuran sama tetapi hash
  berbeda); dicatat, bukan disembunyikan.

### 9d. Daftar periksa rilis, dan CI yang merah

- `docs/releasing.md` dan `tools/release-notes.ps1` (catatan rilis dari CHANGELOG, tautan
  relatif diubah menjadi absolut).
- **Temuan:** hasil CI dibaca lewat API publik GitHub, dan ternyata **CI Windows merah sejak
  tahap 5**, padahal lokal hijau. Penyebabnya satu tes yang mengganti `\n` dengan `\r\n` pada
  berkas yang di CI sudah CRLF (`\r\r\n` ditolak parser TOML). Direproduksi lokal dengan klon
  `core.autocrlf=true`, diperbaiki, dan terbukti hijau di CI.
- Skrip pemasangan `tools/install-shortcut.ps1` untuk Lenovo Vantage (menyalin `koha.exe` ke
  `%LOCALAPPDATA%\Programs\koha`, membuat pintasan Start Menu, menolak build debug dan menolak
  menimpa pintasan milik program lain). Diuji di folder sandbox dengan sembilan skenario, lalu
  dipasang sungguhan.

---

## Konsep Rust menurut urutan munculnya

| Tahap | Konsep |
|---|---|
| 1 | Workspace, crate vs package, `lib.rs`/`main.rs`, path dependency |
| 2a | `Copy`/`Clone`, `const fn`, `FromStr`, `thiserror`, lifetime `'static` |
| 2b | `serde` (derive, atribut), pola Raw → domain, enum rekursif, `collect()` ke `Result`, raw string |
| 2c | Memiliki vs meminjam, lifetime elision, `let ... else`, `match` menyeluruh, aritmetika `usize` |
| 2d | Enum dengan data, `BTreeSet`, `Option::take`, memindahkan nilai keluar agar pinjaman lepas |
| 3 | `ApplicationHandler`, `Option<T>` yang diisi belakangan, `Rc`, callback `FnMut`, urutan `Drop` |
| 4 | `include_bytes!`, `HashMap::entry`, peminjaman field terpisah dalam closure, golden test |
| 5 | `PathBuf`/`Path`, `io::Result`, injeksi dependensi, penulisan atomik, `ExitCode`, derive `clap` |
| 6 | `unsafe` dengan `// SAFETY:`, `#[cfg(windows)]`, trait object `&dyn`, callback C `extern "system"` |
| 8 | Fitur Cargo dan kompilasi bersyarat, atribut tingkat crate, profil rilis |
| 9 | Pengujian dokumentasi, `include_str!` pada berkas repositori, perilaku akhir baris |

## Kesalahan dan koreksi

Daftar ini sengaja lengkap, karena sebagian besar pelajaran ada di sini.

| Kesalahan | Terdeteksi oleh | Perbaikan |
|---|---|---|
| Komponen C++ Visual Studio belum lengkap (`msvcrt.lib`) | `cargo build` | Pasang workload C++ |
| `from_str_radix` menerima `+`; tes salah hitung byte | Tes | Cek digit manual; dua kasus tes |
| Raw string `r#"..."#` berakhir di `"#FFFFFF"` | Kompiler | `r##"..."##` |
| `SoftBufferError` bukan `Send + Sync` | Kompiler (`anyhow`) | `GuiError::Draw(String)` |
| Menu menutup sendiri: event tombol sintetis | Uji ujung-ke-ujung + log sementara | `map_key` mengabaikan event sintetis |
| Skrip uji menulis state asli pengguna | Pemeriksaan sesudah uji | Hapus berkas; uji pakai `--state` sementara |
| Skrip Python merusak escape `\n` di kode Rust | Kompiler dan `clippy` | Kembalikan ke `\n`; hindari escape lewat skrip penyunting |
| Backslash hilang di skrip PowerShell hasil penyuntingan (`TrimEnd('')`) | Membaca isi berkas | `[char]92` |
| `SoftBufferError`/`$PSScriptRoot`/assembly zip: kesalahan kecil di skrip | Dijalankan | Diperbaiki satu per satu |
| Perintah README `notepad (.\koha.exe ...)` kosong | Mengujinya | Dokumen diperbaiki |
| README di zip menaut berkas yang tidak dikemas | Memeriksa isi zip | Berkas ikut dikemas + gerbang tautan |
| Binary membocorkan nama pengguna (62 jalur) | Memindai binary | `--remap-path-prefix` + gerbang |
| Tes CRLF gagal hanya di CI Windows | Membaca hasil CI | Normalkan ke LF dulu; reproduksi dengan `autocrlf=true` |
| Dugaan "Ubuntu/macOS CI akan merah" keliru | Membaca hasil CI | Dokumen platform dikoreksi |
| `sleep` tidak bekerja di Modern Standby | Uji pemilik | Dicabut dari config contoh; dicatat |

## Mengulang setiap tahap

Setiap commit di tabel di atas bisa dibangun dan dites sendiri. Lakukan di **klon terpisah**
supaya repositori kerjamu tidak terganggu:

```powershell
git clone https://github.com/zakiburnama/GUI-KOHA_Rust.git koha-sejarah
cd koha-sejarah
git checkout 1c905c4          # contoh: akhir langkah 2b (HEAD terlepas, ini wajar)
cargo test --workspace        # harus melaporkan 42 tes lolos
git checkout master           # kembali ke versi terbaru
```

Dua catatan: pada commit `a657f4f` sampai `02456b5` tepat satu tes gagal (tes akhir baris CRLF
yang diperbaiki di `5111e86`); itu juga penyebab CI Windows merah. Dan beberapa tahap awal belum punya `Cargo.lock` yang sama dengan sekarang, sehingga versi
dependensi mengikuti berkas kunci pada commit itu.

## Yang belum selesai

- Linux dan macOS (tahap 7); titik lanjutnya di [platforms.md](platforms.md).
- Pengujian dari tombol Lenovo Vantage secara menyeluruh, dan perilaku fokus saat dipanggil
  dari tombol.
- Kandidat optimasi waktu start (jendela tampil setelah gambar pertama siap, font paralel,
  jalur folder lebih murah).
- Mengganti `fontdue` bila `ttf-parser` makin bermasalah.
- Plugin untuk fitur personal (pengeluaran, waktu sholat, ...) dan Search Apps.
- Lisensi, dan penerbitan `v0.1.0` mengikuti [releasing.md](releasing.md).
