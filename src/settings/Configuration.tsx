import { useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface AudioDevice {
  name: string;
  is_default: boolean;
}

export default function Configuration() {
  const [settings, setSettings] = useState<Record<string, string>>({});
  const [devices, setDevices] = useState<AudioDevice[]>([]);
  const [saved, setSaved] = useState(false);

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

  return (
    <div className="space-y-8 max-w-lg">
      <div className="flex items-center justify-between">
        <h2 className="text-text-primary font-semibold">Configuration</h2>
        {saved && <span className="text-xs text-green-400">Saved</span>}
      </div>

      {/* Hotkey */}
      <Field label="Hotkey" description="Global keyboard shortcut to start/stop recording.">
        <select
          value={settings.hotkey || "RAlt"}
          onChange={(e) => update("hotkey", e.target.value)}
          className="w-full bg-surface-dark border border-white/10 rounded-lg px-3 py-2 text-sm text-text-primary focus:outline-none focus:border-accent"
        >
          <option value="RAlt">Right Alt</option>
          <option value="LAlt">Left Alt</option>
          <option value="RControl">Right Ctrl</option>
          <option value="LControl">Left Ctrl</option>
          <option value="F9">F9</option>
          <option value="F10">F10</option>
          <option value="F11">F11</option>
          <option value="F12">F12</option>
        </select>
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
