# Build the .NET Text Monitor as a self-contained sidecar for Tauri
# This script builds the .NET project and copies the executable to the Tauri binaries folder

$ErrorActionPreference = "Stop"

$ProjectRoot = Split-Path -Parent $PSScriptRoot
if (-not $ProjectRoot) {
    $ProjectRoot = (Get-Location).Path
}

$DotNetProject = Join-Path $ProjectRoot "text-monitor\TextMonitor.Service"
$OutputDir = Join-Path $ProjectRoot "src-tauri\binaries"

Write-Host "Building .NET Text Monitor sidecar..." -ForegroundColor Cyan
Write-Host "Project: $DotNetProject"
Write-Host "Output: $OutputDir"

# Clean output directory
if (Test-Path $OutputDir) {
    Remove-Item -Recurse -Force $OutputDir
}
New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null

# Build as single-file self-contained executable
Push-Location $DotNetProject
try {
    dotnet publish `
        -c Release `
        -r win-x64 `
        --self-contained `
        -p:PublishSingleFile=true `
        -p:IncludeNativeLibrariesForSelfExtract=true `
        -o $OutputDir

    if ($LASTEXITCODE -ne 0) {
        throw "dotnet publish failed with exit code $LASTEXITCODE"
    }
}
finally {
    Pop-Location
}

# Rename to Tauri sidecar naming convention
$SourceExe = Join-Path $OutputDir "TextMonitor.Service.exe"
$TargetExe = Join-Path $OutputDir "text-monitor-x86_64-pc-windows-msvc.exe"

if (Test-Path $SourceExe) {
    Move-Item -Force $SourceExe $TargetExe
    Write-Host "Renamed to: text-monitor-x86_64-pc-windows-msvc.exe" -ForegroundColor Green
}

# Clean up PDB file
$PdbFile = Join-Path $OutputDir "TextMonitor.Service.pdb"
if (Test-Path $PdbFile) {
    Remove-Item $PdbFile
}

# Copy to src-tauri root for bundling (externalBin: "text-monitor")
$BundleDir = Join-Path $ProjectRoot "src-tauri"
Copy-Item -Force $TargetExe $BundleDir
Write-Host "Copied to bundle location: $BundleDir" -ForegroundColor Cyan

# Copy to target/debug for development mode
$DevOutputDir = Join-Path $ProjectRoot "src-tauri\target\debug"
if (-not (Test-Path $DevOutputDir)) {
    New-Item -ItemType Directory -Path $DevOutputDir -Force | Out-Null
}
Copy-Item -Force $TargetExe $DevOutputDir
Write-Host "Copied to dev location: $DevOutputDir" -ForegroundColor Cyan

Write-Host ""
Write-Host "Sidecar build complete!" -ForegroundColor Green
Write-Host "Binary: $TargetExe"
Write-Host "Size: $([math]::Round((Get-Item $TargetExe).Length / 1MB, 2)) MB"
