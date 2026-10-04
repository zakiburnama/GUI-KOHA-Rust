# Arsitektur KOHA

Dokumen ini menjelaskan **bagaimana KOHA bekerja** dan **apa maksud tiap direktori dan modul**.
Untuk *cara membangunnya selangkah demi selangkah* lihat
[development-journey.md](development-journey.md); untuk *cara memakainya* lihat
[README](../README.md) dan [configuration.md](configuration.md).

## 1. Gambaran

KOHA adalah popup menu keyboard-saja seperti rofi. Ia **program sekali jalan**: dipanggil oleh
sesuatu (pintasan, tombol Lenovo Vantage), menampilkan menu, menjalankan satu aksi, lalu
**keluar**. Tidak ada proses yang tertinggal, tidak ada pendengar hotkey global, dan seluruh
program berjalan di satu thread utama.

```text
tombol / pintasan
      │
      ▼
 koha.exe ──► baca config + state ──► bangun MenuState (state machine)
                                          │
                       ┌──────────────────┘
                       ▼
          jendela popup  (winit + softbuffer)
            ▲    │  tombol  ─► Input ─► MenuState.update ─► Effect
            │    ▼
        gambar ulang: render menu ke buffer piksel (tata letak + teks + warna tema)
                       │  Effect::Run(aksi)  atau  Effect::Close
                       ▼
          jendela DIHANCURKAN (fokus kembali ke aplikasi sebelumnya)
                       ▼
          execute(aksi) lewat trait Platform  ──►  proses keluar
```

Dua hal pada diagram itu adalah inti rancangannya:

1. **Keputusan dipisahkan dari efek.** Apa yang terjadi saat sebuah tombol ditekan diputuskan
   oleh fungsi murni (`MenuState::update`) yang hanya mengembalikan *deskripsi* efek. Jendela
   dan OS yang melaksanakannya.
2. **Aksi dijalankan sesudah jendela hilang**, bukan di tengah event loop. Itu membuat fokus
   sudah kembali ke aplikasi sebelumnya ketika program yang dipilih diluncurkan.

## 2. Prinsip desain

| Prinsip | Wujud di kode | Alasan |
|---|---|---|
| **Inti murni**, tanpa OS dan tanpa GUI | `koha-core` tidak bergantung pada crate lain dan tidak memanggil API OS | Semua logika menu, config, dan tema bisa diuji unit tanpa tiruan (mock) |
| **Ports and adapters** | Trait `Platform` di `koha-platform`; implementasi per OS dipilih dengan `cfg` | OS baru cukup menambah satu adapter; inti tidak berubah |
| **Render terpisah dari jendela** | `render` menulis ke `&mut [u32]`, bukan ke jendela | Rendering bisa dites dengan memeriksa isi buffer, tanpa membuka jendela |
| **State machine murni** | `MenuState::update(Input) -> Effect` | Perilaku navigasi (termasuk kasus tepi) dites sebagai fungsi biasa |
| **Gagal keras untuk config, toleran untuk state** | `load_config` menolak berkas rusak; `load_state` jatuh ke bawaan | Config milik pengguna: salah tulis harus ketahuan. State milik aplikasi: tidak boleh menghalangi KOHA berjalan |
| **Waktu start adalah fitur** | Tanpa thread latar, `ControlFlow::Wait`, hanya font tema aktif yang dimuat, fitur `startup-trace` bernilai nol saat mati | KOHA dipanggil lewat tombol; terasa lambat berarti gagal |
| **Galat yang bisa ditindaklanjuti** | Pesan menunjuk baris dan kolom, dialog bila tidak ada konsol | Pengguna tidak melihat stderr saat KOHA dipanggil dari tombol |
| **Dokumentasi diuji** | `koha-core/src/docs_examples.rs` | Contoh di dokumen tidak boleh membusuk |

## 3. Peta direktori

```text
gui-koha-rust/
├─ Cargo.toml              workspace: versi bersama, dependensi internal, profil rilis
├─ Cargo.lock              versi dependensi yang terkunci (ikut di-commit)
├─ deny.toml               kebijakan cargo-deny: lisensi, advisory, sumber
├─ THIRD_PARTY_LICENSES.md daftar lisensi dependensi + font (DIHASILKAN, jangan diedit)
├─ README.md  CHANGELOG.md  CONTRIBUTING.md
├─ .github/workflows/ci.yml   CI: fmt, clippy, test, deny (Windows wajib; Ubuntu/macOS eksperimental)
├─ crates/
│  ├─ koha-core/           logika murni
│  │  ├─ src/              lib, rgb, theme, config, state, menu, docs_examples (khusus tes)
│  │  └─ src/default_config.toml   config contoh yang ditanam ke binary
│  ├─ koha-platform/       trait Platform + implementasi per OS
│  │  ├─ src/              lib, cmdline, window_filter, win (Windows), unsupported (lainnya)
│  │  └─ examples/list_windows.rs  mode kering "Close All Windows"
│  ├─ koha-gui/            jendela, tata letak, teks, render
│  │  ├─ src/              lib, layout, canvas, fonts, text, render, input, window, trace
│  │  ├─ assets/fonts/     tiga font OFL + teks lisensinya (ditanam ke binary)
│  │  └─ tests/snapshots/  acuan seni-ASCII untuk tes render teks
│  └─ koha-app/            binary `koha`
│     └─ src/              main, cli, paths, store, diagnostics, report
├─ docs/                   dokumentasi (dokumen ini ada di sini)
└─ tools/                  skrip PowerShell: ukur waktu start, daftar lisensi, kemas rilis, pasang pintasan
```

Yang diabaikan git: `target/` (hasil build) dan `dist/` (hasil pengemasan).

## 4. Ketergantungan antar crate

```text
koha-app ─────────► koha-gui ───────► koha-core
   │                                      ▲
   ├────────────► koha-platform ──────────┘
   │                                      ▲
   └──────────────────────────────────────┘
```

Aturannya: `koha-core` tidak bergantung pada crate lain; `koha-platform` dan `koha-gui`
bergantung pada `koha-core` tetapi **tidak saling mengenal**; hanya `koha-app` yang mengenal
semuanya dan merangkainya. Dependensi pihak ketiga langsung:

| Crate | Dependensi | Dipakai untuk |
|---|---|---|
| `koha-core` | `serde`, `toml`, `thiserror` | Membaca/menulis config dan state; tipe galat |
| `koha-platform` | `thiserror`; `windows` (hanya Windows) | Galat; API Win32 |
| `koha-gui` | `winit`, `softbuffer`, `fontdue`, `thiserror` | Jendela dan event; menampilkan buffer piksel; rasterisasi font; galat |
| `koha-app` | `clap`, `directories`, `anyhow`, `thiserror`; `tempfile` (khusus tes) | Argumen CLI; lokasi berkas; galat tingkat aplikasi; folder sementara di tes |

`thiserror` dipakai di crate library; `anyhow` **hanya** di `koha-app`, tempat galat dilaporkan
ke pengguna.

## 5. `koha-core`: logika murni

Tidak ada panggilan OS dan tidak ada GUI. Hanya mengurai dan menghasilkan teks (membaca dan
menulis berkas adalah tugas `koha-app`).

| Modul | Tanggung jawab | Tipe/fungsi kunci |
|---|---|---|
| [`rgb.rs`](../crates/koha-core/src/rgb.rs) | Warna 24-bit | `Rgb`; `FromStr` (`#RRGGBB`/`RRGGBB`), `Display`, `to_u32()` (`0x00RRGGBB`, format piksel `softbuffer`), `Deserialize` dari string |
| [`theme.rs`](../crates/koha-core/src/theme.rs) | Model tema | `Theme` (`bg, fg, sel_bg, sel_fg, bezel, font`), `ThemeSet` (tujuh tema bawaan, `add_or_replace`, `get_or_default`), konstanta id font |
| [`config.rs`](../crates/koha-core/src/config.rs) | Config TOML: parse dan validasi | `Config`, `MenuItem`, `ItemKind`, `Action`, `Builtin`, `ConfigError`, `DEFAULT_CONFIG` |
| [`state.rs`](../crates/koha-core/src/state.rs) | State yang ditulis aplikasi | `State { version, theme, hidden }` |
| [`menu.rs`](../crates/koha-core/src/menu.rs) | State machine navigasi | `MenuState`, `Input`, `Effect`, `Row` |
| [`default_config.toml`](../crates/koha-core/src/default_config.toml) | Menu contoh bawaan | Ditanam dengan `include_str!`; diuji selalu valid |
| [`docs_examples.rs`](../crates/koha-core/src/docs_examples.rs) | Menguji dokumentasi | Hanya dikompilasi saat tes |

### 5.1 Config: dua tahap

```text
teks TOML ──► Raw* (bentuk persis berkas) ──► validasi ──► Config (tipe domain)
              │                                  │
   galat sintaks/tipe/field asing:        galat aturan (id ganda, label kosong, ...):
   ConfigError::Syntax {baris, kolom}     ConfigError::Invalid {jalur item, pesan}
```

- `RawItem` sengaja satu bentuk **datar** untuk semua jenis item (`exec`, `url`, `builtin`,
  `submenu`); `reject_foreign_fields` menolak field yang tidak berlaku untuk `type`-nya dan
  menyebut nama field-nya.
- Versi dibaca lebih dulu dengan `VersionProbe` (yang mengabaikan field lain), supaya berkas
  dari versi masa depan menghasilkan "versi tidak didukung", bukan galat field asing.
- Aturan `id`: huruf kecil, angka, `_`, `-`, dan unik di seluruh pohon (termasuk submenu).
  `id` dipakai untuk mengingat status ON/OFF, sehingga mengganti `label` tidak menghilangkannya.
- Galat sintaks membawa baris dan kolom (dari rentang byte yang dilaporkan parser TOML).
  Galat aturan membawa **jalur item** (`menu[1].items[0]`), karena `serde` tidak menyimpan
  posisi sumber setelah nilai dibaca.

### 5.2 `MenuState`: inti perilaku

`MenuState` **memiliki** salinan pohon menu (bukan meminjamnya) dan hanya menyimpan keadaan
navigasi:

| Field | Arti |
|---|---|
| `root` | Pohon menu dari config |
| `themes` | `ThemeSet` (bawaan + tema dari config) |
| `state` | `State` saat ini (tema aktif, item yang di-OFF) |
| `path` | Indeks submenu yang sedang dimasuki, dari root |
| `overlay` | `None`, `ThemePicker`, atau `MenuSettings` (tampilan khusus di atas jalur) |
| `selected` | Baris terpilih |
| `closed` | `true` setelah `Run` atau `Close` dikeluarkan |

Satu fungsi privat, `entries()`, menghasilkan daftar `{label, target}` untuk tampilan aktif.
`rows()` memetakan ke label, dan `Enter` mencocokkan `target`. Dengan begitu keduanya **tidak
mungkin tidak sinkron**, dan tidak ada pencocokan teks label (yang dinamis: `(ON)`, `(current)`).
`Target` bisa `Item(indeks asli)`, `MenuSettings`, `Theme(nama)`, atau `Toggle(id)`.

| Input | Menu biasa | Pemilih tema / Menu Settings |
|---|---|---|
| `Up` / `Down` | Seleksi bergerak melingkar → `Redraw` | Sama |
| `Enter` | Submenu: masuk → `Redraw`. Aksi: `Run(aksi)` dan `closed`. `theme_picker`: buka overlay | Tema: ganti tema → `StateChanged`. Toggle: balik ON/OFF → `StateChanged` |
| `Back` | Naik satu tingkat (seleksi kembali ke baris pertama) → `Redraw`; di root: `Close` | Lepas overlay → `Redraw` |
| `Dismiss` | `Close` | `Close` |

Dua detail yang disengaja:

- **Setelah `closed`, semua input diabaikan** (`Effect::None`). Menutup jendela sendiri memicu
  event "kehilangan fokus" yang menyusul; tanpa penjaga ini event itu bisa membatalkan aksi
  yang baru saja diputuskan (bug yang harus ditambal manual di versi AutoHotkey).
- **Baris "Menu Settings" hanya ada di menu utama**, selalu terakhir, dan tidak bisa di-OFF,
  supaya selalu ada jalan kembali bila semua item pernah disembunyikan. Filter `hidden`
  hanya berlaku di menu utama, dan memakai indeks asli sehingga menyembunyikan satu item tidak
  menggeser aksi item lain.

`Effect` yang dihasilkan: `None`, `Redraw` (gambar ulang; tinggi jendela bisa berubah),
`StateChanged` (gambar ulang + terapkan tema + simpan state), `Run(Action)`, `Close`.

### 5.3 Tema dan font

`Theme.font` hanyalah **id** (`press-start-2p`, `vt323`, `ibm-plex-mono`). Core tidak tahu apa
pun soal berkas font; `koha-gui` yang memetakannya. Tema aktif ditentukan oleh `State.theme`,
dan bila namanya tidak dikenal jatuh ke `amber` (`DEFAULT_THEME`).

## 6. `koha-platform`: adapter OS

| Modul | Tanggung jawab |
|---|---|
| [`lib.rs`](../crates/koha-platform/src/lib.rs) | Trait `Platform`, `PlatformError`, `execute`, dan pemilihan implementasi lewat `cfg` |
| [`cmdline.rs`](../crates/koha-platform/src/cmdline.rs) | `quote_arg`/`join_args` (aturan baris perintah Windows) dan `validate_url`; murni, dikompilasi di semua OS |
| [`window_filter.rs`](../crates/koha-platform/src/window_filter.rs) | `is_closable`: aturan jendela mana yang boleh ditutup "Close All Windows" (kriteria Alt+Tab); murni |
| [`win.rs`](../crates/koha-platform/src/win.rs) | `WindowsPlatform` (hanya Windows) dengan crate `windows` |
| [`unsupported.rs`](../crates/koha-platform/src/unsupported.rs) | `UnsupportedPlatform` untuk OS lain: semua operasi mengembalikan `Unsupported` |

```rust
pub trait Platform {
    fn launch(&self, command: &str, args: &[String], admin: bool) -> Result<(), PlatformError>;
    fn open_url(&self, url: &str) -> Result<(), PlatformError>;
    fn lock(&self) -> Result<(), PlatformError>;
    fn sleep(&self) -> Result<(), PlatformError>;
    fn close_all_windows(&self) -> Result<(), PlatformError>;
}
```

`execute(&dyn Platform, &Action)` mengubah sebuah `Action` menjadi panggilan metode yang tepat
(dan memvalidasi URL lebih dulu). `Builtin::ThemePicker` ditolak dengan `NotExecutable`, karena
itu ditangani menu, tidak pernah sampai ke platform. `SystemPlatform` adalah alias yang dipilih
saat kompilasi: `WindowsPlatform` di Windows, `UnsupportedPlatform` selain itu.

Cara `WindowsPlatform` bekerja:

| Operasi | Mekanisme |
|---|---|
| `launch` | `ShellExecuteExW`. Verb `open`, atau `runas` bila `admin` (memunculkan UAC; menolaknya menghasilkan `Cancelled`, bukan galat). Opsi `NOASYNC`, `DOENVSUBST` (perluas `%VAR%` di `command`), `FLAG_NO_UI`. COM diinisialisasi sekali |
| `open_url` | `ShellExecuteExW` dengan URL yang sudah divalidasi (harus diawali skema, supaya jalur berkas tidak dijalankan) |
| `lock` | `LockWorkStation` |
| `sleep` | `SetSuspendState`. **Tidak bekerja di PC yang hanya punya Modern Standby tanpa hibernasi** |
| `close_all_windows` | `EnumWindows` mengumpulkan `WindowInfo`, `is_closable` menyaring, lalu `PostMessageW(WM_CLOSE)`; tidak menunggu dan tidak memaksa |

Dua fungsi khusus Windows di luar trait: `attach_parent_console()` (menempel ke konsol terminal
induk bila ada, karena build rilis tidak punya konsol sendiri) dan `show_error_dialog()`
(`MessageBoxW`). Kode `unsafe` terbatas di `win.rs`, setiap blok kecil dan diberi komentar
`// SAFETY:`.

## 7. `koha-gui`: jendela, tata letak, teks

| Modul | Tanggung jawab |
|---|---|
| [`layout.rs`](../crates/koha-gui/src/layout.rs) | `Layout::new/fitting`: ukuran jendela dan kotak tiap baris dari jumlah baris, skala DPI, dan lebar teks; `centered_position` |
| [`canvas.rs`](../crates/koha-gui/src/canvas.rs) | `Canvas`: bungkus `&mut [u32]` + ukuran; semua penulisan dipotong ke batas; `fill_rect`, `blend_pixel` |
| [`fonts.rs`](../crates/koha-gui/src/fonts.rs) | `FONTS`: tiga font yang ditanam (`include_bytes!`), ukuran logis, dan `pixel_size(scale)` |
| [`text.rs`](../crates/koha-gui/src/text.rs) | `TextRenderer`: muat satu font (`fontdue`), cache glyph, `measure`, `baseline`, `draw` |
| [`render.rs`](../crates/koha-gui/src/render.rs) | `render`: menggambar bezel, baris, dan teks dari `MenuState`; `row_text` (`> ` atau dua spasi) |
| [`input.rs`](../crates/koha-gui/src/input.rs) | `map_key` (tombol → `Input`) dan `FocusGate` (aturan dismiss-on-blur) |
| [`window.rs`](../crates/koha-gui/src/window.rs) | `run`: jendela `winit` + `softbuffer`; perekat tipis |
| [`trace.rs`](../crates/koha-gui/src/trace.rs) | Jejak waktu start; hanya aktif dengan fitur `startup-trace` |

Semuanya kecuali `window.rs` adalah fungsi/tipe yang bisa dites **tanpa membuka jendela**.

### 7.1 Tata letak

Ukuran dasar (piksel logis, dari versi AHK): lebar 300, tinggi baris 26, margin 6, jarak teks 8.
Dikali skala DPI, dan **tiap tepi dibulatkan sendiri** supaya baris yang bersebelahan tidak
punya celah satu piksel pada skala pecahan. Lebar minimum 300; jendela melebar bila teks
terpanjang tidak muat (`fitting`), dibatasi 90% lebar monitor, dan teks yang masih terlalu
panjang dipotong di tepi baris.

### 7.2 Teks dan font

| Font | Dipakai oleh tema | Sifat |
|---|---|---|
| `press-start-2p` | `game_boy` | Tajam sempurna hanya pada kelipatan 8 px, jadi ukurannya **dipaku** ke kelipatan bulat |
| `vt323` | `amber`, `green_term` | Bukan bitmap sejati (tepinya halus di semua ukuran); 20 px dipilih karena lebar karakternya tepat 8 px |
| `ibm-plex-mono` | empat tema lain dan tema buatan | Font outline biasa |

`TextRenderer` hanya memegang satu font pada satu waktu (font tema aktif) dan menyimpan glyph
yang sudah dirasterisasi; cache dibuang saat font atau ukuran berganti. Glyph dicampur ke
buffer dengan alpha blending bilangan bulat. Karakter di luar cakupan font tampil sebagai kotak.

### 7.3 `window.rs`

`run(menu, on_state)` membuat `EventLoop` dengan `ControlFlow::Wait` (tidur sampai ada event,
CPU idle nol) dan satu `App` yang mengimplementasi `ApplicationHandler`. Saat event loop
berjalan (`resumed`) ia membuat jendela:

1. memilih monitor utama dan skalanya, memuat font, menghitung tata letak;
2. membuat jendela **tak terlihat**: tanpa border, tidak bisa di-resize, selalu di atas, tidak
   tampil di taskbar (Windows);
3. membuat `softbuffer` `Context` dan `Surface`, menetapkan ukuran dan posisi tengah, lalu
   menampilkan dan meminta fokus.

Event yang ditangani: `CloseRequested` dan `Focused(false)` (setelah pernah fokus) menjadi
`Dismiss`; `KeyboardInput` melewati `map_key` menjadi `Input`; `ModifiersChanged` melacak
Shift; `ScaleFactorChanged`/`Resized` memicu tata letak ulang; `RedrawRequested` menggambar.
Setiap `Input` diteruskan ke `MenuState::update`, dan `Effect`-nya dilaksanakan:

| `Effect` | Yang dilakukan jendela |
|---|---|
| `Redraw` | Hitung ulang tata letak (ukuran bisa berubah) lalu minta gambar ulang |
| `StateChanged` | Panggil `on_state(state)` (aplikasi menyimpan), lalu seperti `Redraw`; font disamakan dengan tema aktif |
| `Run(aksi)` | Simpan aksi sebagai hasil, hentikan event loop |
| `Close` | Hentikan event loop |

`run` baru **kembali setelah jendelanya dihancurkan** (`drop(app)`), lalu mengembalikan
`Some(aksi)` atau `None`. Struktur kode itulah yang menjamin "tutup dulu, baru jalankan aksi",
tanpa flag manual.

`map_key` mengikuti perilaku versi AHK: hanya `↑ ↓ Tab Shift+Tab Enter Esc` yang diterima,
Shift sendirian diabaikan, tombol lain menutup menu, dan `Enter`/`Esc` yang berulang (tombol
ditahan) diabaikan. **Event tombol sintetis diabaikan**: saat jendela mendapat fokus, `winit`
menerbitkan penekanan palsu untuk tombol yang dianggap sedang ditahan (misalnya tombol pemicu
yang masih tertahan), dan tanpa pengecualian ini menu akan langsung menutup sendiri.

## 8. `koha-app`: perangkai

| Modul | Tanggung jawab |
|---|---|
| [`main.rs`](../crates/koha-app/src/main.rs) | Urutan start; satu-satunya tempat yang mengenal semua crate |
| [`cli.rs`](../crates/koha-app/src/cli.rs) | Definisi argumen (`clap`): `--config`, `--state`, `--init`, `--print-config-path`, `--print-state-path` |
| [`paths.rs`](../crates/koha-app/src/paths.rs) | Menentukan lokasi config dan state (flag › variabel lingkungan › standar). Environment dan folder dasar **disuntikkan**, jadi mudah dites |
| [`store.rs`](../crates/koha-app/src/store.rs) | Membaca config dan state, menyimpan state secara atomik, menulis config contoh. Satu-satunya yang menyentuh filesystem |
| [`diagnostics.rs`](../crates/koha-app/src/diagnostics.rs) | Mengubah `ConfigError` + isi berkas menjadi pesan dengan potongan baris dan tanda `^` |
| [`report.rs`](../crates/koha-app/src/report.rs) | Memilih tujuan galat: stderr bila ada konsol, dialog bila tidak |

Aturan di `store.rs`:

- **Config**: berkas tidak ada dan lokasinya bukan pilihan pengguna → menu contoh bawaan
  (tanpa membuat berkas). Lokasinya dipilih pengguna (`--config`/`KOHA_CONFIG`) tetapi tidak
  ada → galat. Berkas ada tetapi rusak → galat dengan baris dan kolom; **tidak pernah**
  jatuh diam-diam ke bawaan. BOM UTF-8 dan akhir baris CRLF ditoleransi.
- **State**: hilang → bawaan; rusak → bawaan + peringatan. Disimpan **atomik** (tulis ke
  `<berkas>.tmp` di folder yang sama, lalu `rename`), sehingga tidak pernah setengah tertulis.
- **`--init`** memakai `create_new(true)`: pembuatan berkas yang atomik dan tidak pernah menimpa.
- Bila config ditentukan pengguna tetapi state tidak, state diletakkan di sebelah config itu,
  supaya mencoba config lain tidak menimpa state harian.

## 9. Urutan start, langkah demi langkah

Fungsi-fungsi di bawah ada di `koha-app/src/main.rs` kecuali disebut lain.

1. `main`: `trace::start()` (kosong tanpa fitur jejak), lalu `attach_parent_console()` agar
   keluaran terlihat bila dijalankan dari terminal.
2. `run`: `Cli::try_parse()`. `--help`/`--version` dicetak lalu selesai; salah argumen
   dilaporkan lewat `report::error` (kode keluar 2).
3. `paths::resolve` menentukan lokasi config dan state. `--print-config-path`,
   `--print-state-path`, dan `--init` diselesaikan di sini lalu keluar.
4. `store::load_config` (→ `Config`) dan `store::load_state` (→ `State`).
5. `MenuState::new(config, state)`.
6. Sebuah closure `on_state` dibuat: menyimpan state bila berbeda dari yang terakhir tersimpan;
   kegagalan menyimpan hanya peringatan.
7. `koha_gui::run(menu, &mut on_state)`: jendela dan event loop (bagian 7.3). Kembali setelah
   jendela dihancurkan.
8. Bila ada aksi: `koha_platform::execute(&SystemPlatform::default(), &aksi)`. `Cancelled`
   (UAC ditolak) diam-diam dianggap sukses; galat lain dilaporkan.
9. Proses keluar. Tidak ada yang tertinggal.

Waktu antar tahap (dari awal `main` sampai gambar pertama) diukur dan dicatat di
[performance.md](performance.md); totalnya sekitar 34 ms pada satu laptop.

## 10. Penanganan kegagalan

| Keadaan | Perilaku |
|---|---|
| Config tidak ada (lokasi bawaan) | Menu contoh bawaan; satu baris `info` ke stderr |
| Config tidak ada (lokasi pilihan pengguna) | Galat; kode keluar 1 |
| Config rusak | Galat dengan berkas, baris, kolom, dan potongan baris; kode keluar 1 |
| State hilang / rusak | Bawaan / bawaan + peringatan; KOHA tetap jalan |
| State gagal disimpan | Peringatan; menu tetap jalan |
| Font gagal dimuat, jendela gagal dibuat | `GuiError`, dilaporkan; seharusnya tidak terjadi (font tertanam dan dites) |
| Aksi gagal diluncurkan | Galat dengan pesan dari OS |
| URL tanpa skema | Ditolak sebelum menyentuh OS |
| Operasi belum didukung di OS ini | `Unsupported("nama operasi")` |
| Pengguna menolak UAC | Bukan galat; keluar tanpa pesan |

**Ke mana galat dilaporkan** (`report.rs`): bila ada konsol (dijalankan dari terminal),
ke stderr; bila tidak (dijalankan dari tombol atau pintasan), sebagai dialog `MessageBox`,
karena stderr tidak dilihat siapa pun. Build rilis memakai subsistem GUI (tanpa jendela
konsol), jadi tidak ada kilatan konsol saat KOHA dipanggil.

## 11. Pengujian

Total **262 tes** di `cargo test --workspace`:

| Crate | Tes | Cakupan utama |
|---|---|---|
| `koha-core` | 93 | `config` 28, `menu` 31, `rgb`/`state`/`theme` 8 masing-masing, dokumentasi 9 |
| `koha-gui` | 83 | `text` 22, `layout` 18, `render` 13, `input` 12, `canvas` 8, `fonts` 8, jejak 1 |
| `koha-platform` | 37 | `cmdline` 13, `window_filter` 10, `lib` (tiruan Platform) 9, `win` 5 |
| `koha-app` | 49 | `store` 18, `paths` 12, `cli` 7, `diagnostics` 7, `report` 5 |

Lapisannya:

- **Tes unit** di modul `#[cfg(test)]` di berkas yang sama dengan kodenya.
- **Platform tiruan** (`Mock`) mencatat panggilan, sehingga `execute` dites tanpa menyentuh OS.
- **Tes snapshot** untuk rendering teks: seni-ASCII per font disimpan di
  `koha-gui/tests/snapshots/` dan dibandingkan (`UPDATE_SNAPSHOTS=1` untuk memperbaruinya).
- **Tes dokumentasi** (`docs_examples.rs`): blok TOML di dokumen harus valid, dokumen harus
  menyebut semua aksi/tema/font, dan semua tautan relatif harus menunjuk berkas yang ada.
- **Fitur jejak** punya varian tes sendiri (`--features startup-trace`).
- **CI** (`.github/workflows/ci.yml`) menjalankan semuanya di mesin bersih, termasuk
  `cargo deny check`. Windows wajib; Ubuntu dan macOS eksperimental.
- **Manual**: perilaku jendela yang sebenarnya (fokus, posisi, tampilan, tombol Vantage) tidak
  bisa dites unit dan diperiksa dengan menjalankannya.

## 12. Build dan profil

- **Fitur Cargo**: `startup-trace` (mati secara bawaan; fungsi jejak menjadi kosong dan
  dihapus kompiler, terbukti ukuran binary tidak berubah).
- **Profil rilis** (`Cargo.toml` akar): `opt-level 3`, `lto`, `codegen-units = 1`,
  `panic = "abort"`, `strip`. Dipilih dari pengukuran; alasannya ada di
  [performance.md](performance.md).
- **Subsistem**: build rilis `windows_subsystem = "windows"`; build debug tetap konsol.
- **Aset ditanam**: font (`include_bytes!`) dan config contoh (`include_str!`), jadi binary
  mandiri (~1,5 MB) dan tidak membaca berkas aset saat berjalan.
- **Pengemasan** menambah `--remap-path-prefix` supaya binary tidak memuat jalur lokal
  pembuatnya (lihat [tools/README.md](../tools/README.md)).

## 13. Keputusan arsitektur dan alternatif yang ditolak

| Keputusan | Alternatif | Mengapa |
|---|---|---|
| `winit` + `softbuffer` (render CPU) | `egui`, Win32 langsung, Tauri/WebView2 | Start cepat dan ukuran kecil; lintas OS; menu beberapa baris tidak butuh GPU. Tauri/WebView2 sudah ditinggalkan di versi AHK karena berat |
| Config TOML | JSON atau INI | Lazim di ekosistem Rust; komentar; galat dengan baris dan kolom |
| Config terpisah dari state | Satu berkas | Aplikasi tidak pernah menimpa berkas yang diedit pengguna |
| `id` stabil per item | Menyimpan status berdasarkan `label` | Mengganti label tidak menghilangkan status ON/OFF |
| `MenuState` memiliki pohon | Meminjam config (`MenuState<'a>`) | Menghindari struktur yang saling meminjam (self-referential) yang sulit di Rust; biayanya satu salinan kecil |
| Aksi dijalankan setelah jendela hancur | Menjalankan di dalam event loop | Fokus sudah kembali; tidak perlu flag "sedang menutup" |
| Lebar jendela tumbuh mengikuti teks | Lebar tetap + memotong | Font pixel yang lebar tidak muat di 300 px |
| Font per tema, tiga font OFL ditanam | Satu font; font sistem | Tema membawa identitasnya; tidak tergantung font di mesin pengguna |
| `clap` dipertahankan | `lexopt`/`pico-args` | Biaya terukur ~0,15 ms dan ~241 KB; di bawah ambang yang disepakati |
| `windows` crate | `winapi`, `std::process::Command` | `ShellExecute` mengenal App Paths, `.lnk`, dan asosiasi berkas seperti `Run()` di AHK |
| OS selain Windows ditunda | Mengerjakan semuanya sekarang | Arsitekturnya siap (`Platform` + `cfg`); titik lanjutnya di [platforms.md](platforms.md) |

## 14. Keterbatasan arsitektural yang diketahui

- Jendela ditengahkan di **monitor utama**, bukan monitor tempat kursor berada.
- Tidak ada mouse; KOHA keyboard-saja.
- Config dibaca saat start; tidak ada muat-ulang.
- Karakter di luar cakupan font tampil sebagai kotak; tidak ada penataan teks kompleks
  (kerning, aksara kanan-ke-kiri) karena semua font monospace.
- `fontdue` bergantung pada `ttf-parser` yang tidak lagi dipelihara; alasan pengecualiannya
  ada di [deny.toml](../deny.toml).
- Build tidak byte-reproducible.
- Hanya Windows yang dijalankan dan diuji; lihat [platforms.md](platforms.md).

## 15. Menambah sesuatu

Tabel "Menambah sesuatu" di [CONTRIBUTING.md](../CONTRIBUTING.md) menunjuk berkas mana yang
disentuh untuk item menu baru, aksi bawaan, tema, font, dukungan OS, atau operasi `Platform`.
