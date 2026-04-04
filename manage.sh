#!/usr/bin/env bash
# manage.sh — Local SuperWhisper process manager
# Requires: npm install -g pm2

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APP_NAME="localSuperWhisper"
ECOSYSTEM="$SCRIPT_DIR/ecosystem.config.cjs"
LOG_DIR="$SCRIPT_DIR/logs"

# ── platform detection ─────────────────────────────────────────────────────
OS="$(uname -s)"
case "$OS" in
  Linux*)
    PLATFORM="linux"
    EXE="$SCRIPT_DIR/src-tauri/target/release/local-super-whisper"
    ;;
  MINGW*|MSYS*|CYGWIN*|Windows_NT)
    PLATFORM="windows"
    EXE="$SCRIPT_DIR/src-tauri/target/release/local-super-whisper.exe"
    ;;
  *)
    echo "Unsupported platform: $OS"
    exit 1
    ;;
esac

# ── colours ─────────────────────────────────────────────────────────────────
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
CYAN='\033[0;36m'; BOLD='\033[1m'; RESET='\033[0m'

info()    { echo -e "${CYAN}▸${RESET} $*"; }
success() { echo -e "${GREEN}✔${RESET} $*"; }
warn()    { echo -e "${YELLOW}⚠${RESET} $*"; }
error()   { echo -e "${RED}✖${RESET} $*"; }
header()  { echo -e "\n${BOLD}${CYAN}$*${RESET}\n"; }

# ── helpers ──────────────────────────────────────────────────────────────────
check_pm2() {
  if ! command -v pm2 &>/dev/null; then
    error "pm2 not found. Install it first:"
    echo "    npm install -g pm2"
    exit 1
  fi
}

check_built() {
  if [[ ! -f "$EXE" ]]; then
    error "Built executable not found: $EXE"
    warn  "Run option [5] Redeploy to build the app first."
    return 1
  fi
  return 0
}

ensure_log_dir() {
  mkdir -p "$LOG_DIR"
}

# ── commands ─────────────────────────────────────────────────────────────────
cmd_start() {
  header "Starting $APP_NAME"
  check_pm2
  if ! check_built; then exit 1; fi
  ensure_log_dir

  if pm2 list | grep -q "$APP_NAME"; then
    info "Process already registered — doing restart instead."
    pm2 restart "$APP_NAME"
  else
    pm2 start "$ECOSYSTEM"
    pm2 save
  fi
  success "Started. Use option [4] to view logs."
}

cmd_stop() {
  header "Stopping $APP_NAME"
  check_pm2
  pm2 stop "$APP_NAME" 2>/dev/null && success "Stopped." || warn "Process was not running."
}

cmd_restart() {
  header "Restarting $APP_NAME"
  check_pm2
  if ! check_built; then exit 1; fi
  pm2 restart "$APP_NAME" 2>/dev/null || pm2 start "$ECOSYSTEM"
  success "Restarted."
}

cmd_logs() {
  header "Logs — $APP_NAME  (Ctrl+C to exit)"
  check_pm2
  if [[ -d "$LOG_DIR" ]]; then
    echo -e "${YELLOW}Log files:${RESET} $LOG_DIR/"
    ls -lh "$LOG_DIR" 2>/dev/null || true
    echo ""
  fi
  pm2 logs "$APP_NAME" --lines 100
}

cmd_redeploy() {
  header "Redeploy — build + restart"
  check_pm2
  ensure_log_dir

  info "Building frontend (tsc + vite)…"
  cd "$SCRIPT_DIR"
  npm run build

  info "Compiling Tauri/Rust release binary…"
  npm run tauri -- build

  if ! check_built; then
    error "Build succeeded but exe not found at expected path."
    exit 1
  fi

  success "Build complete."
  cmd_restart
  success "Redeployed successfully."
}

cmd_status() {
  header "PM2 Status"
  check_pm2
  pm2 list
}

cmd_setup_startup() {
  if [[ "$PLATFORM" == "linux" ]]; then
    header "Configure Linux Autostart"
    AUTOSTART_DIR="$HOME/.config/autostart"
    mkdir -p "$AUTOSTART_DIR"
    cat > "$AUTOSTART_DIR/local-super-whisper.desktop" << EOF
[Desktop Entry]
Type=Application
Name=Local SuperWhisper
Exec=$EXE
Icon=$SCRIPT_DIR/src-tauri/icons/icon.png
Comment=Voice-to-text transcription
Categories=Utility;Audio;
StartupNotify=false
X-GNOME-Autostart-enabled=true
EOF
    success "Autostart enabled. App will launch on next login."
  else
    header "Configure Windows Startup"
    check_pm2
    if ! command -v pm2-startup &>/dev/null; then
      warn "pm2-windows-startup not found. Installing…"
      npm install -g pm2-windows-startup
    fi
    info "Saving current PM2 process list…"
    pm2 save
    info "Installing PM2 Windows startup task…"
    pm2-startup install
    success "Done. PM2 will now resurrect $APP_NAME on Windows login."
    echo ""
    warn "If you haven't started the app yet, run option [1] Start first, then re-run this option."
  fi
}

cmd_remove_startup() {
  if [[ "$PLATFORM" == "linux" ]]; then
    header "Remove Linux Autostart"
    rm -f "$HOME/.config/autostart/local-super-whisper.desktop"
    success "Autostart disabled."
  else
    header "Remove Windows Startup"
    check_pm2
    if command -v pm2-startup &>/dev/null; then
      pm2-startup uninstall && success "Startup task removed."
    else
      warn "pm2-windows-startup not installed — nothing to remove."
    fi
  fi
}

cmd_install_linux_deps() {
  header "Install Linux Build Dependencies"
  if [[ "$PLATFORM" != "linux" ]]; then
    warn "This option is only available on Linux."
    return
  fi
  info "This requires sudo access."
  sudo apt update
  sudo apt install -y \
    build-essential curl wget \
    libwebkit2gtk-4.1-dev libgtk-3-dev \
    libayatana-appindicator3-dev librsvg2-dev \
    libasound2-dev libssl-dev pkg-config xdotool libxdo-dev
  success "Dependencies installed."
}

# ── interactive menu ──────────────────────────────────────────────────────────
show_menu() {
  echo ""
  echo -e "${BOLD}╔══════════════════════════════════════╗${RESET}"
  echo -e "${BOLD}║   Local SuperWhisper — Manager       ║${RESET}"
  echo -e "${BOLD}╚══════════════════════════════════════╝${RESET}"
  echo ""
  echo -e "  ${GREEN}1)${RESET} Start"
  echo -e "  ${RED}2)${RESET} Stop"
  echo -e "  ${YELLOW}3)${RESET} Restart"
  echo -e "  ${CYAN}4)${RESET} View Logs"
  echo -e "  ${CYAN}5)${RESET} Redeploy  ${YELLOW}(build + restart)${RESET}"
  echo -e "  ${CYAN}6)${RESET} Status"
  echo -e "  ${CYAN}7)${RESET} Enable Startup on Login"
  echo -e "  ${CYAN}8)${RESET} Disable Startup on Login"
  echo -e "  ${CYAN}9)${RESET} Install Build Dependencies  ${YELLOW}[Linux only]${RESET}"
  echo -e "  ${CYAN}0)${RESET} Exit"
  echo ""
  echo -n -e "${BOLD}Choose [0-9]:${RESET} "
}

# ── entry point ───────────────────────────────────────────────────────────────

# Allow direct command invocation: ./manage.sh start | stop | restart | logs | redeploy | status
case "${1:-}" in
  start)    cmd_start;           exit 0 ;;
  stop)     cmd_stop;            exit 0 ;;
  restart)  cmd_restart;         exit 0 ;;
  logs)     cmd_logs;            exit 0 ;;
  redeploy) cmd_redeploy;        exit 0 ;;
  status)   cmd_status;          exit 0 ;;
  startup)  cmd_setup_startup;   exit 0 ;;
esac

# Interactive mode
while true; do
  show_menu
  read -r choice
  case "$choice" in
    1) cmd_start ;;
    2) cmd_stop ;;
    3) cmd_restart ;;
    4) cmd_logs ;;
    5) cmd_redeploy ;;
    6) cmd_status ;;
    7) cmd_setup_startup ;;
    8) cmd_remove_startup ;;
    9) cmd_install_linux_deps ;;
    0) echo "Bye."; exit 0 ;;
    *) warn "Invalid choice." ;;
  esac
  echo ""
  echo -n -e "Press ${BOLD}Enter${RESET} to return to menu…"
  read -r
done
