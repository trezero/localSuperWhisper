# Windows Installer Guide

This guide explains how to build and distribute a professional Windows installer for Local SuperWhisper.

## Overview

The app now uses **Tauri's built-in NSIS installer** which provides:

- ✅ Professional installer wizard with custom branding
- ✅ Start menu shortcuts
- ✅ Desktop shortcut option (user choice during install)
- ✅ Runs in system tray (no visible window on startup)
- ✅ Proper uninstaller via Windows Settings
- ✅ Per-user installation (no admin rights required)
- ✅ Optional code signing for trusted installation

## Building the Installer

### Quick Build

Run the build script from the project root:

```powershell
.\build-installer.ps1
```

This will:
1. Check prerequisites (Node.js, Rust)
2. Install dependencies
3. Build the frontend (React + Vite)
4. Build the Tauri app and create the installer
5. Display the output file location

### Build Options

```powershell
# Clean build (removes previous builds first)
.\build-installer.ps1 -Clean

# Skip npm install (if dependencies already installed)
.\build-installer.ps1 -SkipInstall

# Both options
.\build-installer.ps1 -Clean -SkipInstall
```

### Manual Build

If you prefer to build manually:

```powershell
# Install dependencies
npm install

# Build frontend
npm run build

# Build Tauri app with installer
npm run tauri build
```

## Output Files

The installer will be created at:

```
src-tauri/target/release/bundle/nsis/Local SuperWhisper_0.1.0_x64-setup.exe
```

Additional formats:
- **MSI installer**: `src-tauri/target/release/bundle/msi/Local SuperWhisper_0.1.0_x64_en-US.msi`
- **Portable executable**: `src-tauri/target/release/local-super-whisper.exe`

## Installer Features

### What the Installer Does

1. **Installs to User Directory**
   - Default: `%LOCALAPPDATA%\Programs\Local SuperWhisper\`
   - No admin rights required

2. **Creates Start Menu Entry**
   - Location: `Start Menu > Local SuperWhisper`
   - Includes both app launcher and uninstaller

3. **Optional Desktop Shortcut**
   - User is asked during installation
   - Can be created or skipped

4. **Runs After Installation**
   - App starts automatically after install completes
   - Launches minimized to system tray

5. **Uninstaller**
   - Accessible via Windows Settings > Apps
   - Or via Start Menu > Local SuperWhisper > Uninstall

### What Happens on First Run

1. App launches to system tray (no visible window)
2. User clicks tray icon to open settings
3. First-run setup screen appears (hotkey configuration)
4. User configures hotkey, API endpoint, and microphone
5. App is ready to use

## End-User Experience

### Installation

1. User downloads the `.exe` installer
2. Double-clicks to run (may see Windows SmartScreen warning if unsigned)
3. Follows installer wizard:
   - Accepts license
   - Chooses desktop shortcut (optional)
   - Clicks Install
4. App launches automatically to system tray

### Daily Use

1. App runs in system tray (microphone icon)
2. Click tray icon to open settings
3. Press configured hotkey to start/stop recording
4. Transcribed text is automatically pasted

### Auto-Start on Windows Login

The installer does NOT automatically enable startup on login. Users can enable this manually:

**Option 1: Windows Settings**
1. Open Windows Settings > Apps > Startup
2. Find "Local SuperWhisper"
3. Toggle to "On"

**Option 2: Task Manager**
1. Open Task Manager (Ctrl+Shift+Esc)
2. Go to "Startup" tab
3. Right-click "Local SuperWhisper" > Enable

**Option 3: Manual Shortcut**
1. Press Win+R, type `shell:startup`, press Enter
2. Create a shortcut to the app in this folder

### Uninstallation

**Option 1: Windows Settings**
1. Open Windows Settings > Apps > Installed apps
2. Find "Local SuperWhisper"
3. Click three dots > Uninstall

**Option 2: Start Menu**
1. Open Start Menu
2. Find "Local SuperWhisper" folder
3. Click "Uninstall Local SuperWhisper"

## Code Signing (Optional but Recommended)

Code signing prevents Windows SmartScreen warnings and builds user trust.

### Why Sign?

- **No SmartScreen warnings**: Users won't see "Windows protected your PC" messages
- **Publisher verification**: Shows your organization name in the installer
- **User trust**: Signed apps appear more professional and trustworthy

### How to Sign

1. **Obtain a Code Signing Certificate**
   - Purchase from: DigiCert, Sectigo, GlobalSign, etc.
   - Cost: ~$100-400/year
   - Requires business verification

2. **Configure Tauri for Signing**

   Edit `src-tauri/tauri.conf.json`:

   ```json
   "windows": {
     "certificateThumbprint": "YOUR_CERT_THUMBPRINT_HERE",
     "digestAlgorithm": "sha256",
     "timestampUrl": "http://timestamp.digicert.com"
   }
   ```

3. **Build with Signing**

   The build process will automatically sign the installer if a certificate is configured.

### Finding Your Certificate Thumbprint

```powershell
# List all code signing certificates
Get-ChildItem -Path Cert:\CurrentUser\My -CodeSigningCert

# Copy the Thumbprint value to tauri.conf.json
```

## Distribution

### Recommended Distribution Methods

1. **Direct Download**
   - Host the `.exe` file on your website
   - Provide SHA256 checksum for verification

2. **GitHub Releases**
   - Upload installer as a release asset
   - Users can download from GitHub

3. **Cloud Storage**
   - Google Drive, Dropbox, OneDrive
   - Share direct download link

### File Naming Convention

The installer follows this pattern:
```
Local SuperWhisper_[VERSION]_x64-setup.exe
```

Example: `Local SuperWhisper_0.1.0_x64-setup.exe`

## Troubleshooting

### Windows SmartScreen Warning

**Symptom**: "Windows protected your PC" message when running installer

**Cause**: Installer is not code-signed

**Solutions**:
1. Click "More info" > "Run anyway" (for testing)
2. Sign the installer with a code signing certificate (for distribution)

### Installer Fails to Run

**Symptom**: Installer crashes or shows error

**Checks**:
1. Ensure Windows 10/11 (64-bit)
2. Check antivirus isn't blocking it
3. Run as administrator (if needed)

### App Doesn't Start After Install

**Symptom**: No system tray icon appears

**Checks**:
1. Check Task Manager for running process
2. Look in `%LOCALAPPDATA%\Programs\Local SuperWhisper\`
3. Try running `local-super-whisper.exe` directly
4. Check logs in `%APPDATA%\com.localsuperwhisper.app\`

## Differences from PM2 Deployment

### Old Way (PM2 - Developer Experience)
- Required Node.js and PM2 installed
- Manual `manage.sh` or `manage.ps1` scripts
- Separate build and deployment steps
- Startup configuration via PM2 commands
- Developer-focused workflow

### New Way (Installer - End-User Experience)
- Single `.exe` installer file
- No dependencies (everything bundled)
- One-click installation
- Native Windows integration
- Professional end-user experience

## Version Updates

To release a new version:

1. Update version in `package.json`
2. Update version in `src-tauri/Cargo.toml`
3. Update version in `src-tauri/tauri.conf.json`
4. Build new installer
5. Distribute to users

Users will need to:
1. Uninstall old version (or install over it)
2. Install new version

### Future: Automatic Updates

Tauri supports automatic updates via the `tauri-plugin-updater`. This can be added later to enable:
- Background update checks
- One-click update installation
- No manual uninstall/reinstall needed

## Testing Checklist

Before distributing the installer:

- [ ] Install on clean Windows 10 machine
- [ ] Install on clean Windows 11 machine
- [ ] Verify system tray icon appears
- [ ] Verify settings window opens from tray
- [ ] Test hotkey registration
- [ ] Test recording and transcription
- [ ] Verify Start Menu shortcut works
- [ ] Test uninstaller removes all files
- [ ] Check for leftover registry entries after uninstall
- [ ] Verify app data is preserved across reinstalls

## Support Resources

- **Tauri Documentation**: https://tauri.app/v2/
- **NSIS Documentation**: https://nsis.sourceforge.io/Docs/
- **Code Signing Guide**: https://tauri.app/v2/guides/distribution/sign-windows/
