<#
.SYNOPSIS
  Mengemas rilis Windows KOHA: sebuah zip berisi koha.exe, dokumentasi, dan daftar lisensi,
  beserta berkas checksum SHA-256.

.DESCRIPTION
  Hasilnya ada di dist\ (diabaikan git):

    koha-<versi>-<target>.zip
    koha-<versi>-<target>.zip.sha256

  Mode bawaan membangun dari working tree. Untuk rilis sungguhan pakai -FromClean: skrip
  meng-klon repositori ke folder sementara, membangun dari sana (jadi hanya isi yang sudah
  di-commit yang masuk), dan mengemas hasilnya.

  Semua mode menjalankan gerbang berikut dan berhenti dengan error bila gagal:
    - THIRD_PARTY_LICENSES.md harus mutakhir (tools\gen-third-party.ps1 -Check);
    - versi dari `koha.exe --version` harus sama dengan versi di Cargo.toml;
    - koha.exe tidak boleh memuat nama pengguna atau jalur lokal pembuatnya (jalur sumber
      dependensi biasanya tertanam untuk pesan panic; build dibuat dengan
      --remap-path-prefix agar tidak bocor);
    - zip dibongkar ke folder sementara, isinya dibandingkan dengan aslinya, dan
      `koha.exe --version` dijalankan dari hasil bongkaran.

  Paket yang dibuat dari working tree kotor atau dengan -IncludeUncommitted diberi akhiran
  "-dryrun" pada namanya dan BUILD_INFO.txt-nya menyebut hal itu: itu bukan rilis.

  Paket TIDAK ditandatangani secara digital, sehingga Windows SmartScreen bisa memperingatkan.

.PARAMETER OutDir
  Folder hasil. Bawaan: dist (di akar repositori).

.PARAMETER FromClean
  Bangun dari klon bersih repositori (lihat -Ref), bukan dari working tree.

.PARAMETER Ref
  Commit atau tag yang di-klon dengan -FromClean. Bawaan: HEAD.

.PARAMETER IncludeUncommitted
  Hanya dengan -FromClean: setelah meng-klon, salin perubahan working tree yang belum
  di-commit ke klon. Untuk latihan sebelum commit; hasilnya bukan rilis (diberi "-dryrun").

.PARAMETER KeepWork
  Jangan hapus folder kerja sementara (untuk debugging).

.EXAMPLE
  .\tools\package-release.ps1 -FromClean -Ref v0.1.0
#>
[CmdletBinding()]
param(
  [string]$OutDir,
  [switch]$FromClean,
  [string]$Ref = 'HEAD',
  [switch]$IncludeUncommitted,
  [switch]$KeepWork
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent (Split-Path -Parent $PSCommandPath)
if (-not $OutDir) { $OutDir = Join-Path $root 'dist' }
if ($IncludeUncommitted -and -not $FromClean) { throw '-IncludeUncommitted hanya berarti bersama -FromClean.' }

# ZipFile ada di ...FileSystem, sedangkan ZipArchiveMode dan ZipArchive di System.IO.Compression.
Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem

function Invoke-Native([string]$What, [scriptblock]$Command) {
  & $Command
  if ($LASTEXITCODE -ne 0) { throw "$What gagal (kode keluar $LASTEXITCODE)" }
}

function Get-Sha256([string]$Path) { (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant() }

# `koha.exe` adalah aplikasi GUI: PowerShell hanya menunggunya (dan menangkap keluarannya)
# bila dialirkan lewat pipa.
function Get-KohaVersionLine([string]$Exe) { (& $Exe --version | Out-String).Trim() }

# Tautan relatif di berkas .md di dalam `$Folder` yang tidak menunjuk berkas yang ada.
# (Jawaban yang sama dengan tes `all_relative_markdown_links_resolve`, tetapi atas isi
# paket, supaya tata letak paket tidak diam-diam merusak tautan di dokumentasinya.)
function Get-BrokenLinks([string]$Folder) {
  $base = (Resolve-Path -LiteralPath $Folder).Path.TrimEnd([char]92)   # [char]92 = backslash
  $broken = @()
  foreach ($md in Get-ChildItem -LiteralPath $base -Filter '*.md' -Recurse -File) {
    if ($md.Name -eq 'THIRD_PARTY_LICENSES.md') { continue }   # dihasilkan, hanya berisi URL web
    $inCode = $false
    foreach ($line in [IO.File]::ReadAllLines($md.FullName)) {
      if ($line.TrimStart().StartsWith('```')) { $inCode = -not $inCode; continue }
      if ($inCode) { continue }
      foreach ($m in [regex]::Matches($line, '\]\(([^)]*)\)')) {
        $target = $m.Groups[1].Value.Trim().Trim('<', '>')
        if (-not $target -or $target.StartsWith('#') -or $target -match '^(https?:|mailto:)') { continue }
        $path = ($target -split '[#?]')[0]
        if (-not $path) { continue }
        if (-not (Test-Path -LiteralPath (Join-Path $md.DirectoryName $path))) {
          $broken += ('{0}: {1}' -f $md.FullName.Substring($base.Length + 1), $target)
        }
      }
    }
  }
  $broken
}

$work = Join-Path $env:TEMP ("koha-package-" + [guid]::NewGuid().ToString('N').Substring(0, 8))
New-Item -ItemType Directory -Force $work | Out-Null

try {
  # ------------------------------------------------------------ sumber
  $head = (git -C $root rev-parse HEAD).Trim()
  $dirtyList = @(git -C $root status --porcelain)
  $dirty = $dirtyList.Count -gt 0
  $includesUncommitted = $false

  if ($FromClean) {
    $src = Join-Path $work 'src'
    Invoke-Native 'git clone' { git clone --quiet --no-hardlinks $root $src }
    Invoke-Native "git checkout $Ref" { git -C $src checkout --quiet $Ref }
    $commit = (git -C $src rev-parse HEAD).Trim()
    if ($IncludeUncommitted) {
      $includesUncommitted = $true
      $changed = @(git -C $root ls-files --modified --others --exclude-standard)
      foreach ($rel in $changed) {
        $from = Join-Path $root $rel
        if (-not (Test-Path -LiteralPath $from -PathType Leaf)) { continue }   # berkas terhapus
        $to = Join-Path $src $rel
        New-Item -ItemType Directory -Force (Split-Path -Parent $to) | Out-Null
        Copy-Item -LiteralPath $from -Destination $to -Force
      }
      Write-Host ("Menyalin {0} berkas yang belum di-commit ke klon (latihan, bukan rilis)." -f $changed.Count)
    } elseif ($dirty) {
      Write-Warning ("Working tree punya {0} perubahan yang belum di-commit; -FromClean mengabaikannya." -f $dirtyList.Count)
    }
    $buildTarget = Join-Path $src 'target'
  } else {
    $src = $root
    $commit = $head
    if ($dirty) { Write-Warning "Working tree punya perubahan yang belum di-commit; paket ini dibuat dari working tree (-dryrun)." }
    $buildTarget = Join-Path $root 'target\package'
  }
  $isDryRun = $includesUncommitted -or ((-not $FromClean) -and $dirty)

  # ------------------------------------------------------------ gerbang: berkas wajib
  # Diperiksa sebelum build supaya kegagalan yang sudah pasti tidak memakan waktu kompilasi.
  $missing = @('README.md', 'CHANGELOG.md', 'CONTRIBUTING.md', 'THIRD_PARTY_LICENSES.md', 'deny.toml', 'docs', 'tools', 'Cargo.lock') |
    Where-Object { -not (Test-Path -LiteralPath (Join-Path $src $_)) }
  if ($missing) {
    throw ("berkas wajib tidak ada di sumber ({0}): {1}. Sudah di-commit?" -f $(if ($FromClean) { "commit $Ref" } else { 'working tree' }), ($missing -join ', '))
  }

  # ------------------------------------------------------------ gerbang: lisensi
  Write-Host 'Memeriksa THIRD_PARTY_LICENSES.md ...'
  & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $src 'tools\gen-third-party.ps1') -Check
  if ($LASTEXITCODE -ne 0) { throw 'THIRD_PARTY_LICENSES.md tidak mutakhir.' }

  # ------------------------------------------------------------ build
  # Memetakan jalur lokal supaya tidak tertanam di binary (lihat .DESCRIPTION). Pemisah
  # CARGO_ENCODED_RUSTFLAGS adalah karakter 0x1f, jadi jalur berspasi pun aman.
  $cargoHome = if ($env:CARGO_HOME) { $env:CARGO_HOME } else { Join-Path $env:USERPROFILE '.cargo' }
  $flags = @("--remap-path-prefix=$cargoHome=/cargo", "--remap-path-prefix=$src=/koha") -join [string][char]0x1f
  $previousFlags = [Environment]::GetEnvironmentVariable('CARGO_ENCODED_RUSTFLAGS', 'Process')
  [Environment]::SetEnvironmentVariable('CARGO_ENCODED_RUSTFLAGS', $flags, 'Process')
  Write-Host "Membangun rilis dari $src ..."
  try {
    Push-Location $src
    Invoke-Native 'cargo build --release' { cargo build --release --locked -p koha-app --target-dir $buildTarget }
  } finally {
    Pop-Location
    [Environment]::SetEnvironmentVariable('CARGO_ENCODED_RUSTFLAGS', $previousFlags, 'Process')
  }
  $exe = Join-Path $buildTarget 'release\koha.exe'
  if (-not (Test-Path -LiteralPath $exe)) { throw "koha.exe tidak ditemukan di $exe" }

  # ------------------------------------------------------------ gerbang: versi
  $cargoToml = Get-Content -LiteralPath (Join-Path $src 'Cargo.toml') -Raw
  if ($cargoToml -notmatch '(?ms)\[workspace\.package\].*?^version\s*=\s*"([^"]+)"') { throw 'versi tidak terbaca dari Cargo.toml' }
  $version = $Matches[1]
  $versionLine = Get-KohaVersionLine $exe
  if ($versionLine -ne "koha $version") { throw "versi tidak cocok: Cargo.toml = $version, koha.exe melaporkan '$versionLine'" }

  # ------------------------------------------------------------ gerbang: kebocoran jalur
  $bytes = [IO.File]::ReadAllBytes($exe)
  $text = [Text.Encoding]::GetEncoding('iso-8859-1').GetString($bytes)
  $needles = @($env:USERNAME, $env:USERPROFILE, $src, $root, $cargoHome) | Where-Object { $_ -and $_.Length -ge 4 } | Select-Object -Unique
  foreach ($needle in $needles) {
    if ($text.IndexOf($needle, [StringComparison]::OrdinalIgnoreCase) -ge 0) {
      throw "koha.exe memuat '$needle' (jalur atau nama lokal pembuatnya); build tidak boleh dibagikan."
    }
  }

  # ------------------------------------------------------------ isi paket
  $targetTriple = ((rustc -vV | Select-String '^host:').ToString() -replace '^host:\s*', '').Trim()
  $suffix = if ($isDryRun) { '-dryrun' } else { '' }
  $name = "koha-$version-$targetTriple$suffix"
  $stage = Join-Path $work $name
  New-Item -ItemType Directory -Force $stage | Out-Null

  Copy-Item -LiteralPath $exe -Destination (Join-Path $stage 'koha.exe')
  foreach ($file in 'README.md', 'CHANGELOG.md', 'CONTRIBUTING.md', 'THIRD_PARTY_LICENSES.md', 'deny.toml') {
    $from = Join-Path $src $file
    if (-not (Test-Path -LiteralPath $from)) { throw "berkas wajib tidak ada di sumber: $file" }
    Copy-Item -LiteralPath $from -Destination $stage
  }
  $license = @(Get-ChildItem -LiteralPath $src -File | Where-Object { $_.Name -match '^(?i)(LICEN[CS]E|COPYING)' })
  foreach ($file in $license) { Copy-Item -LiteralPath $file.FullName -Destination $stage }
  # docs\ ikut supaya tautan dan gambar di README tidak rusak.
  Copy-Item -LiteralPath (Join-Path $src 'docs') -Destination (Join-Path $stage 'docs') -Recurse
  # CONTRIBUTING, deny.toml, dan tools\ ikut karena README dan dokumen lain menautnya.
  Copy-Item -LiteralPath (Join-Path $src 'tools') -Destination (Join-Path $stage 'tools') -Recurse

  $exeHash = Get-Sha256 $exe
  $rustc = (rustc -V).Trim()
  function Info-Row([string]$Label, [string]$Value) { '{0,-18}: {1}' -f $Label, $Value }
  $buildInfo = @(
    "KOHA $version",
    (Info-Row 'target' $targetTriple),
    (Info-Row 'commit' $commit),
    (Info-Row 'dari klon bersih' $(if ($FromClean) { 'ya' } else { 'tidak (working tree)' })),
    (Info-Row 'UNCOMMITTED' $(if ($isDryRun) { 'YA. Ini paket latihan (dryrun), BUKAN rilis.' } else { 'tidak' })),
    (Info-Row 'dibangun' "$([DateTime]::UtcNow.ToString('yyyy-MM-dd HH:mm:ss')) UTC"),
    (Info-Row 'kompilator' $rustc),
    (Info-Row 'koha.exe' "$((Get-Item -LiteralPath $exe).Length) byte, SHA-256 $exeHash"),
    (Info-Row 'lisensi KOHA' $(if ($license.Count) { ($license.Name -join ', ') } else { 'belum ada (semua hak dilindungi penulis)' })),
    (Info-Row 'tanda tangan' 'tidak ditandatangani (SmartScreen bisa memperingatkan)')
  ) -join "`r`n"
  [IO.File]::WriteAllText((Join-Path $stage 'BUILD_INFO.txt'), $buildInfo + "`r`n", (New-Object Text.UTF8Encoding $false))

  # ------------------------------------------------------------ gerbang: tautan dokumentasi
  $brokenLinks = @(Get-BrokenLinks $stage)
  if ($brokenLinks.Count) { throw ('tautan rusak di dalam paket:' + [Environment]::NewLine + ($brokenLinks -join [Environment]::NewLine)) }

  # ------------------------------------------------------------ zip
  New-Item -ItemType Directory -Force $OutDir | Out-Null
  $zipPath = Join-Path $OutDir "$name.zip"
  if (Test-Path -LiteralPath $zipPath) { Remove-Item -LiteralPath $zipPath -Force }
  $zip = [IO.Compression.ZipFile]::Open($zipPath, [IO.Compression.ZipArchiveMode]::Create)
  try {
    $stageRoot = (Resolve-Path -LiteralPath $stage).Path.TrimEnd('\')
    foreach ($file in Get-ChildItem -LiteralPath $stage -File -Recurse | Sort-Object FullName) {
      $relative = $file.FullName.Substring($stageRoot.Length + 1).Replace('\', '/')
      # Nama entri memakai '/': Compress-Archive di PowerShell 5.1 memakai '\', yang rusak di OS lain.
      [void][IO.Compression.ZipFileExtensions]::CreateEntryFromFile($zip, $file.FullName, "$name/$relative", [IO.Compression.CompressionLevel]::Optimal)
    }
  } finally { $zip.Dispose() }

  $zipHash = Get-Sha256 $zipPath
  [IO.File]::WriteAllText("$zipPath.sha256", "$zipHash  $name.zip`n", (New-Object Text.UTF8Encoding $false))

  # ------------------------------------------------------------ verifikasi hasil
  Write-Host 'Memverifikasi paket (membongkar ke folder sementara) ...'
  $check = Join-Path $work 'extracted'
  [IO.Compression.ZipFile]::ExtractToDirectory($zipPath, $check)
  $extractedRoot = Join-Path $check $name
  foreach ($file in Get-ChildItem -LiteralPath $stage -File -Recurse) {
    $relative = $file.FullName.Substring($stageRoot.Length + 1)
    $copy = Join-Path $extractedRoot $relative
    if (-not (Test-Path -LiteralPath $copy)) { throw "tidak ada di zip: $relative" }
    if ((Get-Sha256 $copy) -ne (Get-Sha256 $file.FullName)) { throw "isi berbeda di zip: $relative" }
  }
  $reopen = [IO.Compression.ZipFile]::OpenRead($zipPath)
  try {
    $entries = @($reopen.Entries | ForEach-Object { $_.FullName })
    if ($entries | Where-Object { $_ -match '\\' }) { throw 'ada nama entri zip yang memakai backslash' }
    if ($entries | Where-Object { $_ -notlike "$name/*" }) { throw 'ada entri di luar folder paket' }
  } finally { $reopen.Dispose() }
  $extractedVersion = Get-KohaVersionLine (Join-Path $extractedRoot 'koha.exe')
  if ($extractedVersion -ne "koha $version") { throw "koha.exe hasil bongkaran melaporkan '$extractedVersion'" }
  $hashLine = (Get-Content -LiteralPath "$zipPath.sha256" -Raw).Trim()
  if ($hashLine -ne "$zipHash  $name.zip") { throw 'berkas checksum tidak sesuai' }

  # ------------------------------------------------------------ ringkasan
  Write-Host ''
  Write-Host "Paket   : $zipPath"
  Write-Host ("Ukuran  : {0:N0} byte (koha.exe {1:N0} byte)" -f (Get-Item -LiteralPath $zipPath).Length, (Get-Item -LiteralPath $exe).Length)
  Write-Host "SHA-256 : $zipHash"
  Write-Host "Commit  : $commit"
  Write-Host "Versi   : $extractedVersion (dari hasil bongkaran)"
  Write-Host "Isi     :"
  foreach ($entry in $entries) { Write-Host "  $entry" }
  if ($isDryRun) { Write-Warning 'Ini paket LATIHAN (-dryrun): ada perubahan yang belum di-commit. Bukan rilis.' }
  if (-not $license.Count) { Write-Host 'Catatan : belum ada berkas LICENSE di repositori, jadi tidak ada di paket.' }
} finally {
  if ($KeepWork) { Write-Host "Folder kerja dipertahankan: $work" }
  else { Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue }
}
