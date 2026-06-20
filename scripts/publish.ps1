<#
.SYNOPSIS
    Build and publish a Velopack installer for Translator Desktop to GitHub Releases.

.DESCRIPTION
    Pipeline:
      1. Build the self-contained .NET text-helper sidecar.
      2. Compile the Tauri app (frontend + Rust) without Tauri's own bundler.
      3. Stage the app exe + sidecar into a clean folder.
      4. Run `vpk pack` to create the Velopack installer + update feed.
      5. Run `vpk upload github` to publish the release (and the in-app updater feed).

    Requires: dotnet, node/npm, the `vpk` dotnet tool, and a GitHub token
    (env GITHUB_TOKEN, or falls back to `gh auth token`).

.PARAMETER Version
    SemVer version to publish. Defaults to the "version" field in tauri.conf.json.

.PARAMETER Draft
    Create the GitHub release as a draft instead of publishing immediately.

.PARAMETER SkipBuild
    Reuse existing build output (skip sidecar + tauri compile). For re-packing/re-uploading.

.EXAMPLE
    pwsh -File scripts/publish.ps1
    pwsh -File scripts/publish.ps1 -Version 0.2.0
    pwsh -File scripts/publish.ps1 -Draft
#>
[CmdletBinding()]
param(
    [string]$Version,
    [switch]$Draft,
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"

# --- Paths ---------------------------------------------------------------
$RepoRoot   = Split-Path -Parent $PSScriptRoot
$TauriDir   = Join-Path $RepoRoot "src-tauri"
$ReleaseDir = Join-Path $TauriDir "target\release"
$SidecarBin = Join-Path $TauriDir "binaries\text-helper-x86_64-pc-windows-msvc.exe"
$IconPath   = Join-Path $TauriDir "icons\icon.ico"
$StageDir   = Join-Path $RepoRoot "target\velopack-stage"
$OutputDir  = Join-Path $RepoRoot "target\velopack-releases"

$PackId    = "TranslatorDesktop"
$MainExe   = "TranslatorDesktop.exe"
$RepoUrl   = "https://github.com/rockyway/translator-desktop"
$Authors   = "Tam Tran"

# --- Resolve version -----------------------------------------------------
if (-not $Version) {
    $conf = Get-Content (Join-Path $TauriDir "tauri.conf.json") -Raw | ConvertFrom-Json
    $Version = $conf.version
}
Write-Host "==> Publishing Translator Desktop v$Version" -ForegroundColor Cyan

# --- Resolve GitHub token ------------------------------------------------
$Token = $env:GITHUB_TOKEN
if (-not $Token) {
    try { $Token = (gh auth token).Trim() } catch { }
}
if (-not $Token) {
    throw "No GitHub token. Set `$env:GITHUB_TOKEN or run `gh auth login`."
}

# --- 1 & 2. Build --------------------------------------------------------
if (-not $SkipBuild) {
    Write-Host "==> Building sidecar..." -ForegroundColor Cyan
    & (Join-Path $PSScriptRoot "build-sidecar.ps1")
    if ($LASTEXITCODE -ne 0) { throw "Sidecar build failed." }

    Write-Host "==> Building Tauri app (no bundle)..." -ForegroundColor Cyan
    Push-Location $RepoRoot
    try {
        npx tauri build --no-bundle
        if ($LASTEXITCODE -ne 0) { throw "Tauri build failed." }
    } finally {
        Pop-Location
    }
}

# --- 3. Stage ------------------------------------------------------------
Write-Host "==> Staging app files..." -ForegroundColor Cyan

# Tauri names the compiled binary after the Cargo package ("translator-desktop").
$BuiltExe = @(
    (Join-Path $ReleaseDir "translator-desktop.exe"),
    (Join-Path $ReleaseDir "TranslatorDesktop.exe")
) | Where-Object { Test-Path $_ } | Select-Object -First 1

if (-not $BuiltExe) { throw "Built app exe not found in $ReleaseDir. Run without -SkipBuild." }
if (-not (Test-Path $SidecarBin)) { throw "Sidecar not found at $SidecarBin. Run without -SkipBuild." }

if (Test-Path $StageDir) { Remove-Item -Recurse -Force $StageDir }
New-Item -ItemType Directory -Path $StageDir -Force | Out-Null

# Main executable (renamed to a friendly installed name).
Copy-Item -Force $BuiltExe (Join-Path $StageDir $MainExe)
# Sidecar must sit next to the main exe as "text-helper.exe" (Tauri strips the triple at runtime).
Copy-Item -Force $SidecarBin (Join-Path $StageDir "text-helper.exe")
# WebView2Loader.dll if Tauri emitted one (statically linked on modern Tauri, so usually absent).
$Loader = Join-Path $ReleaseDir "WebView2Loader.dll"
if (Test-Path $Loader) { Copy-Item -Force $Loader $StageDir }

# --- 4. Pack -------------------------------------------------------------
Write-Host "==> Packing Velopack release..." -ForegroundColor Cyan
$packArgs = @(
    "pack",
    "--packId",      $PackId,
    "--packVersion", $Version,
    "--packDir",     $StageDir,
    "--mainExe",     $MainExe,
    "--packTitle",   "Translator Desktop",
    "--packAuthors", $Authors,
    "--outputDir",   $OutputDir
)
if (Test-Path $IconPath) { $packArgs += @("--icon", $IconPath) }
vpk @packArgs
if ($LASTEXITCODE -ne 0) { throw "vpk pack failed." }

# --- 5. Upload -----------------------------------------------------------
Write-Host "==> Uploading to GitHub Releases..." -ForegroundColor Cyan
$uploadArgs = @(
    "upload", "github",
    "--repoUrl",   $RepoUrl,
    "--token",     $Token,
    "--outputDir", $OutputDir,
    "--merge"
)
if (-not $Draft) { $uploadArgs += "--publish" }
vpk @uploadArgs
if ($LASTEXITCODE -ne 0) { throw "vpk upload failed." }

Write-Host ""
Write-Host "Done. Published Translator Desktop v$Version" -ForegroundColor Green
if ($Draft) {
    Write-Host "Release created as DRAFT - finalize it on GitHub to go live." -ForegroundColor Yellow
}
