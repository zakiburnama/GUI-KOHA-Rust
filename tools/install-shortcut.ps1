<#
.SYNOPSIS
  Memasang KOHA untuk pengguna ini: menyalin koha.exe ke folder tetap dan membuat pintasan
  di Start Menu, supaya program lain (misalnya Lenovo Vantage) bisa memilihnya.

.DESCRIPTION
  Lenovo Vantage hanya menampilkan program yang terdaftar di Start Menu, jadi dibutuhkan
  pintasan (.lnk) di folder Programs milik pengguna. Skrip ini:

    1. memeriksa koha.exe sumber (harus build rilis tanpa konsol, bukan build debug);
    2. menyalinnya ke %LOCALAPPDATA%\Programs\koha\koha.exe dan memastikan salinannya
       identik (supaya `cargo clean` atau membangun ulang tidak merusak pintasan);
    3. membuat pintasan "%APPDATA%\Microsoft\Windows\Start Menu\Programs\<Name>.lnk";
    4. membaca pintasannya kembali untuk memastikan targetnya benar.

  Aman dijalankan berulang kali: menjalankan lagi memperbarui koha.exe dan pintasannya.
  Skrip menolak menimpa pintasan lain dengan nama yang sama (misalnya "KOHA" milik versi
  AutoHotkey) kecuali kamu memberi -Force. Config dan state KOHA tidak pernah disentuh.

  Hanya mengubah folder milik pengguna ini: tidak butuh hak admin.

.PARAMETER Name
  Nama pintasan (dan yang muncul di Start Menu). Bawaan: "KOHA (Rust)".

.PARAMETER Source
  Jalur koha.exe yang disalin. Bila tidak diberikan, dicari di folder paket (koha.exe di
  sebelah folder tools) lalu di target\release\koha.exe.

.PARAMETER InstallDir
  Folder tujuan salinan. Bawaan: %LOCALAPPDATA%\Programs\koha.

.PARAMETER ShortcutDir
  Folder pintasan. Bawaan: folder Programs di Start Menu pengguna. (Terutama untuk pengujian.)

.PARAMETER Uninstall
  Menghapus pintasan dan koha.exe yang terpasang. Config dan state dibiarkan.

.PARAMETER Force
  Menimpa/menghapus pintasan bernama sama walau targetnya bukan koha.exe terpasang ini.

.EXAMPLE
  .\tools\install-shortcut.ps1

.EXAMPLE
  .\tools\install-shortcut.ps1 -WhatIf        # hanya menampilkan apa yang akan dilakukan

.EXAMPLE
  .\tools\install-shortcut.ps1 -Uninstall
#>
[CmdletBinding(SupportsShouldProcess)]
param(
  [string]$Name = 'KOHA (Rust)',
  [string]$Source,
  [string]$InstallDir,
  [string]$ShortcutDir,
  [switch]$Uninstall,
  [switch]$Force
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent (Split-Path -Parent $PSCommandPath)

if (-not $InstallDir) { $InstallDir = Join-Path $env:LOCALAPPDATA 'Programs\koha' }
if (-not $ShortcutDir) { $ShortcutDir = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs' }

if ($Name.IndexOfAny([IO.Path]::GetInvalidFileNameChars()) -ge 0 -or -not $Name.Trim()) {
  throw "Nama pintasan tidak valid: '$Name'"
}
$destExe = Join-Path $InstallDir 'koha.exe'
$lnk = Join-Path $ShortcutDir "$Name.lnk"

function Get-FullPath([string]$Path) { [IO.Path]::GetFullPath($Path) }
function Test-SamePath([string]$A, [string]$B) {
  if (-not $A -or -not $B) { return $false }
  [string]::Equals((Get-FullPath $A), (Get-FullPath $B), [StringComparison]::OrdinalIgnoreCase)
}
function Get-ShortcutTarget([string]$Path) {
  (New-Object -ComObject WScript.Shell).CreateShortcut($Path).TargetPath
}
# Nilai Subsystem di header PE: 2 = GUI (tanpa konsol), 3 = konsol.
function Get-Subsystem([string]$Path) {
  $bytes = [IO.File]::ReadAllBytes($Path)
  $pe = [BitConverter]::ToInt32($bytes, 0x3C)
  [BitConverter]::ToUInt16($bytes, $pe + 24 + 68)
}
function Get-Sha256([string]$Path) { (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash }
function Test-KohaRunningFrom([string]$ExePath) {
  foreach ($process in Get-Process -Name koha -ErrorAction SilentlyContinue) {
    try { if (Test-SamePath $process.Path $ExePath) { return $true } } catch { }   # Path bisa tak terbaca
  }
  $false
}

# --------------------------------------------------------------------- hapus
if ($Uninstall) {
  if (Test-KohaRunningFrom $destExe) { throw "KOHA sedang berjalan dari $destExe; tutup dulu." }

  if (Test-Path -LiteralPath $lnk) {
    $target = Get-ShortcutTarget $lnk
    if ((Test-SamePath $target $destExe) -or $Force) {
      if ($PSCmdlet.ShouldProcess($lnk, 'Hapus pintasan')) { [IO.File]::Delete($lnk); Write-Host "Pintasan dihapus: $lnk" }
    } else {
      Write-Warning "Pintasan '$lnk' TIDAK dihapus: ia menunjuk ke '$target', bukan ke koha.exe yang terpasang di sini. Pakai -Force bila memang itu yang kamu mau."
    }
  } else {
    Write-Host "Tidak ada pintasan: $lnk"
  }

  if (Test-Path -LiteralPath $destExe) {
    if ($PSCmdlet.ShouldProcess($destExe, 'Hapus koha.exe')) {
      [IO.File]::Delete($destExe); Write-Host "koha.exe dihapus: $destExe"
      if ((Test-Path -LiteralPath $InstallDir) -and -not (Get-ChildItem -LiteralPath $InstallDir -Force)) { [IO.Directory]::Delete($InstallDir); Write-Host "Folder kosong dihapus: $InstallDir" }
    }
  } else {
    Write-Host "Tidak ada koha.exe terpasang di: $InstallDir"
  }
  Write-Host 'Config dan state KOHA tidak disentuh.'
  return
}

# -------------------------------------------------------------------- sumber
if (-not $Source) {
  $candidates = @((Join-Path $root 'koha.exe'), (Join-Path $root 'target\release\koha.exe'))
  $Source = $candidates | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
  if (-not $Source) { throw ("koha.exe tidak ditemukan. Dicari di:`n  " + ($candidates -join "`n  ") + "`nBangun dulu dengan 'cargo build --release', atau berikan -Source.") }
}
if (-not (Test-Path -LiteralPath $Source)) { throw "Sumber tidak ada: $Source" }
$Source = Get-FullPath $Source

if ((Get-Subsystem $Source) -ne 2) {
  throw "$Source adalah build KONSOL (kemungkinan build debug dari 'cargo run'). Itu membuka jendela konsol dan lebih lambat. Pakai build rilis: 'cargo build --release'."
}
$versionLine = (& $Source --version | Out-String).Trim()
if ($versionLine -notmatch '^koha \d+\.\d+\.\d+') { throw "$Source tidak melaporkan versi KOHA yang wajar: '$versionLine'" }

# ----------------------------------------------------- pemeriksaan sebelum ubah
if (Test-Path -LiteralPath $lnk) {
  $existing = Get-ShortcutTarget $lnk
  if (-not (Test-SamePath $existing $destExe) -and -not $Force) {
    throw "Pintasan '$lnk' sudah ada dan menunjuk ke '$existing', bukan ke koha.exe yang akan dipasang. Pakai -Name lain (mis. -Name 'KOHA (Rust)'), atau -Force untuk menimpanya."
  }
}
if ((Test-Path -LiteralPath $destExe) -and (Test-KohaRunningFrom $destExe)) {
  throw "KOHA sedang berjalan dari $destExe; tutup dulu supaya berkasnya bisa diperbarui."
}

# ------------------------------------------------------------------- pasang
Write-Host "Sumber    : $Source  ($versionLine)"
Write-Host "Dipasang  : $destExe"
Write-Host "Pintasan  : $lnk"

if ($PSCmdlet.ShouldProcess($destExe, 'Salin koha.exe')) {
  [void](New-Item -ItemType Directory -Force -Path $InstallDir)
  Copy-Item -LiteralPath $Source -Destination $destExe -Force
  if ((Get-Sha256 $Source) -ne (Get-Sha256 $destExe)) { throw 'Salinan koha.exe tidak identik dengan sumber.' }
}

if ($PSCmdlet.ShouldProcess($lnk, 'Buat pintasan')) {
  [void](New-Item -ItemType Directory -Force -Path $ShortcutDir)
  $shell = New-Object -ComObject WScript.Shell
  $shortcut = $shell.CreateShortcut($lnk)
  $shortcut.TargetPath = $destExe
  $shortcut.WorkingDirectory = $InstallDir
  $shortcut.Description = 'KOHA (Kwik One-Hotkey Access)'
  $shortcut.Save()

  $back = Get-ShortcutTarget $lnk
  if (-not (Test-SamePath $back $destExe)) { throw "Pintasan dibuat tetapi menunjuk ke '$back', bukan '$destExe'." }

  Write-Host ''
  Write-Host "Terpasang. '$Name' kini ada di Start Menu."
  Write-Host ''
  Write-Host 'Langkah berikutnya:'
  Write-Host "  1. Uji: tekan tombol Windows, ketik '$Name', lalu Enter. Menu harus muncul tanpa jendela konsol."
  Write-Host '  2. Lenovo Vantage: Device settings > Input > User defined key > pilih tombol >'
  Write-Host "     'Open applications and files' > pilih '$Name'."
  Write-Host "  3. Memperbarui nanti: bangun ulang (cargo build --release), lalu jalankan skrip ini lagi."
  Write-Host "  Menghapus: .\tools\install-shortcut.ps1 -Uninstall -Name '$Name'"
  Write-Host '  Config dan state KOHA tidak disentuh.'
}
