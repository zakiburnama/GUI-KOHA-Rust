# tools

Alat bantu pengembangan (bukan bagian dari aplikasi).

## bench-startup.ps1

Mengukur waktu start KOHA. Waktu start adalah fitur utama, jadi perubahan yang
menyentuh jalur start harus diukur sebelum dan sesudahnya. Hanya Windows.

```powershell
# bandingkan dengan KOHA versi AutoHotkey, termasuk "harga minimum" memulai proses
cargo build --release
.\tools\bench-startup.ps1 -Ahk C:\path\ke\KOHA.exe -Floor

# rincian tahap di dalam KOHA: bangun dengan fitur jejak, lalu ukur
cargo build --release --features startup-trace -p koha-app --target-dir target\trace
.\tools\bench-startup.ps1 -Trace -Rust target\trace\release\koha.exe -Runs 20
```

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
