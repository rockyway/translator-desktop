#!/bin/bash
# Build the .NET Text Monitor as a self-contained sidecar for Tauri
# This script builds the .NET project and copies the executable to the Tauri binaries folder

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

DOTNET_PROJECT="$PROJECT_ROOT/text-monitor/TextMonitor.Service"
OUTPUT_DIR="$PROJECT_ROOT/src-tauri/binaries"

echo "Building .NET Text Monitor sidecar..."
echo "Project: $DOTNET_PROJECT"
echo "Output: $OUTPUT_DIR"

# Clean output directory
rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR"

# Build as single-file self-contained executable
cd "$DOTNET_PROJECT"
dotnet publish \
    -c Release \
    -r win-x64 \
    --self-contained \
    -p:PublishSingleFile=true \
    -p:IncludeNativeLibrariesForSelfExtract=true \
    -o "$OUTPUT_DIR"

# Rename to Tauri sidecar naming convention
if [ -f "$OUTPUT_DIR/TextMonitor.Service.exe" ]; then
    mv "$OUTPUT_DIR/TextMonitor.Service.exe" "$OUTPUT_DIR/text-monitor-x86_64-pc-windows-msvc.exe"
    echo "Renamed to: text-monitor-x86_64-pc-windows-msvc.exe"
fi

# Clean up PDB file
rm -f "$OUTPUT_DIR/TextMonitor.Service.pdb"

echo ""
echo "Sidecar build complete!"
echo "Binary: $OUTPUT_DIR/text-monitor-x86_64-pc-windows-msvc.exe"
