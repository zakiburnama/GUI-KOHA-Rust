# Referensi konfigurasi

KOHA membaca satu berkas config (`config.toml`) setiap kali dijalankan. Tidak ada
muat-ulang otomatis: ubah berkasnya, lalu panggil KOHA lagi. Pilihan yang dibuat dari
dalam menu (tema aktif, item yang di-OFF) disimpan terpisah di berkas **state** yang
ditulis KOHA sendiri, supaya config-mu tidak pernah ditimpa aplikasi.

> Semua blok `toml` di dokumen ini diuji otomatis (`cargo test`): masing-masing harus
> berupa config yang valid, dan blok `toml state` harus berupa state yang valid. Potongan
> yang bukan config lengkap ditulis sebagai `text`.

## Lokasi berkas

| Berkas | Windows (bawaan) |
|---|---|
| Config | `%APPDATA%\koha\config.toml` |
| State | `%LOCALAPPDATA%\koha\state.toml` |

Urutan prioritas lokasi: opsi baris perintah, lalu variabel lingkungan, lalu bawaan.

| Yang diubah | Opsi | Variabel lingkungan |
|---|---|---|
| Config | `--config <jalur>` | `KOHA_CONFIG` |
| State | `--state <jalur>` | `KOHA_STATE` |

Bila config ditentukan lewat opsi atau variabel dan state tidak, state diletakkan **di
sebelah config itu** (`state.toml`), sehingga mencoba config lain tidak menimpa state
harianmu.

Dua perintah untuk melihat dan membuat berkasnya:

```text
koha --print-config-path      lokasi config yang dipakai
koha --init                   tulis config contoh bila belum ada (tidak pernah menimpa)
```

Bila berkas config **tidak ada**, KOHA memakai menu contoh bawaan tanpa membuat berkas.
Bila berkasnya **ada tetapi rusak**, KOHA tidak diam-diam jatuh ke menu bawaan: ia menampilkan
error (di terminal bila dijalankan dari terminal, atau sebagai dialog bila dipanggil dari
tombol atau pintasan) dan berhenti. Berkas boleh berkode UTF-8 (dengan atau tanpa BOM) dan
berakhir baris LF maupun CRLF.

## Struktur

```toml
version = 1

[[menu]]
id = "github"
label = "GitHub"
type = "url"
url = "https://github.com"
```

- `version` wajib dan harus `1`. Versi lain ditolak dengan pesan yang jelas.
- `[[menu]]` adalah satu baris menu. Urutannya di berkas adalah urutannya di layar.
- Menu tidak boleh kosong.
- Field yang tidak dikenal ditolak (salah ketik langsung ketahuan), begitu juga field yang
  tidak berlaku untuk `type`-nya (misalnya `url` pada item `exec`).

## Item menu

Setiap item punya tiga field umum:

| Field | Aturan |
|---|---|
| `id` | Wajib. Hanya huruf kecil `a-z`, angka, `_`, dan `-`. **Unik di seluruh config**, termasuk di dalam submenu. Dipakai untuk mengingat status ON/OFF, jadi tidak berubah walau `label` diganti. |
| `label` | Wajib, tidak boleh kosong. Teks yang tampil di menu. |
| `type` | Wajib: `exec`, `url`, `builtin`, atau `submenu`. |

### `exec`: menjalankan program

| Field | Aturan |
|---|---|
| `command` | Wajib. Nama atau jalur program. |
| `args` | Opsional. Daftar argumen; tiap elemen adalah **satu** argumen (KOHA mengutipnya dengan benar, jadi spasi di dalamnya aman). |
| `admin` | Opsional, bawaan `false`. `true` meminta elevasi (muncul prompt UAC). |

```toml
version = 1

[[menu]]
id = "editor"
label = "Editor"
type = "exec"
command = '%LOCALAPPDATA%\Programs\Microsoft VS Code\Code.exe'
args = ['C:\proyek\catatan baru.md']

[[menu]]
id = "terminal-admin"
label = "Terminal (Admin)"
type = "exec"
command = "wt.exe"
admin = true
```

Hal yang perlu diketahui:

- **Pakai tanda kutip tunggal untuk jalur Windows** (`'C:\...'`). Di TOML, kutip tunggal
  berarti teks apa adanya; kutip ganda memperlakukan `\` sebagai awal escape, sehingga
  `"C:\proyek"` rusak.
- `%NAMA%` di `command` diperluas (misalnya `%LOCALAPPDATA%`). Di `args` tidak.
- Program dijalankan lewat shell Windows, jadi nama yang terdaftar di `PATH` atau *App Paths*,
  pintasan `.lnk`, dan berkas yang punya aplikasi asosiasi ikut bekerja. Program yang tidak
  ada menghasilkan error yang menyebutnya.
- Program dijalankan **setelah** jendela KOHA ditutup, sehingga fokus sudah kembali ke
  aplikasi sebelumnya. KOHA langsung keluar sesudahnya.
- Dengan `admin = true`, menolak prompt UAC bukan error: KOHA keluar tanpa pesan.

### `url`: membuka tautan

| Field | Aturan |
|---|---|
| `url` | Wajib. Harus diawali skema (`https:`, `mailto:`, `ms-settings:`, `obsidian://`, ...). |

```toml
version = 1

[[menu]]
id = "pengaturan-layar"
label = "Pengaturan layar"
type = "url"
url = "ms-settings:display"

[[menu]]
id = "email"
label = "Tulis email"
type = "url"
url = "mailto:seseorang@example.com"
```

Dibuka dengan aplikasi bawaan untuk skema itu. Teks yang **bukan** URL (misalnya `calc.exe`
atau `C:\Windows\...`) ditolak, karena Windows akan menjalankannya sebagai program. Untuk
menjalankan program, pakai `exec`.

### `builtin`: aksi bawaan

| Field | Aturan |
|---|---|
| `name` | Wajib. Salah satu dari tabel di bawah. |

| `name` | Fungsi |
|---|---|
| `lock` | Mengunci sesi Windows. |
| `sleep` | Menidurkan komputer. **Tidak bekerja di PC yang hanya mendukung Modern Standby tanpa hibernasi** (periksa dengan `powercfg /a`). Karena itu tidak ada di config contoh. |
| `close_all_windows` | Meminta semua jendela aplikasi yang terlihat menutup diri (`WM_CLOSE`). Jendela shell Windows, dialog, dan KOHA sendiri dikecualikan. Aplikasi yang punya perubahan belum tersimpan umumnya menampilkan dialog dulu. **Jendela tempat kamu mengetik atau membaca ini juga akan diminta menutup.** |
| `theme_picker` | Membuka pemilih tema di dalam menu. Memilih tema langsung berlaku dan menu tetap terbuka. |

```toml
version = 1

[[menu]]
id = "tema"
label = "Color Scheme"
type = "builtin"
name = "theme_picker"

[[menu]]
id = "kunci"
label = "Lock"
type = "builtin"
name = "lock"
```

### `submenu`: menu bertingkat

| Field | Aturan |
|---|---|
| `items` | Wajib, minimal satu item. Boleh bersarang lebih dalam. |

```toml
version = 1

[[menu]]
id = "alat"
label = "Alat"
type = "submenu"

  [[menu.items]]
  id = "kunci-layar"
  label = "Lock"
  type = "builtin"
  name = "lock"

  [[menu.items]]
  id = "lainnya"
  label = "Lainnya"
  type = "submenu"

    [[menu.items.items]]
    id = "tutup-semua"
    label = "Close All Windows"
    type = "builtin"
    name = "close_all_windows"
```

`Esc` di dalam submenu kembali satu tingkat; `Esc` di menu utama menutup KOHA.

## Baris "Menu Settings"

Di akhir menu utama, KOHA selalu menambahkan satu baris **Menu Settings**. Baris ini tidak
ada di config dan tidak bisa di-OFF. Isinya daftar semua item menu utama dengan status
`(ON)` atau `(OFF)`; `Enter` membaliknya dan menu tetap terbuka. Item yang di-OFF
disembunyikan dari menu utama setelah kamu kembali dengan `Esc`.

- Hanya item di **menu utama** yang bisa di-OFF, bukan isi submenu.
- Statusnya disimpan di state berdasarkan `id`. Mengganti `label` tidak mengubahnya;
  mengganti `id` membuatnya dianggap item baru.
- Baris ini tidak bisa disembunyikan agar selalu ada jalan kembali bila semua item pernah di-OFF.

## Tema

Tema bawaan (urutannya sama di pemilih tema):

| Nama | Font |
|---|---|
| `game_boy` | `press-start-2p` |
| `amber` | `vt323` |
| `green_term` | `vt323` |
| `catppuccin-mocha` | `ibm-plex-mono` |
| `gruvbox` | `ibm-plex-mono` |
| `vague` | `ibm-plex-mono` |
| `tokyonight` | `ibm-plex-mono` |

Sebelum kamu memilih tema apa pun, yang dipakai adalah `amber`. Setiap tema membawa
fontnya sendiri, jadi mengganti tema juga mengganti font. Font yang tersedia: `press-start-2p`, `vt323`, dan `ibm-plex-mono`
(bawaan untuk tema buatanmu). Id font yang tidak dikenal jatuh ke `ibm-plex-mono`. KOHA
tidak memuat font dari berkas; hanya tiga font ini yang ada.

Menambah tema sendiri dengan `[[themes]]`:

| Field | Arti |
|---|---|
| `name` | Wajib, unik di antara `[[themes]]`. |
| `bg`, `fg` | Latar dan teks baris biasa. |
| `sel_bg`, `sel_fg` | Latar dan teks baris terpilih. |
| `bezel` | Warna bingkai jendela di sekeliling daftar. |
| `font` | Opsional. |

Warna ditulis sebagai `"#RRGGBB"` atau `"RRGGBB"`, huruf besar atau kecil.

```toml
version = 1

[[menu]]
id = "tema"
label = "Color Scheme"
type = "builtin"
name = "theme_picker"

[[themes]]
name = "malam"
bg = "#101018"
fg = "#D0D0E0"
sel_bg = "#D0D0E0"
sel_fg = "#101018"
bezel = "#000000"
font = "ibm-plex-mono"
```

Memakai nama tema bawaan **menimpanya** di posisinya semula (misalnya untuk mengubah warna
`gruvbox`). Nama baru ditambahkan di akhir daftar.

```toml
version = 1

[[menu]]
id = "tema"
label = "Color Scheme"
type = "builtin"
name = "theme_picker"

[[themes]]
name = "gruvbox"
bg = "#1D2021"
fg = "#EBDBB2"
sel_bg = "#D79921"
sel_fg = "#1D2021"
bezel = "#000000"
```

## State

KOHA menulis `state.toml` sendiri setiap kali tema diganti atau item di-ON/OFF. Kamu tidak
perlu mengeditnya, tetapi formatnya sederhana:

```toml state
version = 1
theme = "gruvbox"
hidden = ["kunci", "tutup-semua"]
```

- `theme`: tema aktif. Bila tidak ada atau namanya tidak dikenal (misalnya tema buatan yang
  sudah dihapus dari config), dipakai `amber`.
- `hidden`: `id` item menu utama yang di-OFF. `id` yang sudah tidak ada di config dibiarkan
  (dan tidak berpengaruh), supaya menghapus item sementara lalu mengembalikannya tidak
  menghilangkan statusnya.
- Menghapus berkasnya mengembalikan tema bawaan dan menampilkan semua item.
- Bila berkasnya rusak, KOHA memakai state bawaan dan menulis peringatan, bukan berhenti.

## Pesan error

Kesalahan sintaks atau tipe menunjuk baris dan kolomnya:

```text
error: invalid type: integer `5`, expected a string
 --> C:\Users\kamu\AppData\Roaming\koha\config.toml:4:9
  |
4 | label = 5
  |         ^
```

Kesalahan aturan (id ganda, label kosong, field wajib hilang, dan sejenisnya) menunjuk item
lewat jalurnya, karena TOML tidak menyimpan posisi baris setelah dibaca:

```text
error: menu[1].items[0]: id "kunci" dipakai lebih dari sekali
 --> C:\Users\kamu\AppData\Roaming\koha\config.toml
```

`menu[1].items[0]` berarti item pertama dalam `items` milik item menu kedua.

## Contoh lengkap

```toml
version = 1

[[menu]]
id = "tema"
label = "Color Scheme"
type = "builtin"
name = "theme_picker"

[[menu]]
id = "tautan"
label = "Tautan"
type = "submenu"

  [[menu.items]]
  id = "github"
  label = "GitHub"
  type = "url"
  url = "https://github.com"

[[menu]]
id = "kunci"
label = "Lock"
type = "builtin"
name = "lock"

[[menu]]
id = "tutup-semua"
label = "Close All Windows"
type = "builtin"
name = "close_all_windows"

[[menu]]
id = "terminal-admin"
label = "Terminal (Admin)"
type = "exec"
command = "wt.exe"
admin = true
```
