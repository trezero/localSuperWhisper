# Local SuperWhisper - Windows Installer Build Script
# This script builds a production-ready NSIS installer for Windows

param(
    [switch]$SkipInstall,
    [switch]$Clean
)

$ErrorActionPreference = "Stop"

Write-Host "================================================" -ForegroundColor Cyan
Write-Host "  Local SuperWhisper - Installer Build Script  " -ForegroundColor Cyan
Write-Host "================================================" -ForegroundColor Cyan
Write-Host ""

# Check prerequisites
Write-Host "[1/6] Checking prerequisites..." -ForegroundColor Yellow

$nodeVersion = node --version 2>$null
if (-not $nodeVersion) {
    Write-Host "ERROR: Node.js is not installed or not in PATH" -ForegroundColor Red
    exit 1
}
Write-Host "  âœ“ Node.js: $nodeVersion" -ForegroundColor Green

$rustVersion = rustc --version 2>$null
if (-not $rustVersion) {
    Write-Host "ERROR: Rust is not installed or not in PATH" -ForegroundColor Red
    Write-Host "  Install from: https://rustup.rs" -ForegroundColor Yellow
    exit 1
}
Write-Host "  âœ“ Rust: $rustVersion" -ForegroundColor Green

# Clean previous builds if requested
if ($Clean) {
    Write-Host ""
    Write-Host "[2/6] Cleaning previous builds..." -ForegroundColor Yellow
    if (Test-Path "dist") {
        Remove-Item -Recurse -Force "dist"
        Write-Host "  âœ“ Removed dist/" -ForegroundColor Green
    }
    if (Test-Path "src-tauri\target\release") {
        Remove-Item -Recurse -Force "src-tauri\target\release"
        Write-Host "  âœ“ Removed src-tauri\target\release/" -ForegroundColor Green
    }
} else {
    Write-Host ""
    Write-Host "[2/6] Skipping clean (use -Clean to clean previous builds)" -ForegroundColor Gray
}

# Install dependencies
if (-not $SkipInstall) {
    Write-Host ""
    Write-Host "[3/6] Installing dependencies..." -ForegroundColor Yellow
    npm install
    if ($LASTEXITCODE -ne 0) {
        Write-Host "ERROR: npm install failed" -ForegroundColor Red
        exit 1
    }
    Write-Host "  âœ“ Dependencies installed" -ForegroundColor Green
} else {
    Write-Host ""
    Write-Host "[3/6] Skipping dependency installation (use without -SkipInstall to install)" -ForegroundColor Gray
}

# Build frontend
Write-Host ""
Write-Host "[4/6] Building frontend..." -ForegroundColor Yellow
npm run build
if ($LASTEXITCODE -ne 0) {
    Write-Host "ERROR: Frontend build failed" -ForegroundColor Red
    exit 1
}
Write-Host "  âœ“ Frontend built successfully" -ForegroundColor Green

# Build Tauri app with NSIS installer
Write-Host ""
Write-Host "[5/6] Building Tauri application and installer..." -ForegroundColor Yellow
Write-Host "  This may take several minutes..." -ForegroundColor Gray
npm run tauri build
if ($LASTEXITCODE -ne 0) {
    Write-Host "ERROR: Tauri build failed" -ForegroundColor Red
    exit 1
}
Write-Host "  âœ“ Application and installer built successfully" -ForegroundColor Green

# Find and display output files
Write-Host ""
Write-Host "[6/6] Build complete!" -ForegroundColor Yellow
Write-Host ""
Write-Host "Output files:" -ForegroundColor Cyan

$nsisPath = "src-tauri\target\release\bundle\nsis"
if (Test-Path $nsisPath) {
    $installers = Get-ChildItem -Path $nsisPath -Filter "*.exe" | Sort-Object LastWriteTime -Descending
    foreach ($installer in $installers) {
        $sizeMB = [math]::Round($installer.Length / 1MB, 2)
        Write-Host "  ðŸ“¦ $($installer.Name)" -ForegroundColor Green
        Write-Host "     Location: $($installer.FullName)" -ForegroundColor Gray
        Write-Host "     Size: $sizeMB MB" -ForegroundColor Gray
    }
} else {
    Write-Host "  âš  NSIS installer not found at: $nsisPath" -ForegroundColor Yellow
}

$msiPath = "src-tauri\target\release\bundle\msi"
if (Test-Path $msiPath) {
    $msiInstallers = Get-ChildItem -Path $msiPath -Filter "*.msi" | Sort-Object LastWriteTime -Descending
    foreach ($msi in $msiInstallers) {
        $sizeMB = [math]::Round($msi.Length / 1MB, 2)
        Write-Host "  ðŸ“¦ $($msi.Name)" -ForegroundColor Green
        Write-Host "     Location: $($msi.FullName)" -ForegroundColor Gray
        Write-Host "     Size: $sizeMB MB" -ForegroundColor Gray
    }
}

Write-Host ""
Write-Host "================================================" -ForegroundColor Cyan
Write-Host "  Build completed successfully!                 " -ForegroundColor Cyan
Write-Host "================================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Next steps:" -ForegroundColor Yellow
Write-Host "  1. Test the installer on a clean Windows machine" -ForegroundColor White
Write-Host "  2. (Optional) Sign the installer with a code signing certificate" -ForegroundColor White
Write-Host "  3. Distribute the installer to users" -ForegroundColor White
Write-Host ""

