<#
.SYNOPSIS
  Mengukur waktu start KOHA: dari proses dijalankan sampai ada jendela KOHA yang terlihat.

.DESCRIPTION
  Untuk tiap target, skrip menjalankan program, lalu memolling (EnumWindows) sampai ada
  jendela terlihat berjudul "KOHA" milik proses itu, mencatat waktunya dengan Stopwatch,
  dan menghentikan prosesnya. Target dijalankan BERGANTIAN (bukan satu per satu) supaya
  gangguan sistem yang berubah-ubah tidak berat sebelah.

  Yang diukur adalah "jendela terlihat", BUKAN "piksel pertama tergambar" dan bukan
  "siap menerima tombol". Angkanya mencakup biaya Process.Start dari .NET, sama untuk semua
  target. Resolusi sekitar 0,5 ms. Untuk rincian tahap di dalam KOHA (Rust), pakai -Trace.

  Selama berjalan jangan menyentuh mouse atau keyboard: jendela KOHA muncul dan hilang
  berulang kali di layar, dan perpindahan fokus mengganggu pengukuran.

.PARAMETER Rust
  Jalur koha.exe (Rust). Bawaan: target\release\koha.exe.

.PARAMETER Ahk
  Jalur KOHA.exe (AutoHotkey) untuk dibandingkan. Opsional.

.PARAMETER Runs
  Jumlah pengukuran per target (setelah pemanasan). Bawaan 30.

.PARAMETER Warmup
  Jumlah putaran pemanasan yang dibuang. Bawaan 3.

.PARAMETER Floor
  Ikut mengukur `koha --print-config-path` (proses start dan keluar tanpa jendela): "harga
  minimum" memulai proses di mesin ini, termasuk pemindaian antivirus.

.PARAMETER Trace
  Mode jejak: jalankan -Rust (HARUS dibangun dengan --features startup-trace) dan rangkum
  waktu tiap tahap di dalam KOHA. Tidak membandingkan dengan AHK.

.PARAMETER Csv
  Jalur berkas untuk menyimpan semua pengukuran mentah (target, putaran, ms).

.EXAMPLE
  .\tools\bench-startup.ps1 -Ahk C:\path\ke\KOHA.exe -Floor

.EXAMPLE
  cargo build --release --features startup-trace -p koha-app --target-dir target\trace
  .\tools\bench-startup.ps1 -Trace -Rust target\trace\release\koha.exe -Runs 20
#>
[CmdletBinding()]
param(
  [string]$Rust,
  [string]$Ahk,
  [int]$Runs = 30,
  [int]$Warmup = 3,
  [int]$TimeoutMs = 5000,
  [int]$SettleMs = 400,
  [switch]$Floor,
  [switch]$Trace,
  [string]$Csv
)

$ErrorActionPreference = 'Stop'

# Nilai bawaan ditentukan di sini, bukan di blok param: di Windows PowerShell 5.1
# $PSScriptRoot kosong di dalam nilai bawaan parameter.
if (-not $Rust) {
  $scriptDir = Split-Path -Parent $PSCommandPath
  $Rust = Join-Path $scriptDir '..\target\release\koha.exe'
}

Add-Type @'
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;

public static class Bench {
  delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc p, IntPtr l);
  [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetWindowText(IntPtr h, StringBuilder s, int n);

  public class Result { public bool Ok; public double Ms; public string Error; }

  static uint wantedPid; static string wantedTitle; static bool found;
  static readonly EnumProc Callback = delegate(IntPtr h, IntPtr l) {
    uint pid; GetWindowThreadProcessId(h, out pid);
    if (pid == wantedPid && IsWindowVisible(h)) {
      var sb = new StringBuilder(64); GetWindowText(h, sb, 64);
      if (sb.ToString() == wantedTitle) { found = true; return false; }
    }
    return true;
  };

  static void Kill(Process p) {
    try { if (!p.HasExited) p.Kill(); p.WaitForExit(3000); } catch (Exception) { }
    p.Dispose();
  }

  // Dari sebelum Process.Start sampai ada jendela terlihat berjudul `title` milik proses itu.
  public static Result MeasureWindow(string exe, string args, string title, int timeoutMs) {
    var r = new Result();
    var psi = new ProcessStartInfo(exe, args); psi.UseShellExecute = false;
    var sw = Stopwatch.StartNew();
    Process p;
    try { p = Process.Start(psi); } catch (Exception e) { r.Error = e.Message; return r; }
    wantedPid = (uint)p.Id; wantedTitle = title;
    while (sw.ElapsedMilliseconds < timeoutMs) {
      found = false; EnumWindows(Callback, IntPtr.Zero);
      if (found) { r.Ok = true; break; }
      if (p.HasExited) { r.Error = "proses keluar sebelum ada jendela (kode " + p.ExitCode + ")"; break; }
      Thread.Sleep(0);
    }
    r.Ms = sw.Elapsed.TotalMilliseconds;
    if (!r.Ok && r.Error == null) r.Error = "timeout";
    Kill(p);
    return r;
  }

  // Dari sebelum Process.Start sampai proses selesai sendiri (tanpa jendela).
  public static Result MeasureExit(string exe, string args, int timeoutMs) {
    var r = new Result();
    var psi = new ProcessStartInfo(exe, args);
    psi.UseShellExecute = false; psi.RedirectStandardOutput = true; psi.RedirectStandardError = true;
    var sw = Stopwatch.StartNew();
    Process p;
    try { p = Process.Start(psi); } catch (Exception e) { r.Error = e.Message; return r; }
    bool exited = p.WaitForExit(timeoutMs);
    r.Ms = sw.Elapsed.TotalMilliseconds;
    r.Ok = exited; if (!exited) r.Error = "timeout";
    Kill(p);
    return r;
  }

  // Menjalankan program dengan stderr ditangkap, berhenti begitu baris `stopAt` muncul.
  public static List<string> RunTrace(string exe, string args, string stopAt, int timeoutMs) {
    var lines = new List<string>();
    var done = new ManualResetEvent(false);
    var psi = new ProcessStartInfo(exe, args);
    psi.UseShellExecute = false; psi.RedirectStandardError = true; psi.RedirectStandardOutput = true;
    var p = Process.Start(psi);
    p.ErrorDataReceived += delegate(object s, DataReceivedEventArgs e) {
      if (e.Data == null) return;
      lock (lines) { lines.Add(e.Data); }
      if (e.Data.Contains(stopAt)) done.Set();
    };
    p.OutputDataReceived += delegate(object s, DataReceivedEventArgs e) { };
    p.BeginErrorReadLine(); p.BeginOutputReadLine();
    done.WaitOne(timeoutMs);
    Kill(p);
    lock (lines) { return new List<string>(lines); }
  }
}
'@

function Get-Stats([double[]]$Values) {
  $sorted = @($Values | Sort-Object)
  $n = $sorted.Count
  if ($n -eq 0) { return [pscustomobject]@{ N = 0; Min = $null; Median = $null; Mean = $null; P90 = $null; Max = $null } }
  $mid = [math]::Floor($n / 2)
  $median = if ($n % 2) { $sorted[$mid] } else { ($sorted[$mid - 1] + $sorted[$mid]) / 2 }
  $p90 = $sorted[[math]::Min($n - 1, [math]::Ceiling(0.9 * $n) - 1)]
  [pscustomobject]@{
    N = $n; Min = $sorted[0]; Median = $median
    Mean = ($sorted | Measure-Object -Average).Average; P90 = $p90; Max = $sorted[$n - 1]
  }
}

function Format-Ms($v) { if ($null -eq $v) { '-' } else { '{0,8:N1}' -f $v } }

function Assert-File($path, $name) {
  if (-not (Test-Path -LiteralPath $path)) { throw "$name tidak ditemukan: $path" }
  (Resolve-Path -LiteralPath $path).Path
}

$Rust = Assert-File $Rust 'koha.exe (Rust)'
$tempState = Join-Path $env:TEMP 'koha-bench-state.toml'   # supaya state asli tidak tersentuh
$rustArgs = "--state `"$tempState`""

# ---------------------------------------------------------------- mode jejak
if ($Trace) {
  Write-Host "Mode jejak: $Rust"
  Write-Host "(harus dibangun dengan --features startup-trace; jangan sentuh mouse/keyboard)`n"
  $perStage = [ordered]@{}
  $good = 0
  for ($i = 1; $i -le ($Runs + $Warmup); $i++) {
    $lines = [Bench]::RunTrace($Rust, $rustArgs, 'first frame presented', $TimeoutMs)
    $parsed = @($lines | ForEach-Object {
      if ($_ -match '^\[trace\]\s+([0-9.]+)\s+ms\s+(.+)$') { [pscustomobject]@{ Ms = [double]$Matches[1]; Stage = $Matches[2].Trim() } }
    })
    if ($parsed.Count -eq 0 -or $parsed[-1].Stage -ne 'first frame presented') { Write-Warning "putaran $i tanpa jejak lengkap"; Start-Sleep -Milliseconds $SettleMs; continue }
    if ($i -gt $Warmup) {
      $good++
      foreach ($entry in $parsed) {
        if (-not $perStage.Contains($entry.Stage)) { $perStage[$entry.Stage] = New-Object System.Collections.Generic.List[double] }
        $perStage[$entry.Stage].Add($entry.Ms)
      }
    }
    Start-Sleep -Milliseconds $SettleMs
  }
  if ($good -eq 0) { throw 'tidak ada jejak yang terkumpul; pastikan binary dibangun dengan --features startup-trace' }
  Write-Host ("{0,-24} {1,10} {2,10} {3,10} {4,10}" -f 'tahap', 'median ms', 'selisih', 'min', 'maks')
  $previous = 0.0
  foreach ($stage in $perStage.Keys) {
    $s = Get-Stats $perStage[$stage].ToArray()
    Write-Host ("{0,-24} {1,10:N3} {2,10:N3} {3,10:N3} {4,10:N3}" -f $stage, $s.Median, ($s.Median - $previous), $s.Min, $s.Max)
    $previous = $s.Median
  }
  Write-Host "`n$good putaran terukur. 'selisih' = tambahan median terhadap tahap sebelumnya."
  Write-Host "Waktu 0 = awal main; tidak mencakup waktu OS memuat proses sebelum main."
  return
}

# ------------------------------------------------------------ mode perbandingan
$targets = @()
$targets += [pscustomobject]@{ Name = 'Rust  koha.exe'; Kind = 'window'; Exe = $Rust; Args = $rustArgs; Values = New-Object System.Collections.Generic.List[double]; First = $null; Failures = 0; LastError = $null }
if ($Ahk) {
  $ahkPath = Assert-File $Ahk 'KOHA.exe (AHK)'
  $targets += [pscustomobject]@{ Name = 'AHK   KOHA.exe'; Kind = 'window'; Exe = $ahkPath; Args = ''; Values = New-Object System.Collections.Generic.List[double]; First = $null; Failures = 0; LastError = $null }
}
if ($Floor) {
  $targets += [pscustomobject]@{ Name = 'Rust  --print-config-path (tanpa jendela)'; Kind = 'exit'; Exe = $Rust; Args = "--print-config-path --state `"$tempState`""; Values = New-Object System.Collections.Generic.List[double]; First = $null; Failures = 0; LastError = $null }
}

function Measure-Target($target) {
  if ($target.Kind -eq 'window') { [Bench]::MeasureWindow($target.Exe, $target.Args, 'KOHA', $TimeoutMs) }
  else { [Bench]::MeasureExit($target.Exe, $target.Args, $TimeoutMs) }
}

$total = ($Runs + $Warmup + 1) * $targets.Count
Write-Host ("Pengukuran: {0} target, {1} putaran (+{2} pemanasan), sekitar {3:N0} detik." -f $targets.Count, $Runs, $Warmup, ($total * ($SettleMs + 150) / 1000))
Write-Host "JANGAN sentuh mouse atau keyboard sampai selesai.`n"

# Peluncuran pertama sesi ini ("dingin"): dicatat terpisah dari statistik utama.
foreach ($t in $targets) {
  $r = Measure-Target $t
  if ($r.Ok) { $t.First = $r.Ms } else { $t.LastError = $r.Error }
  Start-Sleep -Milliseconds $SettleMs
}

$raw = New-Object System.Collections.Generic.List[object]
for ($round = 1; $round -le ($Warmup + $Runs); $round++) {
  # Urutan target dibalik tiap putaran genap supaya tidak ada yang selalu jalan lebih dulu.
  $order = if ($round % 2) { $targets } else { $targets[($targets.Count - 1)..0] }
  foreach ($t in $order) {
    $r = Measure-Target $t
    if ($round -gt $Warmup) {
      if ($r.Ok) { $t.Values.Add($r.Ms); $raw.Add([pscustomobject]@{ Target = $t.Name; Round = $round - $Warmup; Ms = $r.Ms }) }
      else { $t.Failures++; $t.LastError = $r.Error }
    }
    Start-Sleep -Milliseconds $SettleMs
  }
  if ($round % 10 -eq 0) { Write-Host ("  ... putaran {0}/{1}" -f $round, ($Warmup + $Runs)) }
}

Write-Host "`nHasil (ms, waktu sampai jendela KOHA terlihat; makin kecil makin baik)`n"
Write-Host ("{0,-42} {1,4} {2,8} {3,8} {4,8} {5,8} {6,8} {7,9}" -f 'target', 'N', 'min', 'median', 'rata2', 'p90', 'maks', 'pertama')
foreach ($t in $targets) {
  $s = Get-Stats $t.Values.ToArray()
  Write-Host ("{0,-42} {1,4} {2} {3} {4} {5} {6} {7}" -f $t.Name, $s.N, (Format-Ms $s.Min), (Format-Ms $s.Median), (Format-Ms $s.Mean), (Format-Ms $s.P90), (Format-Ms $s.Max), (Format-Ms $t.First))
  if ($t.Failures -gt 0) { Write-Warning ("{0}: {1} pengukuran gagal (terakhir: {2})" -f $t.Name, $t.Failures, $t.LastError) }
}
Write-Host @"

Catatan:
 - 'pertama' = peluncuran pertama sesi ini sebelum pemanasan (paling dekat dengan keadaan dingin).
 - Angka mencakup biaya Process.Start dari .NET (sama untuk semua target) dan resolusi ~0,5 ms.
 - Baris '--print-config-path' = harga minimum memulai sebuah proses di mesin ini.
 - 'Jendela terlihat' bukan 'piksel pertama tergambar'. Rincian tahap di Rust: gunakan -Trace.
 - Satu mesin, satu sesi: jangan dibaca sebagai klaim umum.
"@

if ($Csv) {
  $raw | Export-Csv -NoTypeInformation -Encoding utf8 -LiteralPath $Csv
  Write-Host "Data mentah: $Csv"
}
