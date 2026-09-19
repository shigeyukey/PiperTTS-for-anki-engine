# Copyright (C) Shigeyuki <http://patreon.com/Shigeyuki>
# License: GNU AGPL version 3 or later <http://www.gnu.org/licenses/agpl.html>

[CmdletBinding()]
param(
    [string]$EmsdkRoot = "",
    [string]$SourceDir = "",
    [string]$LibpiperDir = "",
    [string]$LibpiperCommit = "850c45a3d70b5981de4d908ba98caf39796e7201",
    [string]$OutDir = "",
    [string]$OrtOutDir = "",
    [string]$LicenseOutDir = "",
    [string]$EspeakNgDir = "",
    [string]$EspeakDataDir = "",
    [string]$EspeakNgCommit = "212928b394a96e8fd2096616bfd54e17845c48f6",
    [string]$PiperTtsVersion = "",
    [string]$OrtWebVersion = "1.30.0",
    [switch]$ForceFetch,
    [switch]$SkipOrt
)

$ErrorActionPreference = "Stop"

function Write-Step([string]$msg) { Write-Host "[build_piper_wasm] $msg" }
function Fail([string]$msg) { Write-Host "[build_piper_wasm] ERROR: $msg"; exit 1 }

$ScriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = Split-Path -Parent $ScriptRoot
$BuildDir = Join-Path $RepoRoot "work\piper_wasm"
if (-not $SourceDir) { $SourceDir = Join-Path $RepoRoot "src" }
if (-not $OutDir) { $OutDir = Join-Path $RepoRoot "artifacts\built" }
if (-not $OrtOutDir) { $OrtOutDir = Join-Path $RepoRoot "artifacts\downloaded" }
if (-not $LicenseOutDir) { $LicenseOutDir = Join-Path $RepoRoot "artifacts\licenses" }

if (-not (Test-Path (Join-Path $SourceDir "piper_wasm_api.cpp"))) {
    Fail "source not found: $SourceDir\piper_wasm_api.cpp"
}

New-Item -ItemType Directory -Force $BuildDir | Out-Null
New-Item -ItemType Directory -Force $OutDir | Out-Null
New-Item -ItemType Directory -Force $OrtOutDir | Out-Null
New-Item -ItemType Directory -Force $LicenseOutDir | Out-Null

function Save-LicenseFile([string]$sourceDir, [string]$destinationName, [string]$licenseUrl) {
    if ($sourceDir) {
        $candidates = @("COPYING", "COPYING.txt", "LICENSE", "LICENSE.txt", "LICENSE.md")
        foreach ($candidate in $candidates) {
            $candidatePath = Join-Path $sourceDir $candidate
            if (Test-Path $candidatePath) {
                Copy-Item -Force $candidatePath (Join-Path $LicenseOutDir $destinationName)
                Write-Step "collected license: $destinationName"
                return
            }
        }
    }
    $note = @(
        "The license text could not be collected automatically.",
        "See: $licenseUrl"
    ) -join "`r`n"
    [System.IO.File]::WriteAllText((Join-Path $LicenseOutDir $destinationName), $note)
    Write-Step "WARNING: license text not found, wrote a pointer: $destinationName"
}

if ($LibpiperDir) {
    if (-not (Test-Path (Join-Path $LibpiperDir "src\piper.cpp"))) {
        Fail "libpiper source not found: $LibpiperDir"
    }
    $LibpiperDir = (Resolve-Path $LibpiperDir).Path
    $libpiperRefLabel = "local"
    Write-Step "using local libpiper source: $LibpiperDir"
} else {
    $cachedLibpiper = Join-Path $BuildDir "libpiper"
    if ((Test-Path (Join-Path $cachedLibpiper "src\piper.cpp")) -and (-not $ForceFetch)) {
        $LibpiperDir = $cachedLibpiper
        $libpiperRefLabel = $LibpiperCommit
        Write-Step "using cached libpiper source: $LibpiperDir"
    } else {
        $libpiperArchive = Join-Path $BuildDir "piper1-gpl-$LibpiperCommit.tar.gz"
        if ((-not (Test-Path $libpiperArchive)) -or $ForceFetch) {
            $libpiperUrl = "https://codeload.github.com/OHF-Voice/piper1-gpl/tar.gz/$LibpiperCommit"
            Write-Step "downloading piper1-gpl source: $libpiperUrl"
            Invoke-WebRequest -Uri $libpiperUrl -OutFile $libpiperArchive
        }
        $libpiperExtractDir = Join-Path $BuildDir "piper1-gpl-extract"
        if (Test-Path $libpiperExtractDir) { Remove-Item -Recurse -Force $libpiperExtractDir }
        New-Item -ItemType Directory -Force $libpiperExtractDir | Out-Null
        Write-Step "extracting libpiper from the piper1-gpl archive"
        tar -xzf $libpiperArchive -C $libpiperExtractDir
        $libpiperRoot = Get-ChildItem $libpiperExtractDir | Where-Object { $_.PSIsContainer } | Select-Object -First 1
        if (-not $libpiperRoot) { Fail "unexpected piper1-gpl archive layout" }
        $extractedLibpiper = Join-Path $libpiperRoot.FullName "libpiper"
        if (-not (Test-Path (Join-Path $extractedLibpiper "src\piper.cpp"))) {
            Fail "libpiper not found in the piper1-gpl archive"
        }
        Save-LicenseFile $libpiperRoot.FullName "LICENSE-piper1-gpl.txt" "https://github.com/OHF-Voice/piper1-gpl/blob/$LibpiperCommit/LICENSE.md"
        if (Test-Path $cachedLibpiper) { Remove-Item -Recurse -Force $cachedLibpiper }
        Move-Item $extractedLibpiper $cachedLibpiper
        Remove-Item -Recurse -Force $libpiperExtractDir
        $LibpiperDir = $cachedLibpiper
        $libpiperRefLabel = $LibpiperCommit
        Write-Step "installed libpiper: $LibpiperDir"
    }
}
Write-Step "libpiper: $LibpiperDir"
$piperLicensePath = Join-Path $LicenseOutDir "LICENSE-piper1-gpl.txt"
if (-not (Test-Path $piperLicensePath)) {
    Save-LicenseFile "" "LICENSE-piper1-gpl.txt" "https://github.com/OHF-Voice/piper1-gpl/blob/$LibpiperCommit/LICENSE.md"
}

if (-not $EmsdkRoot) {
    Fail "emsdk is not set (pass -EmsdkRoot, e.g. -EmsdkRoot C:\_github\emsdk)"
}

Write-Step "activating emsdk: $EmsdkRoot"
if (-not (Test-Path $EmsdkRoot)) { Fail "emsdk not found: $EmsdkRoot" }
$emsdkEnv = Join-Path $EmsdkRoot "emsdk_env.ps1"
if (Test-Path $emsdkEnv) { & $emsdkEnv | Out-Null }

$EmscriptenDir = Join-Path $EmsdkRoot "upstream\emscripten"
function Find-EmscriptenTool([string]$Name) {
    foreach ($ext in @(".exe", ".bat", ".cmd")) {
        $candidate = Join-Path $EmscriptenDir ($Name + $ext)
        if (Test-Path $candidate) { return $candidate }
    }
    $cmd = Get-Command $Name -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }
    return $null
}

$emcc = Find-EmscriptenTool "emcc"
$emcmake = Find-EmscriptenTool "emcmake"
if (-not $emcc) { Fail "emcc not found (searched $EmscriptenDir)" }
if (-not $emcmake) { Fail "emcmake not found (searched $EmscriptenDir)" }

$cmake = (Get-Command cmake -ErrorAction SilentlyContinue).Source
if (-not $cmake) { Fail "cmake not found in PATH" }

$emccVersion = (& $emcc --version 2>&1 | Select-Object -First 1)
Write-Step "emcc: $emccVersion"

$EspeakSrcDir = ""
if ($EspeakNgDir) {
    if (-not (Test-Path (Join-Path $EspeakNgDir "CMakeLists.txt"))) {
        Fail "espeak-ng source not found: $EspeakNgDir"
    }
    $EspeakSrcDir = (Resolve-Path $EspeakNgDir).Path
    Write-Step "using local espeak-ng source: $EspeakSrcDir"
} else {
    $cachedSrc = Join-Path $BuildDir "espeak-ng"
    if ((Test-Path (Join-Path $cachedSrc "CMakeLists.txt")) -and (-not $ForceFetch)) {
        $EspeakSrcDir = $cachedSrc
        Write-Step "using cached espeak-ng source: $EspeakSrcDir"
    } else {
        $archive = Join-Path $BuildDir "espeak-ng-$EspeakNgCommit.tar.gz"
        if ((-not (Test-Path $archive)) -or $ForceFetch) {
            $url = "https://codeload.github.com/espeak-ng/espeak-ng/tar.gz/$EspeakNgCommit"
            Write-Step "downloading espeak-ng source: $url"
            Invoke-WebRequest -Uri $url -OutFile $archive
        }
        $extractDir = Join-Path $BuildDir "espeak-ng-extract"
        if (Test-Path $extractDir) { Remove-Item -Recurse -Force $extractDir }
        New-Item -ItemType Directory -Force $extractDir | Out-Null
        Write-Step "extracting espeak-ng source"
        tar -xzf $archive -C $extractDir
        $inner = Get-ChildItem $extractDir | Where-Object { $_.PSIsContainer } | Select-Object -First 1
        if (-not $inner) { Fail "unexpected espeak-ng archive layout" }
        if (Test-Path $cachedSrc) { Remove-Item -Recurse -Force $cachedSrc }
        Move-Item $inner.FullName $cachedSrc
        Remove-Item -Recurse -Force $extractDir
        $EspeakSrcDir = $cachedSrc
    }
}
Save-LicenseFile $EspeakSrcDir "LICENSE-espeak-ng.txt" "https://github.com/espeak-ng/espeak-ng/blob/$EspeakNgCommit/COPYING"

function Patch-EspeakNgCompat([string]$srcDir) {
    $compatWchar = Join-Path $srcDir "src\include\compat\wchar.h"
    if (-not (Test-Path $compatWchar)) { return }

    $text = [System.IO.File]::ReadAllText($compatWchar)
    if ($text.Contains("piper_wasm patch")) { return }

    $names = @(
        "iswalnum", "iswalpha", "iswblank", "iswcntrl", "iswdigit",
        "iswgraph", "iswlower", "iswprint", "iswpunct", "iswspace",
        "iswupper", "iswxdigit", "tolower", "toupper"
    )

    $pushLines = @("/* piper_wasm patch (auto_build/scripts/build_piper_wasm.ps1) */")
    foreach ($name in $names) { $pushLines += "#pragma push_macro(`"$name`")" }
    foreach ($name in $names) { $pushLines += "#undef $name" }

    $popLines = @()
    for ($i = $names.Count - 1; $i -ge 0; $i--) { $popLines += "#pragma pop_macro(`"$($names[$i])`")" }

    $block = ($pushLines + @("#include_next <wchar.h>") + $popLines) -join "`r`n"
    $text = $text.Replace("#include_next <wchar.h>", $block)
    [System.IO.File]::WriteAllText($compatWchar, $text)
    Write-Step "patched espeak-ng compat/wchar.h for Emscripten"
}
Patch-EspeakNgCompat $EspeakSrcDir


$espeakBuild = Join-Path $BuildDir "espeak-ng-build"
$libespeak = $null
$libucd = $null

if (-not $ForceFetch) {
    $libespeak = Get-ChildItem $espeakBuild -Recurse -Filter "libespeak-ng.a" -ErrorAction SilentlyContinue | Select-Object -First 1
    $libucd = Get-ChildItem $espeakBuild -Recurse -Filter "libucd.a" -ErrorAction SilentlyContinue | Select-Object -First 1
}

if ($libespeak -and $libucd) {
    Write-Step "reusing espeak-ng build: $espeakBuild"
} else {
    if (-not (Test-Path (Join-Path $espeakBuild "build.ninja"))) {
        Write-Step "configuring espeak-ng (emcmake/cmake)"
        if (Test-Path $espeakBuild) { Remove-Item -Recurse -Force $espeakBuild }
        & $emcmake $cmake -S $EspeakSrcDir -B $espeakBuild -G Ninja `
            -DCMAKE_BUILD_TYPE=Release `
            -DBUILD_SHARED_LIBS=OFF `
            -DCOMPILE_INTONATIONS=OFF `
            -DENABLE_TESTS=OFF `
            -DUSE_ASYNC=OFF `
            -DUSE_MBROLA=OFF `
            -DUSE_LIBSONIC=OFF `
            -DUSE_LIBPCAUDIO=OFF `
            -DUSE_KLATT=OFF `
            -DUSE_SPEECHPLAYER=OFF
        if ($LASTEXITCODE -ne 0) { Fail "espeak-ng cmake configure failed" }
    } else {
        Write-Step "reusing espeak-ng build directory: $espeakBuild"
    }

    Write-Step "building espeak-ng (this takes a few minutes)"
    & $cmake --build $espeakBuild --target espeak-ng --parallel
    if ($LASTEXITCODE -ne 0) { Fail "espeak-ng build failed" }

    $libespeak = Get-ChildItem $espeakBuild -Recurse -Filter "libespeak-ng.a" | Select-Object -First 1
    $libucd = Get-ChildItem $espeakBuild -Recurse -Filter "libucd.a" | Select-Object -First 1
    if (-not $libespeak) { Fail "libespeak-ng.a not found in $espeakBuild" }
    if (-not $libucd) { Fail "libucd.a not found in $espeakBuild" }
}
Write-Step "libespeak-ng.a: $($libespeak.FullName)"
Write-Step "libucd.a: $($libucd.FullName)"

$espeakInclude = Join-Path $EspeakSrcDir "src\include"
if (-not (Test-Path (Join-Path $espeakInclude "espeak-ng\speak_lib.h"))) {
    Fail "espeak-ng/speak_lib.h not found under $espeakInclude"
}


$piperTtsVersionUsed = $PiperTtsVersion
$dataDir = ""
if ($EspeakDataDir) {
    if (-not (Test-Path (Join-Path $EspeakDataDir "phontab"))) {
        Fail "espeak-ng-data not found: $EspeakDataDir"
    }
    $dataDir = (Resolve-Path $EspeakDataDir).Path
    Write-Step "using local espeak-ng-data: $dataDir"
} else {
    $cachedData = Join-Path $BuildDir "espeak-ng-data"
    if ((Test-Path (Join-Path $cachedData "phontab")) -and (-not $ForceFetch)) {
        $dataDir = $cachedData
        Write-Step "using cached espeak-ng-data: $dataDir"
    } else {
        Write-Step "resolving piper-tts wheel on PyPI (for espeak-ng-data)"
        $pypi = Invoke-RestMethod -Uri "https://pypi.org/pypi/piper-tts/json"
        if (-not $piperTtsVersionUsed) { $piperTtsVersionUsed = $pypi.info.version }
        $release = $pypi.releases.$piperTtsVersionUsed
        if (-not $release) { Fail "piper-tts $piperTtsVersionUsed not found on PyPI" }

        $wheels = $release | Where-Object { $_.filename -like "*.whl" }
        $wheel = $wheels | Where-Object { $_.filename -match "win_amd64" } | Select-Object -First 1
        if (-not $wheel) { $wheel = $wheels | Select-Object -First 1 }
        if (-not $wheel) { Fail "no wheel found for piper-tts $piperTtsVersionUsed" }

        $wheelPath = Join-Path $BuildDir $wheel.filename
        if ((-not (Test-Path $wheelPath)) -or $ForceFetch) {
            Write-Step "downloading $($wheel.filename)"
            Invoke-WebRequest -Uri $wheel.url -OutFile $wheelPath
        }

        Write-Step "extracting piper/espeak-ng-data from the wheel"
        if (Test-Path $cachedData) { Remove-Item -Recurse -Force $cachedData }
        New-Item -ItemType Directory -Force $cachedData | Out-Null
        Add-Type -AssemblyName System.IO.Compression.FileSystem
        $zip = [System.IO.Compression.ZipFile]::OpenRead($wheelPath)
        try {
            $prefix = "piper/espeak-ng-data/"
            $count = 0
            foreach ($entry in $zip.Entries) {
                if (-not $entry.FullName.StartsWith($prefix)) { continue }
                if ($entry.FullName.EndsWith("/")) { continue }
                $relative = $entry.FullName.Substring($prefix.Length)
                $dest = Join-Path $cachedData ($relative -replace "/", "\")
                $destParent = Split-Path -Parent $dest
                if (-not (Test-Path $destParent)) {
                    New-Item -ItemType Directory -Force $destParent | Out-Null
                }
                [System.IO.Compression.ZipFileExtensions]::ExtractToFile($entry, $dest, $true)
                $count++
            }
        } finally {
            $zip.Dispose()
        }
        if ($count -le 0) { Fail "espeak-ng-data not found in $($wheel.filename)" }
        Write-Step "extracted $count files"
        $dataDir = $cachedData
    }
}

$outJs = Join-Path $OutDir "piper_wasm.js"
$exportedFunctions = @(
    "_malloc",
    "_free",
    "_piper_wasm_create",
    "_piper_wasm_free",
    "_piper_wasm_start",
    "_piper_wasm_next_count",
    "_piper_wasm_next_copy",
    "_piper_wasm_sample_rate",
    "_piper_wasm_num_speakers",
    "_piper_wasm_version"
) -join "','"

Write-Step "compiling piper_wasm.js (phonemizer only, espeak-ng-data embedded)"
$emccArgs = @(
    (Join-Path $LibpiperDir "src\piper.cpp"),
    (Join-Path $LibpiperDir "src\chinese_phonemizer.cpp"),
    (Join-Path $SourceDir "piper_wasm_api.cpp"),
    "-std=c++17",
    "-I", (Join-Path $SourceDir "ort_stub"),
    "-I", (Join-Path $LibpiperDir "include"),
    "-I", $espeakInclude,
    "-o", $outJs,
    "--no-entry",
    "-s", "MODULARIZE=1",
    "-s", "EXPORT_NAME=PIPER",
    "-s", "SINGLE_FILE=1",
    "-s", "ALLOW_MEMORY_GROWTH=1",
    "-s", "INITIAL_MEMORY=67108864",
    "-s", "ENVIRONMENT=web",
    "-s", "FORCE_FILESYSTEM=1",
    "-s", "EXPORTED_FUNCTIONS=['$exportedFunctions']",
    "-s", "EXPORTED_RUNTIME_METHODS=['ccall','UTF8ToString','stringToUTF8','lengthBytesUTF8','HEAP32','HEAPU8']",
    "--embed-file", "$dataDir@/espeak-ng-data",
    "-Oz",
    "--closure", "0",
    $libespeak.FullName,
    $libucd.FullName
)
& $emcc @emccArgs
if ($LASTEXITCODE -ne 0) { Fail "emcc failed with exit code $LASTEXITCODE" }

$jsSize = [math]::Round((Get-Item $outJs).Length / 1MB, 1)
Write-Step "piper_wasm.js: $outJs ($jsSize MB)"

$ortDir = $OrtOutDir
if (-not $SkipOrt) {
    New-Item -ItemType Directory -Force $ortDir | Out-Null
    $ortBase = "https://cdn.jsdelivr.net/npm/onnxruntime-web@$OrtWebVersion/dist"
    $ortFiles = @(
        "ort.wasm.bundle.min.mjs",
        "ort-wasm-simd-threaded.wasm"
    )
    foreach ($name in $ortFiles) {
        $dest = Join-Path $ortDir $name
        if ((Test-Path $dest) -and (-not $ForceFetch)) {
            Write-Step "using existing $name"
            continue
        }
        Write-Step "downloading $name"
        Invoke-WebRequest -Uri "$ortBase/$name" -OutFile $dest
    }

    $licensePath = Join-Path $ortDir "LICENSE.txt"
    if ((-not (Test-Path $licensePath)) -or $ForceFetch) {
        try {
            $response = Invoke-WebRequest -Uri "https://cdn.jsdelivr.net/npm/onnxruntime-web@$OrtWebVersion/LICENSE"
            [System.IO.File]::WriteAllText($licensePath, $response.Content)
        } catch {
            $note = @(
                "ONNX Runtime Web $OrtWebVersion (https://github.com/microsoft/onnxruntime)",
                "Copyright (c) Microsoft Corporation. All rights reserved.",
                "Licensed under the MIT License.",
                "",
                "Files in this folder (ort.wasm.bundle.min.mjs, ort-wasm-simd-threaded.wasm)",
                "are the official prebuilt artifacts taken from the onnxruntime-web npm package."
            ) -join "`n"
            [System.IO.File]::WriteAllText($licensePath, $note)
        }
    }
}

if ($EspeakNgDir) { $espeakRefLabel = "local" } else { $espeakRefLabel = $EspeakNgCommit }
$info = [ordered]@{
    built_at          = (Get-Date).ToString("s")
    piper_tts         = $piperTtsVersionUsed
    libpiper          = $libpiperRefLabel
    libpiper_dir      = $LibpiperDir
    espeak_ng_commit  = $espeakRefLabel
    emscripten        = "$emccVersion"
    ort_web           = $OrtWebVersion
    piper_wasm_js_mb  = $jsSize
}
$info | ConvertTo-Json -Depth 4 | Set-Content -Encoding utf8 (Join-Path $OutDir "build_info.json")

Write-Step "SUCCESS"
Write-Step "  output:     $OutDir"
Write-Step "  ort:        $OrtOutDir"
Write-Step "  licenses:   $LicenseOutDir"
