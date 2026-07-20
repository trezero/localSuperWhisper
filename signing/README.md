# Signing

Self-signed Authenticode setup for Local SuperWhisper Windows installers.
**Internal/dev distribution only** — not a substitute for a commercial OV/EV
cert if you ever ship publicly.

## What's in this folder

| File | Purpose | Commit? |
|---|---|---|
| `cert.pfx` | Private key + cert. Used by build machines to sign installers. | ❌ gitignored |
| `cert.cer` | Public cert only. Distributed to end-user machines for trust. | ❌ gitignored |
| `thumbprint.txt` | SHA-1 thumbprint, also hardcoded in `src-tauri/tauri.conf.json`. | ❌ gitignored |
| `install-signing-cert.ps1` | Imports `cert.pfx` on a build machine (per-user, no admin). | ✅ |
| `trust-cert.ps1` | Imports `cert.cer` into Trusted Root + Trusted Publisher (admin). | ✅ |

The cert files are gitignored — share them via 1Password / a secure file
share / Slack DM, **never** through the repo.

## Cert details

- **Subject:** `CN=Local SuperWhisper Dev, O=Workflow Intelligence Nexus, C=US`
- **Thumbprint:** `64A2BBF8CE5DE6FE0F16B55B227B5151044CC4CE`
- **Issued:** 2026-05-07
- **Expires:** 2029-05-07
- **Algorithm:** RSA 2048 / SHA-256
- **Timestamp authority:** `http://timestamp.digicert.com`

When the cert expires, generate a new one and update both
`src-tauri/tauri.conf.json` and `thumbprint.txt`.

## Build machine setup (one-time per machine)

Drop `cert.pfx` into this folder, then:

```powershell
.\signing\install-signing-cert.ps1
```

This imports the cert into `Cert:\CurrentUser\My`. From now on:

```powershell
npm run tauri -- build
```

…will sign the NSIS `.exe` and the `.msi` automatically — Tauri reads the
thumbprint from `tauri.conf.json` and looks the cert up in your store.

## End-user machine setup (one-time per user)

Distribute `cert.cer` (public, safe to share) plus this command. Users run an
elevated PowerShell:

```powershell
.\signing\trust-cert.ps1
```

This imports the public cert into `Cert:\LocalMachine\Root` and
`Cert:\LocalMachine\TrustedPublisher`. After that, signed installers run
without any SmartScreen warning on that machine.

## What signing does and doesn't do

✅ Removes the silent SmartScreen block on internal team machines that have
   `cert.cer` trusted.
✅ Removes the "Unknown publisher" UAC label — the prompt now shows
   "Local SuperWhisper Dev" as the verified publisher.
✅ Lets `cargo build --release --features custom-protocol` produce binaries
   that don't trip Defender's machine-learning heuristics quite as often.

❌ Does **not** clear SmartScreen on machines that haven't trusted the cert.
   For public distribution you need a commercial OV (with reputation warm-up)
   or EV (immediate) cert, or Azure Trusted Signing.
❌ Does **not** make the binary trustworthy on its own — the trust depends on
   end-user machines importing `cert.cer`.

## Rotating the cert

```powershell
$cert = New-SelfSignedCertificate `
    -Type CodeSigningCert `
    -Subject 'CN=Local SuperWhisper Dev, O=Workflow Intelligence Nexus, C=US' `
    -KeyAlgorithm RSA -KeyLength 2048 -HashAlgorithm SHA256 `
    -KeyUsage DigitalSignature -KeyExportPolicy Exportable `
    -CertStoreLocation Cert:\CurrentUser\My `
    -NotAfter (Get-Date).AddYears(3) `
    -FriendlyName 'Local SuperWhisper Dev Signing'

$pwd = New-Object System.Security.SecureString
Export-PfxCertificate -Cert $cert -FilePath .\signing\cert.pfx -Password $pwd
Export-Certificate    -Cert $cert -FilePath .\signing\cert.cer -Type CERT
$cert.Thumbprint | Out-File .\signing\thumbprint.txt -Encoding ascii -NoNewline
```

Then update `certificateThumbprint` in `src-tauri/tauri.conf.json` and
re-distribute the new `cert.cer` to the team.
