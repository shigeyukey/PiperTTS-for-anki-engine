# Copyright (C) Shigeyuki <http://patreon.com/Shigeyuki>
# License: GNU AGPL version 3 or later <http://www.gnu.org/licenses/agpl.html>

[CmdletBinding()]
param(
    [string]$EmsdkRoot = "",
    [string]$CargoBinDir = "",
    [string]$SourceDir = "",
    [string]$OutDir = "",
    [string]$LicenseOutDir = "",
    [switch]$KeepRawWasm,
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"

function Write-Step([string]$msg) { Write-Host "[build_jpreprocess_wasm] $msg" }
function Fail([string]$msg) { Write-Host "[build_jpreprocess_wasm] ERROR: $msg"; exit 1 }

$ScriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = Split-Path -Parent $ScriptRoot
if (-not $SourceDir) { $SourceDir = Join-Path $RepoRoot "jpreprocess_poc" }
if (-not $OutDir) { $OutDir = Join-Path $RepoRoot "artifacts\built\ja" }
if (-not $LicenseOutDir) { $LicenseOutDir = Join-Path $RepoRoot "artifacts\licenses" }

if (-not (Test-Path (Join-Path $SourceDir "Cargo.toml"))) {
    Fail "jpreprocess_poc not found: $SourceDir"
}

New-Item -ItemType Directory -Force $OutDir | Out-Null
New-Item -ItemType Directory -Force $LicenseOutDir | Out-Null

if (-not $EmsdkRoot) {
    Fail "emsdk is not set (pass -EmsdkRoot, e.g. -EmsdkRoot C:\_github\emsdk)"
}

Write-Step "activating emsdk: $EmsdkRoot"
if (-not (Test-Path $EmsdkRoot)) { Fail "emsdk not found: $EmsdkRoot" }
$emsdkEnv = Join-Path $EmsdkRoot "emsdk_env.ps1"
if (Test-Path $emsdkEnv) { & $emsdkEnv | Out-Null }

$EmscriptenDir = Join-Path $EmsdkRoot "upstream\emscripten"
$emcc = Join-Path $EmscriptenDir "emcc.exe"
if (-not (Test-Path $emcc)) {
    $cmd = Get-Command emcc -ErrorAction SilentlyContinue
    if ($cmd) { $emcc = $cmd.Source } else { Fail "emcc not found (searched $EmscriptenDir)" }
}
$emccVersion = (& $emcc --version 2>&1 | Select-Object -First 1)
Write-Step "emcc: $emccVersion"

if (-not $CargoBinDir) { $CargoBinDir = Join-Path $env:USERPROFILE ".cargo\bin" }
if (Test-Path $CargoBinDir) { $env:Path = "$CargoBinDir;$env:Path" }
$cargo = (Get-Command cargo -ErrorAction SilentlyContinue).Source
if (-not $cargo) { Fail "cargo not found (install Rust or pass -CargoBinDir)" }
$rustcVersion = (& rustc --version 2>&1 | Select-Object -First 1)
Write-Step "rustc: $rustcVersion"

if (-not $SkipBuild) {
    Write-Step "building jpreprocess_poc (release, wasm32-unknown-emscripten)"
    Push-Location $SourceDir
    try {
        & cargo build --release --target wasm32-unknown-emscripten
        if ($LASTEXITCODE -ne 0) { Fail "cargo build failed with exit code $LASTEXITCODE" }
    } finally {
        Pop-Location
    }
} else {
    Write-Step "skipping cargo build (-SkipBuild)"
}

$releaseDir = Join-Path $SourceDir "target\wasm32-unknown-emscripten\release"
$srcJs = Join-Path $releaseDir "jpreprocess_poc.js"
$srcWasm = Join-Path $releaseDir "jpreprocess_poc.wasm"
foreach ($path in @($srcJs, $srcWasm)) {
    if (-not (Test-Path $path)) { Fail "build artifact not found: $path" }
}

$outJs = Join-Path $OutDir "ja_phonemizer.js"
$outGz = Join-Path $OutDir "ja_phonemizer.wasm.gz"
$outRaw = Join-Path $OutDir "ja_phonemizer.wasm"

Copy-Item -Force $srcJs $outJs
Write-Step "ja_phonemizer.js: $([math]::Round((Get-Item $outJs).Length / 1KB, 1)) KB"

$wasmBytes = (Get-Item $srcWasm).Length
Write-Step "compressing the wasm ($([math]::Round($wasmBytes / 1MB, 1)) MB) ..."
$input = [System.IO.File]::ReadAllBytes($srcWasm)
$outStream = [System.IO.File]::Create($outGz)
try {
    $gzStream = New-Object System.IO.Compression.GZipStream(
        $outStream, [System.IO.Compression.CompressionLevel]::SmallestSize)
    try {
        $gzStream.Write($input, 0, $input.Length)
    } finally {
        $gzStream.Dispose()
    }
} finally {
    $outStream.Dispose()
}
$gzBytes = (Get-Item $outGz).Length
Write-Step ("ja_phonemizer.wasm.gz: {0:N1} MB ({1:N1}% of the wasm)" -f ($gzBytes / 1MB), (100 * $gzBytes / $wasmBytes))

if ($KeepRawWasm) {
    Copy-Item -Force $srcWasm $outRaw
    Write-Step "ja_phonemizer.wasm: $([math]::Round($wasmBytes / 1MB, 1)) MB (debug copy)"
} elseif (Test-Path $outRaw) {
    Remove-Item -Force $outRaw
}

$licensePath = Join-Path $LicenseOutDir "LICENSE-jpreprocess.txt"
try {
    Invoke-WebRequest -Uri "https://raw.githubusercontent.com/jpreprocess/jpreprocess/main/LICENSE" -OutFile $licensePath
    Write-Step "collected license: LICENSE-jpreprocess.txt"
} catch {
    $note = @(
        "The license text could not be collected automatically.",
        "See: https://github.com/jpreprocess/jpreprocess"
    ) -join "`r`n"
    [System.IO.File]::WriteAllText($licensePath, $note)
    Write-Step "WARNING: license text not found, wrote a pointer: LICENSE-jpreprocess.txt"
}

$sha256Wasm = (Get-FileHash -Path $srcWasm -Algorithm SHA256).Hash
$sha256Gz = (Get-FileHash -Path $outGz -Algorithm SHA256).Hash
$info = [ordered]@{
    built_at           = (Get-Date).ToString("s")
    emscripten         = "$emccVersion"
    rustc              = "$rustcVersion"
    wasm_bytes         = $wasmBytes
    wasm_mb            = [math]::Round($wasmBytes / 1MB, 1)
    wasm_gz_bytes      = $gzBytes
    wasm_gz_mb         = [math]::Round($gzBytes / 1MB, 1)
    wasm_sha256        = $sha256Wasm
    wasm_gz_sha256     = $sha256Gz
    raw_wasm_included  = [bool]$KeepRawWasm
}
$info | ConvertTo-Json -Depth 4 | Set-Content -Encoding utf8 (Join-Path $OutDir "build_info.json")

Write-Step "SUCCESS"
Write-Step "  output: $OutDir"
Write-Step "  licenses: $LicenseOutDir"
