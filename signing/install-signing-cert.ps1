param([string]$PfxPath = (Join-Path $PSScriptRoot 'cert.pfx'))

# Imports the build-time signing cert (cert.pfx) into the current user's
# personal cert store so `tauri build` can find it by thumbprint.
#
# Run this ONCE on every machine that builds the installer.
# Does NOT require admin (CurrentUser\My is per-user).

$ErrorActionPreference = 'Stop'

if (-not (Test-Path $PfxPath)) {
    Write-Error "PFX not found: $PfxPath"
    exit 1
}

$emptyPwd = New-Object System.Security.SecureString
$cert = Import-PfxCertificate `
    -FilePath $PfxPath `
    -CertStoreLocation Cert:\CurrentUser\My `
    -Password $emptyPwd `
    -Exportable

Write-Host "Imported into Cert:\CurrentUser\My"
Write-Host "  Subject:    $($cert.Subject)"
Write-Host "  Thumbprint: $($cert.Thumbprint)"
Write-Host "  NotAfter:   $($cert.NotAfter)"
Write-Host ""
Write-Host "Build pipeline is ready. Run: npm run tauri -- build"
