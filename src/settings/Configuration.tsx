import { useEffect, useState, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";

function displayKey(code: string): string {
  const names: Record<string, string> = {
    AltRight: "Right Alt", AltLeft: "Left Alt",
    ControlRight: "Right Ctrl", ControlLeft: "Left Ctrl",
    ShiftRight: "Right Shift", ShiftLeft: "Left Shift",
    MetaLeft: "Left Win", MetaRight: "Right Win",
    CapsLock: "Caps Lock", ScrollLock: "Scroll Lock",
    Pause: "Pause", Insert: "Insert", Delete: "Delete",
    Home: "Home", End: "End", PageUp: "Page Up", PageDown: "Page Down",
    F1: "F1", F2: "F2", F3: "F3", F4: "F4", F5: "F5", F6: "F6",
    F7: "F7", F8: "F8", F9: "F9", F10: "F10", F11: "F11", F12: "F12",
  };
  return names[code] || code;
}

interface AudioDevice {
  name: string;
  is_default: boolean;
}

export default function Configuration() {
  const [settings, setSettings] = useState<Record<string, string>>({});
  const [devices, setDevices] = useState<AudioDevice[]>([]);
  const [saved, setSaved] = useState(false);
  const [listeningForHotkey, setListeningForHotkey] = useState(false);
  const [hotkeyError, setHotkeyError] = useState<string | null>(null);
  const hotkeyListenerRef = useRef<((e: KeyboardEvent) => void) | null>(null);

  const loadSettings = useCallback(async () => {
    const pairs = await invoke<[string, string][]>("get_settings");
    const map: Record<string, string> = {};
    pairs.forEach(([k, v]) => (map[k] = v));
    setSettings(map);
  }, []);

  const loadDevices = useCallback(async () => {
    const d = await invoke<AudioDevice[]>("get_audio_devices");
    setDevices(d);
  }, []);

  useEffect(() => {
    loadSettings();
    loadDevices();
  }, [loadSettings, loadDevices]);

  const update = async (key: string, value: string) => {
    setSettings((prev) => ({ ...prev, [key]: value }));
    await invoke("update_setting", { key, value });
    setSaved(true);
    setTimeout(() => setSaved(false), 2000);
  };

  const startHotkeyListen = () => {
    if (hotkeyListenerRef.current) {
      window.removeEventListener("keydown", hotkeyListenerRef.current, true);
    }
    setListeningForHotkey(true);
    setHotkeyError(null);

    const handler = async (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      window.removeEventListener("keydown", handler, true);
      hotkeyListenerRef.current = null;
      setListeningForHotkey(false);

      if (e.code === "Escape") return;

      try {
        await invoke("update_setting", { key: "hotkey", value: e.code });
        await invoke("register_hotkey", { key: e.code });
        setSettings((prev) => ({ ...prev, hotkey: e.code }));
        setSaved(true);
        setTimeout(() => setSaved(false), 2000);
      } catch {
        setHotkeyError(`Could not register "${displayKey(e.code)}" — try a different key.`);
      }
    };

    hotkeyListenerRef.current = handler;
    window.addEventListener("keydown", handler, true);
  };

  return (
    <div className="space-y-8 max-w-lg">
      <div className="flex items-center justify-between">
        <h2 className="text-text-primary font-semibold">Configuration</h2>
        {saved && <span className="text-xs text-green-400">Saved</span>}
      </div>

      {/* Hotkey */}
      <Field label="Hotkey" description="Global keyboard shortcut to start/stop recording.">
        <div className="space-y-2">
          <div className="flex items-center gap-2">
            <div className="flex-1 bg-surface-dark border border-white/10 rounded-lg px-3 py-2 text-sm text-text-primary">
              {listeningForHotkey
                ? <span className="text-accent">Press any key… (Esc to cancel)</span>
                : settings.hotkey
                  ? displayKey(settings.hotkey)
                  : <span className="text-text-muted">Not set</span>
              }
            </div>
            <button
              onClick={startHotkeyListen}
              disabled={listeningForHotkey}
              className="px-3 py-2 bg-surface-dark border border-white/10 rounded-lg text-sm text-text-secondary hover:text-text-primary hover:border-accent/50 transition-colors disabled:opacity-50"
            >
              Choose Hotkey
            </button>
          </div>
          {hotkeyError && <p className="text-red-400 text-xs">{hotkeyError}</p>}
        </div>
      </Field>

      {/* API URL */}
      <Field label="API URL" description="Faster-Whisper server endpoint.">
        <input
          type="text"
          value={settings.api_url || ""}
          onChange={(e) => update("api_url", e.target.value)}
          className="w-full bg-surface-dark border border-white/10 rounded-lg px-3 py-2 text-sm text-text-primary focus:outline-none focus:border-accent"
          placeholder="http://172.16.1.222:8028/v1"
        />
      </Field>

      {/* API Key */}
      <Field label="API Key" description="Authorization key for the Whisper API.">
        <input
          type="password"
          value={settings.api_key || ""}
          onChange={(e) => update("api_key", e.target.value)}
          className="w-full bg-surface-dark border border-white/10 rounded-lg px-3 py-2 text-sm text-text-primary focus:outline-none focus:border-accent"
          placeholder="cant-be-empty"
        />
      </Field>

      {/* Model ID */}
      <Field label="Model ID" description="Whisper model identifier on the server.">
        <input
          type="text"
          value={settings.model_id || ""}
          onChange={(e) => update("model_id", e.target.value)}
          className="w-full bg-surface-dark border border-white/10 rounded-lg px-3 py-2 text-sm text-text-primary focus:outline-none focus:border-accent"
          placeholder="deepdml/faster-whisper-large-v3-turbo-ct2"
        />
      </Field>

      {/* Microphone */}
      <Field label="Microphone" description="Audio input device for recording.">
        <select
          value={settings.mic_device || "default"}
          onChange={(e) => update("mic_device", e.target.value)}
          className="w-full bg-surface-dark border border-white/10 rounded-lg px-3 py-2 text-sm text-text-primary focus:outline-none focus:border-accent"
        >
          <option value="default">System Default</option>
          {devices.map((d) => (
            <option key={d.name} value={d.name}>
              {d.name} {d.is_default ? "(default)" : ""}
            </option>
          ))}
        </select>
      </Field>
    </div>
  );
}

function Field({ label, description, children }: { label: string; description: string; children: React.ReactNode }) {
  return (
    <div>
      <label className="block text-sm font-medium text-text-primary mb-1">{label}</label>
      <p className="text-xs text-text-muted mb-2">{description}</p>
      {children}
    </div>
  );
}
