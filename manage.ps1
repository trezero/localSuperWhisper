# manage.ps1 — Local SuperWhisper process manager (Windows PowerShell)
# Requires: npm install -g pm2  &&  npm install -g pm2-windows-startup
#
# Usage:
#   .\manage.ps1              # interactive menu
#   .\manage.ps1 start
#   .\manage.ps1 stop
#   .\manage.ps1 restart
#   .\manage.ps1 logs
#   .\manage.ps1 redeploy
#   .\manage.ps1 status
#   .\manage.ps1 startup
#   .\manage.ps1 uninstall    # clean slate (PM2 + optional appdata/install)

$ErrorActionPreference = "Stop"

$ScriptDir  = $PSScriptRoot
$AppName    = "localSuperWhisper"
$Exe        = Join-Path $ScriptDir "src-tauri\target\release\local-super-whisper.exe"
$Ecosystem  = Join-Path $ScriptDir "ecosystem.config.cjs"
$LogDir     = Join-Path $ScriptDir "logs"

# ── colours ──────────────────────────────────────────────────────────────────
function Info    { param($msg) Write-Host "  > $msg" -ForegroundColor Cyan }
function Success { param($msg) Write-Host "  v $msg" -ForegroundColor Green }
function Warn    { param($msg) Write-Host "  ! $msg" -ForegroundColor Yellow }
function Err     { param($msg) Write-Host "  x $msg" -ForegroundColor Red }
function Header  { param($msg) Write-Host "`n=== $msg ===`n" -ForegroundColor Cyan }

# ── helpers ──────────────────────────────────────────────────────────────────
function Check-PM2 {
    if (-not (Get-Command pm2 -ErrorAction SilentlyContinue)) {
        Err "pm2 not found. Install it first:"
        Write-Host "    npm install -g pm2"
        Write-Host "    npm install -g pm2-windows-startup"
        return $false
    }
    return $true
}

function Check-Built {
    if (-not (Test-Path $Exe)) {
        Err "Built executable not found: $Exe"
        Warn "Run option [5] Redeploy to build the app first."
        return $false
    }
    return $true
}

function Ensure-LogDir {
    if (-not (Test-Path $LogDir)) {
        New-Item -ItemType Directory -Path $LogDir | Out-Null
    }
}

function PM2-Has-App {
    $output = pm2 list 2>&1 | Out-String
    return $output -match [regex]::Escape($AppName)
}

# ── commands ─────────────────────────────────────────────────────────────────
function Cmd-Start {
    Header "Starting $AppName"
    if (-not (Check-PM2)) { return }
    if (-not (Check-Built)) { return }
    Ensure-LogDir

    if (PM2-Has-App) {
        Info "Process already registered — restarting."
        pm2 restart $AppName
    } else {
        pm2 start $Ecosystem
        pm2 save
    }
    Success "Started. Use option [4] to view logs."
}

function Cmd-Stop {
    Header "Stopping $AppName"
    if (-not (Check-PM2)) { return }
    try {
        pm2 stop $AppName 2>$null
        Success "Stopped."
    } catch {
        Warn "Process was not running."
    }
}

function Cmd-Restart {
    Header "Restarting $AppName"
    if (-not (Check-PM2)) { return }
    if (-not (Check-Built)) { return }
    if (PM2-Has-App) {
        pm2 restart $AppName
    } else {
        pm2 start $Ecosystem
    }
    pm2 save
    Success "Restarted."
}

function Cmd-Logs {
    Header "Logs — $AppName  (Ctrl+C to exit)"
    if (-not (Check-PM2)) { return }
    if (Test-Path $LogDir) {
        Warn "Log files: $LogDir\"
        Get-ChildItem $LogDir -ErrorAction SilentlyContinue | Format-Table Name, Length, LastWriteTime
    }
    pm2 logs $AppName --lines 100
}

function Cmd-Redeploy {
    Header "Redeploy — build + restart"
    if (-not (Check-PM2)) { return }
    Ensure-LogDir

    Info "Building frontend (tsc + vite)..."
    Set-Location $ScriptDir
    npm run build

    Info "Compiling Tauri/Rust release binary..."
    npm run tauri -- build

    if (-not (Check-Built)) {
        Err "Build succeeded but exe not found at expected path."
        return
    }

    Success "Build complete."
    Cmd-Restart
    Success "Redeployed successfully."
}

function Cmd-Status {
    Header "PM2 Status"
    if (-not (Check-PM2)) { return }
    pm2 list
}

function Cmd-SetupStartup {
    Header "Configure Windows Startup"
    if (-not (Check-PM2)) { return }

    if (-not (Get-Command pm2-startup -ErrorAction SilentlyContinue)) {
        Warn "pm2-windows-startup not found. Installing..."
        npm install -g pm2-windows-startup
    }

    Info "Saving current PM2 process list..."
    pm2 save

    Info "Installing PM2 Windows startup task..."
    pm2-startup install

    Success "Done. PM2 will resurrect $AppName on Windows login."
    Write-Host ""
    Warn "If the app is not started yet, run option [1] first, then re-run this."
}

function Cmd-RemoveStartup {
    Header "Remove Windows Startup"
    if (-not (Check-PM2)) { return }
    if (Get-Command pm2-startup -ErrorAction SilentlyContinue) {
        pm2-startup uninstall
        Success "Startup task removed."
    } else {
        Warn "pm2-windows-startup not installed — nothing to remove."
    }
}

function Cmd-Uninstall {
    Header "Uninstall — Clean Slate"

    $appData  = Join-Path $env:APPDATA "com.localsuperwhisper.app"
    $nsisRoot = Join-Path $env:LOCALAPPDATA "Programs\Local SuperWhisper"

    Warn "This removes the dev/PM2 deployment so the NSIS installer can be tested cleanly."
    Write-Host ""
    Write-Host "  This will:"
    Write-Host "    1. Stop and delete the PM2 process for $AppName"
    Write-Host "    2. Disable the PM2 Windows startup task (if installed)"
    Write-Host "    3. Prompt before wiping app data ($appData)"
    Write-Host "    4. Prompt before removing a prior NSIS install ($nsisRoot)"
    Write-Host ""
    $go = Read-Host "Proceed? [y/N]"
    if ($go -notmatch '^[Yy]') { Warn "Cancelled."; return }

    # 1. PM2 stop + delete + save
    if (Check-PM2) {
        if (PM2-Has-App) {
            Info "Stopping and deleting PM2 entry for $AppName..."
            try { pm2 stop   $AppName 2>$null | Out-Null } catch { }
            try { pm2 delete $AppName 2>$null | Out-Null } catch { }
            try { pm2 save           2>$null | Out-Null } catch { }
            Success "PM2 process removed."
        } else {
            Info "PM2 has no entry for $AppName — skipping."
        }
    }

    # 2. PM2 Windows startup task
    if (Get-Command pm2-startup -ErrorAction SilentlyContinue) {
        Info "Removing PM2 Windows startup task..."
        try { pm2-startup uninstall 2>$null | Out-Null } catch { }
        Success "Startup task removed."
    } else {
        Info "pm2-windows-startup not installed — skipping startup uninstall."
    }

    # 3. App data (settings DB, history, vocabulary, sounds cache)
    Write-Host ""
    if (Test-Path $appData) {
        Warn "App data exists at: $appData"
        Warn "This contains your settings DB, transcription history, and vocabulary."
        $wipe = Read-Host "Wipe app data? [y/N]"
        if ($wipe -match '^[Yy]') {
            Remove-Item -Recurse -Force $appData
            Success "App data wiped."
        } else {
            Info "App data preserved — the installer will reuse it on first run."
        }
    } else {
        Info "No app data found at $appData."
    }

    # 4. Prior NSIS install (only present after a previous installer run)
    Write-Host ""
    if (Test-Path $nsisRoot) {
        Warn "Existing NSIS install found at: $nsisRoot"
        $uninst = Get-ChildItem $nsisRoot -Filter "uninstall.exe" -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($uninst) {
            $run = Read-Host "Run the NSIS uninstaller? [Y/n]"
            if ($run -notmatch '^[Nn]') {
                Info "Launching NSIS uninstaller (interactive)..."
                Start-Process -FilePath $uninst.FullName -Wait
                if (Test-Path $nsisRoot) {
                    Warn "Install directory still present — uninstaller may have been cancelled."
                } else {
                    Success "NSIS install removed."
                }
            } else {
                Info "Skipped NSIS uninstaller. Install directory left in place."
            }
        } else {
            Warn "No uninstall.exe found — removing directory directly (registry entries may persist)."
            Remove-Item -Recurse -Force $nsisRoot
            Success "Install directory removed."
        }
    } else {
        Info "No prior NSIS install found at $nsisRoot."
    }

    Write-Host ""
    Success "Uninstall complete. You can now run the NSIS installer for a clean test."
}

# ── menu ─────────────────────────────────────────────────────────────────────
function Show-Menu {
    Write-Host ""
    Write-Host "  +--------------------------------------+" -ForegroundColor Cyan
    Write-Host "  |   Local SuperWhisper -- Manager      |" -ForegroundColor Cyan
    Write-Host "  +--------------------------------------+" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "    1)  Start"                                    -ForegroundColor Green
    Write-Host "    2)  Stop"                                     -ForegroundColor Red
    Write-Host "    3)  Restart"                                  -ForegroundColor Yellow
    Write-Host "    4)  View Logs"                                -ForegroundColor Cyan
    Write-Host "    5)  Redeploy  (build + restart)"              -ForegroundColor Cyan
    Write-Host "    6)  Status"                                   -ForegroundColor Cyan
    Write-Host "    7)  Enable Startup on Login"                  -ForegroundColor Cyan
    Write-Host "    8)  Disable Startup on Login"                 -ForegroundColor Cyan
    Write-Host "    9)  Uninstall  (clean slate for installer test)" -ForegroundColor Red
    Write-Host "    0)  Exit"                                     -ForegroundColor Gray
    Write-Host ""
    Write-Host -NoNewline "  Choose [0-9]: " -ForegroundColor White
}

# ── entry point ───────────────────────────────────────────────────────────────
$cmd = if ($args.Count -gt 0) { $args[0] } else { $null }

switch ($cmd) {
    "start"     { & Cmd-Start;        exit 0 }
    "stop"      { & Cmd-Stop;         exit 0 }
    "restart"   { & Cmd-Restart;      exit 0 }
    "logs"      { & Cmd-Logs;         exit 0 }
    "redeploy"  { & Cmd-Redeploy;     exit 0 }
    "status"    { & Cmd-Status;       exit 0 }
    "startup"   { & Cmd-SetupStartup; exit 0 }
    "uninstall" { & Cmd-Uninstall;    exit 0 }
}

# Interactive loop
while ($true) {
    Show-Menu
    $choice = Read-Host
    switch ($choice) {
        "1" { & Cmd-Start }
        "2" { & Cmd-Stop }
        "3" { & Cmd-Restart }
        "4" { & Cmd-Logs }
        "5" { & Cmd-Redeploy }
        "6" { & Cmd-Status }
        "7" { & Cmd-SetupStartup }
        "8" { & Cmd-RemoveStartup }
        "9" { & Cmd-Uninstall }
        "0" { Write-Host "Bye." -ForegroundColor Green; exit 0 }
        default { Warn "Invalid choice." }
    }
    Write-Host ""
    Read-Host "Press Enter to return to menu"
}
