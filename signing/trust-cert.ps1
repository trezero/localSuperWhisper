param([string]$CerPath = (Join-Path $PSScriptRoot 'cert.cer'))

$ErrorActionPreference = 'Stop'
$logPath = Join-Path $PSScriptRoot 'trust-cert.log'
Start-Transcript -Path $logPath -Force | Out-Null

try {
    if (-not (Test-Path $CerPath)) {
        throw "Certificate not found: $CerPath"
    }

    Write-Host "Importing $CerPath into LocalMachine stores..."

    $root = Import-Certificate -FilePath $CerPath -CertStoreLocation Cert:\LocalMachine\Root
    Write-Host "  Trusted Root:        $($root.Thumbprint)"

    $pub = Import-Certificate -FilePath $CerPath -CertStoreLocation Cert:\LocalMachine\TrustedPublisher
    Write-Host "  Trusted Publisher:   $($pub.Thumbprint)"

    Write-Host ""
    Write-Host "Done. The signing cert is now trusted on this machine."
}
catch {
    Write-Host "ERROR: $_"
    Stop-Transcript | Out-Null
    exit 1
}

Stop-Transcript | Out-Null
