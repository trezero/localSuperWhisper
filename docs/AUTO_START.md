# Auto-Start on Windows Login

This guide explains how to configure Local SuperWhisper to start automatically when Windows boots.

## For End Users

After installing Local SuperWhisper, you can enable auto-start using any of these methods:

### Method 1: Windows Settings (Recommended)

1. Open **Windows Settings** (Win + I)
2. Go to **Apps** > **Startup**
3. Find **Local SuperWhisper** in the list
4. Toggle the switch to **On**

### Method 2: Task Manager

1. Open **Task Manager** (Ctrl + Shift + Esc)
2. Click the **Startup** tab
3. Find **Local SuperWhisper** in the list
4. Right-click and select **Enable**

### Method 3: Manual Startup Folder

1. Press **Win + R** to open Run dialog
2. Type `shell:startup` and press Enter
3. Create a shortcut to Local SuperWhisper in this folder
   - Right-click > New > Shortcut
   - Browse to: `%LOCALAPPDATA%\Programs\Local SuperWhisper\local-super-whisper.exe`
   - Name it "Local SuperWhisper"

## For Developers

If you want to add auto-start functionality directly into the installer, you can configure it in the NSIS installer settings.

### Option A: Registry-Based Auto-Start

Add this to `src-tauri/tauri.conf.json` under the `nsis` section:

```json
"nsis": {
  "installerHooks": "./installer-hooks",
  ...
}
```

Then create `src-tauri/installer-hooks/install.nsh`:

```nsis
!macro customInstall
  ; Add registry key for auto-start
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "LocalSuperWhisper" "$INSTDIR\local-super-whisper.exe"
!macroend
```

And `src-tauri/installer-hooks/uninstall.nsh`:

```nsis
!macro customUninstall
  ; Remove registry key
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "LocalSuperWhisper"
!macroend
```

### Option B: Tauri Plugin (Future)

Tauri has a community plugin for auto-start functionality:

```bash
npm install tauri-plugin-autostart
```

This provides programmatic control over auto-start from within the app, allowing users to toggle it in the settings UI.

## Verification

To verify auto-start is working:

1. Enable auto-start using one of the methods above
2. Restart your computer
3. After login, check the system tray for the Local SuperWhisper icon
4. If the icon appears, auto-start is working correctly

## Troubleshooting

### App doesn't start on login

**Check Task Manager Startup tab:**
- Ensure Local SuperWhisper is listed and enabled
- Check the "Status" column - it should say "Enabled"

**Check Registry (if using registry method):**
1. Press Win + R, type `regedit`, press Enter
2. Navigate to: `HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run`
3. Look for "LocalSuperWhisper" entry
4. Value should point to the correct executable path

**Check Startup folder:**
1. Press Win + R, type `shell:startup`, press Enter
2. Look for Local SuperWhisper shortcut
3. Right-click > Properties to verify the target path is correct

### App starts but crashes immediately

**Check logs:**
- Location: `%APPDATA%\com.localsuperwhisper.app\logs\`
- Look for error messages in the log files

**Check dependencies:**
- Ensure the Faster-Whisper server is accessible
- Verify API URL is configured correctly in settings

## Best Practices

### For End Users
- **Recommended**: Use Windows Settings or Task Manager to enable auto-start
- This gives you full control and visibility
- Easy to disable if needed

### For Developers/Distributors
- **Don't force auto-start**: Let users choose during or after installation
- **Provide clear documentation**: Explain how to enable/disable auto-start
- **Test thoroughly**: Ensure auto-start works across Windows 10 and 11
- **Handle failures gracefully**: If the app can't start, don't crash silently

## Security Considerations

- Auto-start entries can be modified by malware
- Users should verify the executable path points to the legitimate installation
- Code signing helps users verify the app's authenticity
- Regular security updates are important for auto-start applications

## Related Documentation

- [Windows Installer Guide](WINDOWS_INSTALLER.md) - Building and distributing the installer
- [Tauri Auto-Start Plugin](https://github.com/tauri-apps/tauri-plugin-autostart) - Official plugin documentation
