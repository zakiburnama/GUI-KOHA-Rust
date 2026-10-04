# Performa: waktu start dan ukuran binary

Waktu start adalah fitur utama KOHA: ia dipanggil lewat tombol, menampilkan menu,
menjalankan satu aksi, lalu keluar. Dokumen ini mencatat apa yang diukur, bagaimana, apa
hasilnya, dan keputusan yang diambil darinya.

> **Baca ini dulu.** Semua angka berasal dari **satu laptop** (ThinkPad, Windows 11,
> antivirus aktif) pada **dua sesi** pengukuran. Jangan dikutip sebagai klaim umum.
> Cara mengulangnya di mesinmu ada di bagian paling bawah.

## Ringkasan

| Pertanyaan | Jawaban dari data |
|---|---|
| Rust lawan AHK sampai jendela terlihat | Rust ~34 ms, AHK ~68 ms (median, dua sesi konsisten) |
| Berapa "harga minimum" memulai proses | ~16 ms, jadi sekitar separuh waktu Rust adalah OS memulai proses |
| `opt-level` 3 lawan `"z"` | `"z"` konsisten **~2 ms lebih lambat** (35,8 lawan 34,0 ms), tapi 20% lebih kecil |
| LTO + `codegen-units = 1` + `panic = "abort"` + `strip` | **29% lebih kecil**, tanpa menambah waktu start |
| Biaya `clap` | ~0,15 ms waktu, ~241 KB ukuran (16% binary) |
| Peluncuran pertama setelah build/instal | **Lambat sekali sekali saja** (0,6 sampai 5 detik pada exe baru), lalu normal |

## Keputusan

1. **Profil rilis**: `opt-level = 3`, `lto = true`, `codegen-units = 1`, `panic = "abort"`,
   `strip = true` (ditulis di `Cargo.toml` akar). Varian bawaan dan varian `opt-level 3`
   tidak terbedakan dalam waktu start, jadi yang lebih kecil menang. `"z"` tidak dipilih
   karena waktu start adalah prioritas dan selisih ~2 ms-nya konsisten, walau kecil.
   Untuk berpindah ke `"z"`, ubah satu baris `opt-level`.
2. **`clap` dipertahankan.** Aturan yang disepakati: ganti hanya bila biaya waktunya
   terukur lebih dari ~0,5 ms. Hasilnya ~0,15 ms. Menggantinya dengan penguraian manual
   menghemat ~241 KB tetapi tidak ada keuntungan waktu start yang terukur.

## Metode

- Skrip: [`tools/bench-startup.ps1`](../tools/bench-startup.ps1). Mengukur dari sebelum
  `Process.Start` sampai ada jendela **terlihat** berjudul `KOHA` milik proses itu
  (`EnumWindows` dipolling terus-menerus, `Stopwatch` beresolusi tinggi), lalu menghentikan
  prosesnya.
- Target dijalankan **bergantian** (urutannya dibalik tiap putaran genap), 3 putaran
  pemanasan dibuang, lalu 30 pengukuran per target.
- Pembanding dasar: `koha --print-config-path` (proses start dan keluar tanpa jendela).
- AHK: `KOHA.exe` hasil kompilasi dari repo `win-koha-ahk`, apa adanya.
- Rincian tahap di dalam KOHA: build dengan `--features startup-trace`, dibaca lewat `-Trace`.
- Ukuran: `koha.exe` hasil `cargo build --release -p koha-app` dengan tiap profil di
  folder target terpisah.

Tiga profil yang dibandingkan:

| Varian | `opt-level` | `lto` | `codegen-units` | `panic` | `strip` |
|---|---|---|---|---|---|
| A | 3 | tidak | 16 (bawaan) | unwind | tidak |
| B | 3 | ya | 1 | abort | ya |
| C | `"z"` | ya | 1 | abort | ya |

## Hasil

### Ukuran binary

| Varian | Ukuran | Terhadap A | Waktu build |
|---|---|---|---|
| A | 2.107.392 B (2.058,0 KB) | | 77 dtk |
| **B** | **1.505.280 B (1.470,0 KB)** | −29% | 98 dtk |
| C | 1.206.784 B (1.178,5 KB) | −43% | 63 dtk |

Riwayat ukuran (profil bawaan): 132,5 KB tanpa dependensi → 1.000 KB (`winit`,
`softbuffer`) → 1.507 KB (font, `fontdue`) → 2.033 KB (`clap`, `directories`, berkas) →
2.059 KB (`windows`).

### Waktu sampai jendela terlihat (ms)

Sesi 1:

| Target | N | min | median | rata-rata | p90 | maks | peluncuran pertama |
|---|---|---|---|---|---|---|---|
| Rust A | 30 | 32,6 | 34,9 | 36,4 | 38,2 | 60,1 | 616,1 |
| Rust B | 30 | 32,1 | 34,3 | 41,9 | 36,0 | 241,8 | 4.982,8 |
| Rust C | 30 | 34,1 | 36,2 | 44,2 | 42,3 | 198,7 | 783,8 |
| AHK | 30 | 63,8 | 68,5 | 71,0 | 79,1 | 98,5 | 68,1 |
| Dasar (`--print-config-path`) | 30 | 14,7 | 16,1 | 16,1 | 16,9 | 18,6 | 19,0 |

Sesi 2 (diulang tanpa mengubah apa pun):

| Target | N | min | median | rata-rata | p90 | maks | peluncuran pertama |
|---|---|---|---|---|---|---|---|
| Rust A | 30 | 32,1 | 34,0 | 34,2 | 35,8 | 36,2 | 45,8 |
| Rust B | 30 | 32,2 | 34,0 | 34,7 | 36,8 | 40,8 | 39,0 |
| Rust C | 30 | 34,1 | 35,8 | 35,9 | 37,2 | 39,7 | 38,4 |
| AHK | 30 | 62,9 | 68,4 | 68,4 | 70,9 | 73,6 | 68,1 |
| Dasar (`--print-config-path`) | 30 | 15,5 | 15,8 | 16,0 | 16,6 | 17,1 | 17,6 |

Cara membaca:

- **Median dan p90** lebih bisa dipercaya daripada rata-rata dan maks. Beberapa lonjakan
  200 ms+ di sesi 1 (pada B dan C) adalah gangguan sistem sesaat, dan tidak muncul di sesi 2.
- **Median konsisten antar sesi** (A 34,9 / 34,0; B 34,3 / 34,0; C 36,2 / 35,8; AHK
  68,5 / 68,4). Selisih A dan B tidak terbedakan dari derau. Selisih B dan C (~1,8 ms)
  muncul di kedua sesi.
- **Peluncuran pertama.** Di sesi 1, tiga exe yang baru dibuat memakan 616, 4.983, dan 784 ms
  pada peluncuran pertama; di sesi 2 (exe yang sama, sudah pernah dijalankan) hanya 46, 39,
  dan 38 ms. Polanya jelas: **sekali saja untuk berkas baru**. Penyebabnya *diduga*
  pemindaian antivirus atas berkas yang belum pernah dijalankan; itu belum dibuktikan
  (antivirus tidak dimatikan atau diperiksa). Yang belum diketahui: apakah pengguna yang
  memasang `koha.exe` hasil rilis (tanpa tanda tangan digital) akan mengalami hal yang
  sama pada peluncuran pertamanya. Kemungkinan besar ya. **Satu pengamatan tambahan
  (langkah 9c)** memperlemah dugaan itu: `koha.exe` yang baru dibongkar dari zip rilis
  diluncurkan pertama kali dalam 52 ms, tidak lambat. Itu satu pengukuran, jadi belum
  menjelaskan apa pun, tetapi lambatnya peluncuran pertama tampaknya bukan sifat semua
  berkas baru; mungkin hanya berkas yang baru saja di-*link* oleh kompilator.

### Rincian di dalam KOHA (varian B, median 30 putaran, dari awal `main`)

| Tahap | Waktu kumulatif (ms) | Tambahan (ms) |
|---|---|---|
| console attached | 0,21 | 0,21 |
| args parsed (`clap`) | 0,35 | 0,15 |
| paths resolved (`directories`) | 3,60 | **3,25** |
| config, state, menu | 3,85 | 0,26 |
| event loop created (`winit`) | 6,22 | 2,36 |
| font loaded | 9,54 | **3,30** |
| layout computed | 9,60 | 0,06 |
| window created | 18,50 | **8,90** |
| surface ready, window fitted | 18,93 | 0,43 |
| window shown (`set_visible`) | 33,72 | **14,79** |
| first frame presented | 34,13 | 0,41 |

Dari sini:

- Waktu di dalam `main` sampai gambar pertama ~34 ms. Waktu OS memuat proses sebelum `main`
  tidak termasuk (lihat pembanding dasar).
- `set_visible` memakan hampir separuhnya (14,8 ms), sebagian besar di dalam Windows
  (menampilkan jendela, DWM). Pembuatan jendela 8,9 ms juga milik OS.
- `paths resolved` 3,25 ms dan `font loaded` 3,30 ms adalah bagian yang berada di tangan kita.
  Penguraian `clap` hanya 0,15 ms.

## Keterbatasan (penting)

1. **"Jendela terlihat" bukan "piksel pertama tergambar".** Jendela Rust ditandai terlihat
   di awal pemanggilan `set_visible` (sekitar 19 ms dari `main`), tetapi gambar pertama
   baru selesai sekitar 15 ms kemudian. Artinya angka Rust di tabel perbandingan
   **mengecualikan ~15 ms jendela kosong**. Waktu AHK sampai tergambar tidak diukur, jadi
   tidak ada angka perbandingan tergambar-dengan-tergambar. Perkiraan kasar: Rust sampai
   tergambar sekitar 49 ms, AHK paling cepat 68 ms, sehingga Rust tetap sekitar 1,4 kali
   lebih cepat atau lebih. **Jangan menyebut "dua kali lebih cepat"**: itu hanya benar
   untuk metrik "terlihat".
2. Satu mesin, dua sesi, antivirus aktif. Beban latar dan mode daya memengaruhi hasil.
3. Resolusi pengukuran ~0,5 ms; angka mencakup biaya `Process.Start` dari .NET (sama untuk
   semua target). Karena itu angka mutlak sedikit lebih besar daripada waktu "sebenarnya",
   tetapi perbandingan antar target adil.
4. Peluncuran dari tombol Lenovo Vantage (dan apa yang dilakukan Vantage sebelum menjalankan
   program) tidak diukur.
5. Waktu `AHK KOHA.exe` mencakup apa pun yang dilakukannya saat start di mesin ini.

## Kandidat optimasi (belum dikerjakan)

Berdasarkan rincian di atas, urut dari yang paling mungkin berguna. Semuanya perlu
disetujui dan diukur ulang sebelum dikerjakan:

1. **Tampilkan jendela setelah gambar pertama siap** (gambar ke buffer saat jendela masih
   tersembunyi, baru `set_visible`). Total waktu tidak berubah, tetapi pengguna tidak lagi
   melihat jendela kosong ~15 ms. Ini memperbaiki *persepsi*, bukan waktu mentah.
2. **Parse font secara paralel dengan pembuatan jendela** (hemat sampai ~3 ms).
3. **`paths resolved`**: `BaseDirs::new` memanggil API folder Windows beberapa kali (3,25 ms).
   Membaca `APPDATA`/`LOCALAPPDATA` langsung hampir tanpa biaya, dengan risiko berbeda dari
   folder yang dialihkan (known folder redirection).
4. Bagian OS (`set_visible`, pembuatan jendela) sulit dikurangi dari sisi aplikasi.

## Mengulang pengukuran

```powershell
# bangun tiga profil di folder terpisah (variabel lingkungan menimpa profil di Cargo.toml)
$env:CARGO_PROFILE_RELEASE_OPT_LEVEL='3'; $env:CARGO_PROFILE_RELEASE_LTO='true'
$env:CARGO_PROFILE_RELEASE_CODEGEN_UNITS='1'; $env:CARGO_PROFILE_RELEASE_PANIC='abort'
$env:CARGO_PROFILE_RELEASE_STRIP='true'
cargo build --release -p koha-app --target-dir target\prof-b

# bandingkan (jangan sentuh mouse/keyboard selama berjalan)
.\tools\bench-startup.ps1 -Rust target\prof-a\release\koha.exe `
  -Also "B=target\prof-b\release\koha.exe","C=target\prof-c\release\koha.exe" `
  -Ahk C:\path\ke\KOHA.exe -Floor -Runs 30 -Warmup 3

# rincian tahap
cargo build --release --features startup-trace -p koha-app --target-dir target\prof-b-trace
.\tools\bench-startup.ps1 -Trace -Rust target\prof-b-trace\release\koha.exe -Runs 30
```

Jalankan dua kali dan bandingkan sesinya; satu sesi saja bisa menyesatkan.
