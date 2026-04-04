const path = require("path");

const PROJECT_ROOT = path.resolve(__dirname);
const EXE = path.join(
  PROJECT_ROOT,
  "src-tauri",
  "target",
  "release",
  "local-super-whisper.exe"
);

module.exports = {
  apps: [
    {
      name: "localSuperWhisper",
      script: EXE,
      cwd: PROJECT_ROOT,

      // Don't restart on clean exit (code 0 = user closed the tray app intentionally).
      // DO restart on crash (non-zero exit).
      autorestart: true,
      stop_exit_codes: [0],
      max_restarts: 10,
      min_uptime: "5s",

      // GUI apps on Windows don't write to stdout, but capture stderr for crash logs.
      out_file: path.join(PROJECT_ROOT, "logs", "app-out.log"),
      error_file: path.join(PROJECT_ROOT, "logs", "app-err.log"),
      log_date_format: "YYYY-MM-DD HH:mm:ss",

      // PM2 should not interpret this as a Node.js script.
      interpreter: "none",
    },
  ],
};
