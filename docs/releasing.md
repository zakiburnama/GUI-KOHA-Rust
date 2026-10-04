# Merilis KOHA

Dokumen ini adalah daftar periksa untuk membuat rilis. Alat pendukungnya dijelaskan di
[tools/README.md](../tools/README.md). Langkah yang menyentuh GitHub (push, tag di remote,
menerbitkan Release) dilakukan oleh pemilik repositori; tidak ada alat di repositori ini
yang melakukannya otomatis.

Contoh di bawah memakai versi `0.1.0`; ganti sesuai versi yang dirilis.

## 0. Prasyarat: keputusan dan uji manual

Hal-hal ini tidak bisa dibuktikan oleh skrip. Selesaikan sebelum memberi tag.

- [ ] **Lisensi sudah diputuskan.** Saat ini KOHA tidak punya lisensi (semua hak dilindungi
  penulis), dan [README](../README.md#lisensi) menyatakannya. Merilis tanpa lisensi sah, tetapi
  orang lain belum berhak memakai ulang kodenya. Bila kamu memilih lisensi, tambahkan berkas
  `LICENSE` di akar repositori (`package-release.ps1` otomatis menyertakannya di paket), isi
  bagian lisensi README, dan perbarui catatan di `CHANGELOG.md`.
- [ ] **Keluaran dari terminal interaktif.** Di PowerShell atau Windows Terminal, dengan
  `koha.exe` hasil `cargo build --release`:

  ```powershell
  .\target\release\koha.exe --help
  .\target\release\koha.exe --print-config-path
  .\target\release\koha.exe --init --print-config-path
  ```

  Ketiganya harus mencetak sesuatu. Yang ketiga harus mencetak galat "cannot be used with"
  ke terminal (bukan dialog). Prompt boleh muncul lebih dulu daripada keluarannya.
- [ ] **Dialog error tanpa konsol.** Tekan `Win+R` dan jalankan (ganti jalurnya):

  ```text
  C:\jalur\ke\koha.exe --config C:\tidak-ada.toml
  ```

  Harus muncul dialog "config tidak ditemukan".
- [ ] **Dari tombol Lenovo Vantage** (atau pemicu lain yang kamu pakai): menu muncul dan
  menerima tombol, dan memilih satu aksi menjalankannya. Perhatikan apakah fokus jendela
  didapat saat dipanggil dari tombol (aturan Windows tentang proses yang baru dijalankan
  bisa menolaknya).
- [ ] **CI hijau pada commit yang akan dirilis**: job `check (windows-latest)` dan `deny`
  wajib hijau di halaman Actions repositori. Job Ubuntu dan macOS bersifat eksperimental.

## 1. Pemeriksaan otomatis lokal

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

Semuanya harus lolos. Ini sama dengan yang dijalankan CI.

## 2. Siapkan commit rilis

1. Pastikan versi di `Cargo.toml` (`[workspace.package]`) benar. Bila diubah, jalankan
   `cargo build --locked` dulu agar `Cargo.lock` mengikuti.
2. Di `CHANGELOG.md`, ubah judul bagian rilis dari `## [0.1.0] - belum dirilis` menjadi
   bertanggal, misalnya `## [0.1.0] - 2026-10-04`. Pindahkan isi `[Unreleased]` ke bagian
   baru bila ada.
3. Commit, misalnya `release: 0.1.0`.

## 3. Beri tag (lokal dulu)

```powershell
git tag -a v0.1.0 -m "KOHA 0.1.0"
```

Tag dibuat secara lokal dulu supaya bisa dibatalkan sebelum ada yang melihatnya.

## 4. Kemas dari tag

```powershell
.\tools\package-release.ps1 -FromClean -Ref v0.1.0
```

Ini meng-klon tag ke folder sementara, membangun dari sana (hanya yang sudah di-commit yang
masuk), dan menolak mengemas bila ada gerbang yang gagal (lisensi tidak mutakhir, versi tidak
cocok, `koha.exe` memuat jalur atau nama pengguna lokal, tautan dokumentasi rusak). Hasilnya:

```text
dist\koha-0.1.0-x86_64-pc-windows-msvc.zip
dist\koha-0.1.0-x86_64-pc-windows-msvc.zip.sha256
```

Pastikan namanya **tanpa** `-dryrun`, dan bahwa `BUILD_INFO.txt` di dalamnya menyatakan
`UNCOMMITTED: tidak` dan `dari klon bersih: ya`.

## 5. Uji paket yang akan dirilis

Bongkar zip ke folder sementara dan coba dari sana, bukan dari folder proyek:

```powershell
Expand-Archive .\dist\koha-0.1.0-x86_64-pc-windows-msvc.zip -DestinationPath $env:TEMP\koha-uji
& "$env:TEMP\koha-uji\koha-0.1.0-x86_64-pc-windows-msvc\koha.exe" --version | Out-String
Get-FileHash .\dist\koha-0.1.0-x86_64-pc-windows-msvc.zip -Algorithm SHA256
Get-Content .\dist\koha-0.1.0-x86_64-pc-windows-msvc.zip.sha256
```

Hash dari `Get-FileHash` harus sama dengan isi berkas `.sha256`. Jalankan menunya sekali dari
sana dengan `--state` ke berkas sementara agar state aslimu tidak berubah.

## 6. Dorong

Hanya setelah paketnya baik:

```powershell
git push origin master
git push origin v0.1.0
```

Lihat halaman Actions: CI untuk commit rilis harus hijau (lihat prasyarat).

## 7. Catatan rilis

```powershell
.\tools\release-notes.ps1 -Version 0.1.0 -OutFile dist\RELEASE_NOTES.md
```

Skrip ini mengambil bagian CHANGELOG, mengubah tautan relatif menjadi tautan absolut ke tag,
dan mencantumkan SHA-256 paket. Ia memperingatkan bila CHANGELOG belum bertanggal.

## 8. Terbitkan di GitHub

Lewat antarmuka web: repositori → **Releases** → **Draft a new release** → pilih tag
`v0.1.0` yang sudah ada → judul `KOHA 0.1.0` → tempel isi `dist\RELEASE_NOTES.md` → lampirkan
**kedua** berkas `koha-0.1.0-x86_64-pc-windows-msvc.zip` dan `...zip.sha256` → untuk versi `0.x`
sebaiknya centang **Set as a pre-release** → **Publish release**.

Atau dengan GitHub CLI (`gh`; belum terpasang di mesin pengembangan, dan harus sudah login):

```powershell
gh release create v0.1.0 `
  dist\koha-0.1.0-x86_64-pc-windows-msvc.zip `
  dist\koha-0.1.0-x86_64-pc-windows-msvc.zip.sha256 `
  --title "KOHA 0.1.0" --notes-file dist\RELEASE_NOTES.md --prerelease
```

## 9. Setelah terbit

- Unduh zip dari halaman Release di komputer yang bersih (atau folder lain), cocokkan hashnya
  dengan yang tertulis, dan jalankan. Ini satu-satunya cara memastikan apa yang diterima
  pengguna sama dengan yang kamu uji.
- Tautan `[0.1.0]` di dasar `CHANGELOG.md` kini menunjuk halaman Release yang ada.
- Siapkan bagian `[Unreleased]` yang kosong untuk perubahan berikutnya.

## Bila ada yang salah

- **Sebelum push**: hapus tag lokal dengan `git tag -d v0.1.0`, perbaiki, lalu ulangi dari
  langkah 2 atau 3. Karena belum ada yang melihatnya, ini aman.
- **Setelah push tetapi sebelum Release diterbitkan**: hapus tag di remote dengan
  `git push origin :refs/tags/v0.1.0` hanya bila yakin belum ada yang mengambilnya.
- **Setelah Release diterbitkan**: jangan menulis ulang tag. Terbitkan perbaikan sebagai versi
  baru (`0.1.1`), dan bila perlu tandai Release yang bermasalah di halamannya.

## Yang tidak dijamin

- **Paket tidak ditandatangani** secara digital, sehingga Windows SmartScreen bisa
  memperingatkan, terutama untuk berkas yang diunduh dari internet. Untuk zip yang diunduh,
  membuka *Properties* lalu **Unblock** sebelum membongkar (atau `Unblock-File`) bisa
  menghindari peringatan itu pada berkas di dalamnya.
- **Build tidak byte-reproducible**: orang lain yang membangun dari tag yang sama akan
  mendapat `koha.exe` berukuran sama tetapi berhash berbeda. Checksum menjamin berkas yang
  diunduh tidak berubah, bukan bahwa ia bisa dibangun ulang secara identik.
- **Peluncuran pertama bisa lambat** pada berkas baru (lihat
  [performance.md](performance.md)).

## Pelajaran dari CI

CI Windows mengambil kode dengan akhir baris **CRLF** (`core.autocrlf`), sedangkan di mesin
pengembangan berkasnya LF. Satu test yang mengganti `\n` dengan `\r\n` pada berkas yang sudah
CRLF menghasilkan `\r\r\n` dan gagal **hanya di CI**. Untuk mereproduksi kegagalan sejenis
secara lokal:

```powershell
git -c core.autocrlf=true clone --no-hardlinks . $env:TEMP\koha-crlf
cd $env:TEMP\koha-crlf
cargo test --workspace --no-fail-fast
```

`--no-fail-fast` penting: tanpanya Cargo berhenti di target test pertama yang gagal dan
menyembunyikan kegagalan lain.
