# tools

Alat bantu pengembangan dan rilis (bukan bagian dari aplikasi). Semuanya skrip PowerShell
untuk Windows.

| Skrip | Fungsi |
|---|---|
| [bench-startup.ps1](bench-startup.ps1) | Mengukur waktu start KOHA |
| [gen-third-party.ps1](gen-third-party.ps1) | Menghasilkan `THIRD_PARTY_LICENSES.md` |
| [package-release.ps1](package-release.ps1) | Mengemas rilis Windows (zip dan checksum) |
| [release-notes.ps1](release-notes.ps1) | Menyusun catatan rilis dari `CHANGELOG.md` |
| [install-shortcut.ps1](install-shortcut.ps1) | Memasang `koha.exe` dan pintasan Start Menu (untuk Lenovo Vantage) |

## install-shortcut.ps1

Memasang KOHA untuk pengguna yang menjalankannya, supaya program lain (terutama Lenovo
Vantage, yang hanya menampilkan program dari Start Menu) bisa memilihnya.

```powershell
.\tools\install-shortcut.ps1                    # salin koha.exe + buat pintasan "KOHA (Rust)"
.\tools\install-shortcut.ps1 -WhatIf            # hanya menampilkan apa yang akan dilakukan
.\tools\install-shortcut.ps1 -Uninstall         # hapus pintasan dan koha.exe terpasang
.\tools\install-shortcut.ps1 -Name 'KOHA'       # nama lain (menolak menimpa pintasan orang lain)
```

- `koha.exe` disalin ke `%LOCALAPPDATA%\Programs\koha\` dan salinannya diverifikasi identik;
  pintasan dibuat di folder Programs milik pengguna di Start Menu lalu dibaca kembali untuk
  memastikan targetnya benar. Tidak butuh hak admin.
- Sumbernya dicari di folder paket (`koha.exe` di sebelah `tools\`) lalu di
  `target\release\koha.exe`; bisa ditentukan dengan `-Source`. **Build debug ditolak**
  (subsistem konsol: membuka jendela konsol dan lebih lambat).
- Menolak menimpa pintasan bernama sama yang menunjuk ke program lain (misalnya `KOHA.lnk`
  milik versi AutoHotkey) kecuali dengan `-Force`, dan menolak memperbarui `koha.exe` yang
  sedang berjalan.
- `-Uninstall` hanya menghapus pintasan yang menunjuk ke `koha.exe` terpasang ini. Config dan
  state tidak pernah disentuh.

## bench-startup.ps1

Mengukur waktu start KOHA. Waktu start adalah fitur utama, jadi perubahan yang
menyentuh jalur start harus diukur sebelum dan sesudahnya.

```powershell
# bandingkan dengan KOHA versi AutoHotkey, termasuk "harga minimum" memulai proses
cargo build --release
.\tools\bench-startup.ps1 -Ahk C:\path\ke\KOHA.exe -Floor

# rincian tahap di dalam KOHA: bangun dengan fitur jejak, lalu ukur
cargo build --release --features startup-trace -p koha-app --target-dir target\trace
.\tools\bench-startup.ps1 -Trace -Rust target\trace\release\koha.exe -Runs 20
```

Opsi `-Also "Nama=jalur"` menambahkan varian Rust lain ke perbandingan (mis. profil rilis
berbeda); beberapa varian dipisah koma. Hasil pengukuran yang sudah ada ada di
[docs/performance.md](../docs/performance.md).

Hal yang perlu diketahui sebelum mempercayai angkanya:

- Yang diukur adalah **jendela KOHA terlihat**, bukan piksel pertama tergambar dan bukan
  siap menerima tombol. Mode `-Trace` melengkapi sisi Rust dengan tahap "first frame
  presented" (tidak tersedia untuk AHK).
- Target dijalankan **bergantian** supaya gangguan sistem tidak berat sebelah; beberapa
  putaran pemanasan dibuang; peluncuran pertama dilaporkan terpisah.
- Angka mencakup biaya `Process.Start` dari .NET (sama untuk semua target) dan resolusi
  sekitar 0,5 ms.
- Jangan sentuh mouse atau keyboard saat berjalan. Jendela KOHA muncul dan hilang berulang
  di layar.
- Hasilnya milik satu mesin pada satu sesi (antivirus, daya, dan beban latar ikut
  berpengaruh). Jangan dikutip sebagai klaim umum.
- Skrip hanya menghentikan proses yang ia mulai sendiri, dan menjalankan Rust dengan
  `--state` ke berkas sementara sehingga state asli tidak tersentuh.

Fitur Cargo `startup-trace` mencetak waktu tiap tahap ke stderr (`[trace] ... ms  tahap`).
Saat fitur mati (bawaan), pemanggilannya dihapus kompiler dan tidak ada biaya.

## gen-third-party.ps1

Menghasilkan `THIRD_PARTY_LICENSES.md` di akar repositori: lisensi semua dependensi Rust
yang masuk binary Windows, dan tiga font yang tertanam. Dijalankan ulang setiap kali
dependensi atau font berubah.

```powershell
.\tools\gen-third-party.ps1           # menulis ulang berkasnya
.\tools\gen-third-party.ps1 -Check    # hanya memeriksa; exit 1 bila tidak mutakhir
```

Paket yang tidak membawa berkas lisensi di crate terbitannya tidak disembunyikan: mereka
dicantumkan di bagian tersendiri dan dilaporkan sebagai peringatan. Alat ini membantu memenuhi
kewajiban menyertakan pemberitahuan lisensi; ia bukan nasihat hukum.

## package-release.ps1

Mengemas rilis Windows ke `dist\` (diabaikan git):

```text
koha-<versi>-<target>.zip
koha-<versi>-<target>.zip.sha256
```

Zip berisi `koha.exe`, `README.md`, `CHANGELOG.md`, `CONTRIBUTING.md`,
`THIRD_PARTY_LICENSES.md`, `deny.toml`, `docs\`, `tools\`, dan `BUILD_INFO.txt` (versi,
commit, kompilator, hash `koha.exe`, dan apakah dibangun dari klon bersih).

```powershell
# rilis sungguhan: klon bersih dari commit atau tag, hanya yang sudah di-commit yang masuk
.\tools\package-release.ps1 -FromClean -Ref v0.1.0

# latihan sebelum commit: menyertakan perubahan yang belum di-commit (diberi akhiran -dryrun)
.\tools\package-release.ps1 -FromClean -IncludeUncommitted
```

Skrip berhenti dengan error bila salah satu gerbang gagal:

- berkas wajib hilang dari sumber (diperiksa sebelum build, jadi tidak memakan waktu kompilasi);
- `THIRD_PARTY_LICENSES.md` tidak mutakhir;
- versi dari `koha.exe --version` tidak sama dengan versi di `Cargo.toml`;
- **`koha.exe` memuat nama pengguna atau jalur lokal pembuatnya.** Binary Rust biasanya
  menanam jalur sumber dependensi (untuk pesan panic), misalnya
  `C:\Users\<nama>\.cargo\registry\...`. Build rilis dibuat dengan `--remap-path-prefix`
  supaya itu tidak ikut terbagikan. (`trim-paths` di `Cargo.toml` belum stabil di Cargo
  1.99, sehingga pemetaan dilakukan oleh skrip ini, bukan oleh `cargo build --release`
  biasa. Binary yang kamu bangun sendiri untuk dirimu tidak dipetakan, dan tidak perlu.)
- ada tautan relatif yang rusak di dokumentasi di dalam paket;
- hasil zip, setelah dibongkar ke folder sementara, berbeda dari aslinya, memakai nama
  entri dengan `\`, atau `koha.exe`-nya tidak melaporkan versi yang benar.

Hal yang perlu diketahui:

- Paket **tidak ditandatangani** secara digital, sehingga Windows SmartScreen bisa
  memperingatkan.
- Build **tidak byte-reproducible**: dua build bersih dari commit yang sama menghasilkan
  `koha.exe` berukuran sama tetapi berhash berbeda. Checksum menjamin berkas yang dibagikan
  tidak berubah, bukan bahwa orang lain bisa membangun berkas identik.
- Paket dari working tree kotor atau dengan `-IncludeUncommitted` bernama `...-dryrun` dan
  `BUILD_INFO.txt`-nya menyatakan itu bukan rilis.

Langkah lengkap membuat rilis ada di [docs/releasing.md](../docs/releasing.md).

## release-notes.ps1

Mengambil bagian `## [<versi>]` dari `CHANGELOG.md` dan menyusunnya menjadi catatan rilis
GitHub: pembuka singkat, SHA-256 paket (dibaca dari berkas `.sha256` di `dist\`, atau dari
`-Zip`), dan tautan relatif yang diubah menjadi tautan absolut ke tag rilis (catatan rilis
GitHub tidak punya konteks repositori). Skrip ini tidak menerbitkan apa pun.

```powershell
.\tools\release-notes.ps1 -Version 0.1.0 -OutFile dist\RELEASE_NOTES.md
```

Ia memperingatkan bila `CHANGELOG.md` belum menanggali versi itu, dan gagal bila versinya
tidak ada di `CHANGELOG.md`.
