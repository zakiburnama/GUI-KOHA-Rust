<#
.SYNOPSIS
  Menyusun catatan rilis suatu versi dari CHANGELOG.md, siap ditempel di GitHub Releases.

.DESCRIPTION
  Mengambil bagian `## [<versi>]` dari CHANGELOG.md, menambahkan pembuka singkat (hanya
  Windows, tidak ditandatangani, cara memakai) dan SHA-256 paket bila ada, dan mengubah
  tautan relatif menjadi tautan absolut ke repositori pada tag rilis, karena catatan rilis
  GitHub tidak punya konteks repositori.

  Hasilnya dicetak, dan ditulis ke -OutFile bila diberikan. Skrip ini tidak menerbitkan
  apa pun.

.PARAMETER Version
  Versi tanpa awalan "v", misalnya 0.1.0.

.PARAMETER Zip
  Jalur zip rilis. Bila diberikan, SHA-256-nya dibaca dari berkas `<zip>.sha256`. Bila tidak,
  dicari di dist\ (paket `-dryrun` diabaikan).

.PARAMETER OutFile
  Tulis hasilnya juga ke berkas ini (UTF-8 tanpa BOM).

.EXAMPLE
  .\tools\release-notes.ps1 -Version 0.1.0 -OutFile dist\RELEASE_NOTES.md
#>
[CmdletBinding()]
param(
  [Parameter(Mandatory)][string]$Version,
  [string]$Zip,
  [string]$OutFile
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent (Split-Path -Parent $PSCommandPath)

# ------------------------------------------------------------ bagian CHANGELOG
$lines = Get-Content -LiteralPath (Join-Path $root 'CHANGELOG.md')
$heading = '^##\s+\[' + [regex]::Escape($Version) + '\](?:\s+-\s+(?<date>.+))?\s*$'
$start = -1
for ($i = 0; $i -lt $lines.Count; $i++) {
  if ($lines[$i] -match $heading) { $start = $i; $date = $Matches['date']; break }
}
if ($start -lt 0) { throw "CHANGELOG.md tidak punya bagian '## [$Version]'." }

$end = $lines.Count
for ($i = $start + 1; $i -lt $lines.Count; $i++) {
  if ($lines[$i] -match '^##\s+\[' -or $lines[$i] -match '^\[[^\]]+\]:\s') { $end = $i; break }
}
$section = @($lines[($start + 1)..($end - 1)])
while ($section.Count -and -not $section[0].Trim()) { $section = @($section | Select-Object -Skip 1) }
while ($section.Count -and -not $section[-1].Trim()) { $section = @($section | Select-Object -SkipLast 1) }
if (-not $section.Count) { throw "Bagian '## [$Version]' di CHANGELOG.md kosong." }

if (-not $date -or $date -notmatch '^\d{4}-\d{2}-\d{2}$') {
  Write-Warning "CHANGELOG.md belum menanggali '$Version' (tertulis: '$date'). Ubah judulnya menjadi '## [$Version] - TAHUN-BULAN-TANGGAL' sebelum memberi tag."
}

# ------------------------------------------------------------ tautan absolut
$remote = (git -C $root remote get-url origin 2>$null | Out-String).Trim()
if (-not $remote) { throw 'repositori tidak punya remote origin; tidak bisa membentuk tautan absolut.' }
$base = ($remote -replace '\.git$', '') -replace '^git@github\.com:', 'https://github.com/'
$tag = "v$Version"
$body = ($section -join "`n")
# ](target) yang bukan web, mailto, atau jangkar di halaman yang sama -> tautan ke tag rilis
$body = [regex]::Replace($body, '\]\((?!https?:|mailto:|#)([^)\s]+)\)', { param($m) "](" + "$base/blob/$tag/" + $m.Groups[1].Value + ")" })
# Judul bagian turun satu tingkat: "###" -> "##" (judul rilis sudah "#").
$body = [regex]::Replace($body, '(?m)^###\s', '## ')

# ------------------------------------------------------------ SHA-256
$hashFile = $null
if ($Zip) { $hashFile = "$Zip.sha256" }
else {
  $hashFile = Get-ChildItem -LiteralPath (Join-Path $root 'dist') -Filter "koha-$Version-*.zip.sha256" -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -notmatch 'dryrun' } | Select-Object -First 1 -ExpandProperty FullName
}
$assetLine = $null
if ($hashFile -and (Test-Path -LiteralPath $hashFile)) {
  $parts = (Get-Content -LiteralPath $hashFile -Raw).Trim() -split '\s+', 2
  $assetLine = "SHA-256 ``$($parts[1].Trim())``: ``$($parts[0])``"
} else {
  Write-Warning 'Paket atau berkas .sha256 tidak ditemukan; catatan rilis dibuat tanpa SHA-256. Jalankan tools\package-release.ps1 -FromClean lebih dulu.'
}

# ------------------------------------------------------------ rakit
$out = New-Object System.Collections.Generic.List[string]
$out.Add("# KOHA $Version")
$out.Add('')
$out.Add('Popup menu keyboard-saja seperti rofi untuk Windows: dipanggil, memilih satu aksi, lalu keluar tanpa proses yang tertinggal.')
$out.Add('')
$out.Add('**Hanya Windows** (diuji di Windows 11). Unduh zip di bawah, bongkar, dan jalankan `koha.exe`; cara memakainya ada di [README]' + "($base/blob/$tag/README.md)" + '.')
$out.Add('')
if ($assetLine) { $out.Add($assetLine); $out.Add('') }
$out.Add('> Berkas ini **tidak ditandatangani** secara digital, jadi Windows SmartScreen bisa memperingatkan. Hash di atas memungkinkanmu memastikan berkas yang kamu unduh sama dengan yang dirilis.')
$out.Add('')
$out.Add($body)
$out.Add('')
$out.Add("Riwayat lengkap: [CHANGELOG]($base/blob/$tag/CHANGELOG.md).")
$text = ($out -join "`n") + "`n"

if ($OutFile) {
  $dir = Split-Path -Parent $OutFile
  if ($dir) { New-Item -ItemType Directory -Force $dir | Out-Null }
  [IO.File]::WriteAllText($OutFile, $text, (New-Object System.Text.UTF8Encoding $false))
  Write-Host "Catatan rilis ditulis ke: $OutFile"
  Write-Host ''
}
$text
