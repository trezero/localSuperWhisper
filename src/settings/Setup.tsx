import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useNavigate } from "react-router-dom";

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

export default function Setup({ onDone }: { onDone: () => void }) {
  const [listening, setListening] = useState(false);
  const [chosenKey, setChosenKey] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const navigate = useNavigate();
  const listenerRef = useRef<((e: KeyboardEvent) => void) | null>(null);

  useEffect(() => {
    return () => {
      if (listenerRef.current) {
        window.removeEventListener("keydown", listenerRef.current, true);
      }
    };
  }, []);

  const startListening = () => {
    if (listenerRef.current) {
      window.removeEventListener("keydown", listenerRef.current, true);
    }
    setListening(true);
    setChosenKey(null);
    setError(null);

    const handler = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();

      if (e.code === "Escape") {
        setListening(false);
        window.removeEventListener("keydown", handler, true);
        listenerRef.current = null;
        return;
      }

      setChosenKey(e.code);
      setListening(false);
      window.removeEventListener("keydown", handler, true);
      listenerRef.current = null;
    };

    listenerRef.current = handler;
    window.addEventListener("keydown", handler, true);
  };

  const save = async () => {
    if (!chosenKey) return;
    setSaving(true);
    setError(null);
    try {
      await invoke("update_setting", { key: "hotkey", value: chosenKey });
      await invoke("register_hotkey", { key: chosenKey });
      onDone();
      navigate("/settings");
    } catch (e) {
      setError(`Could not register "${displayKey(chosenKey)}" — try a different key.`);
      setSaving(false);
    }
  };

  return (
    <div className="flex h-screen bg-surface items-center justify-center">
      <div className="max-w-sm w-full px-6 space-y-6">
        <div>
          <h1 className="text-text-primary font-semibold text-lg">Welcome to Local SuperWhisper</h1>
          <p className="text-text-muted text-sm mt-1">
            Choose a global hotkey to start and stop recording from anywhere.
          </p>
          <p className="text-text-muted text-xs mt-2">
            Tip: F9–F12 and other non-modifier keys work most reliably.
          </p>
        </div>

        <div className="space-y-3">
          {!chosenKey && !listening && (
            <button
              onClick={startListening}
              className="w-full py-3 px-4 bg-accent text-white rounded-lg font-medium text-sm hover:bg-accent/90 transition-colors"
            >
              Choose Hotkey
            </button>
          )}

          {listening && (
            <div className="w-full py-3 px-4 bg-surface-dark border border-accent/50 rounded-lg text-center">
              <p className="text-accent text-sm font-medium">Press any key…</p>
              <p className="text-text-muted text-xs mt-1">Esc to cancel</p>
            </div>
          )}

          {chosenKey && (
            <>
              <div className="w-full py-3 px-4 bg-surface-dark border border-white/10 rounded-lg flex items-center justify-between">
                <span className="text-text-muted text-sm">Selected</span>
                <span className="text-text-primary font-semibold">{displayKey(chosenKey)}</span>
              </div>
              {error && <p className="text-red-400 text-xs">{error}</p>}
              <div className="flex gap-2">
                <button
                  onClick={startListening}
                  className="flex-1 py-2 px-4 bg-surface-dark border border-white/10 rounded-lg text-sm text-text-secondary hover:text-text-primary transition-colors"
                >
                  Change
                </button>
                <button
                  onClick={save}
                  disabled={saving}
                  className="flex-1 py-2 px-4 bg-accent text-white rounded-lg text-sm font-medium hover:bg-accent/90 transition-colors disabled:opacity-50"
                >
                  {saving ? "Saving…" : "Save & Continue"}
                </button>
              </div>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
