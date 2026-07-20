# Deployment Summary: Developer vs End-User Experience

## Overview

Local SuperWhisper now supports **two deployment modes**:

1. **Developer Mode** - PM2-based process management (existing)
2. **End-User Mode** - Windows installer with native integration (new)

## Comparison

| Feature | Developer Mode (PM2) | End-User Mode (Installer) |
|---------|---------------------|---------------------------|
| **Target Audience** | Developers, power users | General end users |
| **Installation** | Manual setup, multiple steps | Single `.exe` installer |
| **Dependencies** | Node.js, PM2, Rust | None (all bundled) |
| **Startup** | PM2 commands or scripts | Runs from Start Menu |
| **Auto-start** | PM2 startup configuration | Windows Settings toggle |
| **Updates** | Git pull + rebuild | Download new installer |
| **Uninstall** | Manual file deletion | Windows Apps & Features |
| **System Integration** | Limited | Full (Start Menu, tray, etc.) |
| **Distribution** | Git repository | Single executable file |

## When to Use Each Mode

### Use Developer Mode (PM2) When:
- You're actively developing the application
- You need hot-reload and debugging capabilities
- You want to modify the source code
- You're testing on Linux or WSL2
- You need process management features (logs, restart, etc.)

### Use End-User Mode (Installer) When:
- Distributing to non-technical users
- You want a professional installation experience
- Users shouldn't need to know about Node.js or Rust
- You want native Windows integration
- You're ready for production deployment

## Migration Path

### From Developer to End-User

If you've been running in developer mode and want to switch to the installer:

1. **Stop PM2 process:**
   ```powershell
   pm2 stop localSuperWhisper
   pm2 delete localSuperWhisper
   pm2 unstartup  # Remove from Windows startup
   ```

2. **Build the installer:**
   ```powershell
   .\build-installer.ps1
   ```

3. **Install the application:**
   - Run the generated `.exe` installer
   - Your settings and data will be preserved (same database location)

4. **Clean up (optional):**
   - Remove PM2 if not needed: `npm uninstall -g pm2`
   - Keep the source code for future development

### From End-User to Developer

If you want to develop after installing via the installer:

1. **Uninstall the application** (via Windows Settings)
2. **Clone the repository** (if not already present)
3. **Install dependencies:** `npm install`
4. **Run in dev mode:** `npm run tauri dev`

Your settings and data are preserved in `%APPDATA%\com.localsuperwhisper.app\`

## File Locations

Both modes use the same data directory, so settings are preserved:

- **Database:** `%APPDATA%\com.localsuperwhisper.app\local_super_whisper.db`
- **Logs:** `%APPDATA%\com.localsuperwhisper.app\logs\`
- **Sounds:** Bundled in the application

### Installer Mode Additional Locations:
- **Executable:** `%LOCALAPPDATA%\Programs\Local SuperWhisper\`
- **Start Menu:** `Start Menu > Local SuperWhisper`
- **Uninstaller:** `%LOCALAPPDATA%\Programs\Local SuperWhisper\Uninstall.exe`

### Developer Mode Additional Locations:
- **Source Code:** Your git repository location
- **PM2 Logs:** `logs/app-err.log`, `logs/app-out.log`
- **PM2 Config:** `ecosystem.config.cjs`

## Building for Distribution

### Quick Build

```powershell
.\build-installer.ps1
```

Output: `src-tauri\target\release\bundle\nsis\Local SuperWhisper_0.1.0_x64-setup.exe`

### Build Options

```powershell
# Clean build
.\build-installer.ps1 -Clean

# Skip npm install
.\build-installer.ps1 -SkipInstall
```

### What Gets Bundled

The installer includes:
- ✅ Compiled Rust backend
- ✅ React frontend (built)
- ✅ All dependencies (no external requirements)
- ✅ Sound files
- ✅ Icons and assets
- ✅ Uninstaller

The installer does NOT include:
- ❌ Source code
- ❌ Node.js or Rust toolchains
- ❌ Development dependencies
- ❌ Git history

## Distribution Checklist

Before distributing the installer to end users:

- [ ] Update version numbers in all config files
- [ ] Test on clean Windows 10 machine
- [ ] Test on clean Windows 11 machine
- [ ] Verify all features work (recording, transcription, paste)
- [ ] Test uninstaller removes all files
- [ ] Create installer images (optional but recommended)
- [ ] Sign the installer (optional but recommended)
- [ ] Write release notes
- [ ] Update documentation
- [ ] Create SHA256 checksum for verification

## Support Considerations

### Developer Mode Support
- Users need technical knowledge
- Can debug via PM2 logs and commands
- Can modify source code
- Updates via git pull

### End-User Mode Support
- Users expect "it just works"
- Limited debugging options for end users
- Cannot modify without rebuilding
- Updates via new installer download

## Recommended Workflow

### For Development Team:
1. Develop using `npm run tauri dev` (hot reload)
2. Test using PM2 deployment (`./manage.ps1 redeploy`)
3. Build installer for release (`.\build-installer.ps1`)
4. Test installer on clean machines
5. Distribute installer to end users

### For End Users:
1. Download installer
2. Run installer
3. Configure on first launch
4. Use daily via system tray
5. Update by downloading new installer

## Future Enhancements

Potential improvements for end-user experience:

- **Automatic updates** - Using `tauri-plugin-updater`
- **In-app auto-start toggle** - Using `tauri-plugin-autostart`
- **Crash reporting** - Using Sentry or similar
- **Usage analytics** - Optional telemetry
- **Multi-language support** - Internationalization
- **Custom installer themes** - Branded installer images

## Documentation

- **[Windows Installer Guide](WINDOWS_INSTALLER.md)** - Complete installer documentation
- **[Auto-Start Guide](AUTO_START.md)** - Configuring Windows startup
- **[Installer Images](INSTALLER_IMAGES.md)** - Creating custom branding
- **[README.md](../README.md)** - Main project documentation

## Questions?

- **For development questions:** See README.md and manage.ps1/manage.sh
- **For distribution questions:** See WINDOWS_INSTALLER.md
- **For end-user support:** See AUTO_START.md and troubleshooting sections
