import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

export type PermissionState = "granted" | "denied" | "unknown";

export interface PermissionCheck {
  id: string;
  label: string;
  description: string;
  state: PermissionState;
  detail: string | null;
  required: boolean;
  settings_url: string | null;
}

const STATE_STYLES: Record<PermissionState, { dot: string; text: string; word: string }> = {
  granted: { dot: "bg-green-400", text: "text-green-400", word: "Ready" },
  denied: { dot: "bg-red-400", text: "text-red-400", word: "Needs attention" },
  unknown: { dot: "bg-yellow-400", text: "text-yellow-400", word: "Unknown" },
};

function Row({ check, onOpen }: { check: PermissionCheck; onOpen: (url: string) => void }) {
  const style = STATE_STYLES[check.state];

  return (
    <div className="py-3 px-4 bg-surface-dark border border-white/10 rounded-lg space-y-1">
      <div className="flex items-center justify-between gap-3">
        <div className="flex items-center gap-2 min-w-0">
          <span className={`w-2 h-2 rounded-full shrink-0 ${style.dot}`} aria-hidden />
          <span className="text-text-primary text-sm font-medium truncate">{check.label}</span>
          {!check.required && (
            <span className="text-text-muted text-[10px] uppercase tracking-wide shrink-0">
              optional
            </span>
          )}
        </div>
        <span className={`text-xs shrink-0 ${style.text}`}>{style.word}</span>
      </div>

      <p className="text-text-muted text-xs">{check.description}</p>

      {check.state !== "granted" && check.detail && (
        <p className="text-text-secondary text-xs pt-1">{check.detail}</p>
      )}

      {check.state !== "granted" && check.settings_url && (
        <button
          onClick={() => onOpen(check.settings_url!)}
          className="text-accent text-xs hover:underline pt-1"
        >
          Open the right settings page →
        </button>
      )}
    </div>
  );
}

export default function Permissions({
  onContinue,
  continueLabel = "Continue",
}: {
  onContinue: () => void;
  continueLabel?: string;
}) {
  const [checks, setChecks] = useState<PermissionCheck[] | null>(null);
  const [checking, setChecking] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const runChecks = useCallback(async () => {
    setChecking(true);
    setError(null);
    try {
      setChecks(await invoke<PermissionCheck[]>("check_permissions", { device: null }));
    } catch (e) {
      setError(String(e));
    } finally {
      setChecking(false);
    }
  }, []);

  useEffect(() => {
    runChecks();
  }, [runChecks]);

  const openSettings = async (url: string) => {
    try {
      await invoke("open_permission_settings", { url });
    } catch (e) {
      setError(String(e));
    }
  };

  const blockers = (checks ?? []).filter((c) => c.required && c.state !== "granted");
  const ready = checks !== null && blockers.length === 0;

  return (
    <div className="space-y-4">
      <div>
        <h1 className="text-text-primary font-semibold text-lg">Check permissions</h1>
        <p className="text-text-muted text-sm mt-1">
          Your OS has to allow a few things before dictation can work. Granting the
          microphone usually shows a system prompt the first time it is checked.
        </p>
      </div>

      {checks === null && (
        <p className="text-text-muted text-sm">Checking…</p>
      )}

      {checks !== null && (
        <div className="space-y-2">
          {checks.map((c) => (
            <Row key={c.id} check={c} onOpen={openSettings} />
          ))}
        </div>
      )}

      {error && <p className="text-red-400 text-xs">{error}</p>}

      {checks !== null && !ready && (
        <p className="text-text-muted text-xs">
          After changing a setting, come back and re-check. macOS sometimes requires
          quitting and reopening the app before a new grant takes effect.
        </p>
      )}

      <div className="flex gap-2">
        <button
          onClick={runChecks}
          disabled={checking}
          className="flex-1 py-2 px-4 bg-surface-dark border border-white/10 rounded-lg text-sm text-text-secondary hover:text-text-primary transition-colors disabled:opacity-50"
        >
          {checking ? "Checking…" : "Re-check"}
        </button>
        <button
          onClick={onContinue}
          disabled={!ready}
          className="flex-1 py-2 px-4 bg-accent text-white rounded-lg text-sm font-medium hover:bg-accent/90 transition-colors disabled:opacity-50"
        >
          {continueLabel}
        </button>
      </div>

      {checks !== null && !ready && (
        <button
          onClick={onContinue}
          className="w-full text-text-muted text-xs hover:text-text-secondary transition-colors"
        >
          Skip for now — dictation will not work until this is resolved
        </button>
      )}
    </div>
  );
}
