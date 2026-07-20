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
  fix_steps: string[];
  resettable: string | null;
}

const BADGE: Record<PermissionState, { dot: string; chip: string; word: string }> = {
  granted: { dot: "bg-green-400", chip: "text-green-400 bg-green-400/10", word: "Working" },
  denied: { dot: "bg-red-400", chip: "text-red-400 bg-red-400/10", word: "Not working" },
  unknown: { dot: "bg-yellow-400", chip: "text-yellow-400 bg-yellow-400/10", word: "Unknown" },
};

function Row({
  check,
  onOpen,
  onReset,
  busy,
  notice,
}: {
  check: PermissionCheck;
  onOpen: (url: string) => void;
  onReset: (service: string) => void;
  busy: boolean;
  notice: string | null;
}) {
  const badge = BADGE[check.state];
  const broken = check.state !== "granted";

  return (
    <div
      className={`rounded-lg border ${
        broken ? "border-red-400/30 bg-red-400/[0.03]" : "border-white/10 bg-surface-dark"
      }`}
    >
      <div className="px-4 py-3 space-y-1">
        <div className="flex items-center justify-between gap-3">
          <div className="flex items-center gap-2 min-w-0">
            <span className={`w-2 h-2 rounded-full shrink-0 ${badge.dot}`} aria-hidden />
            <span className="text-text-primary text-sm font-medium truncate">{check.label}</span>
          </div>
          <span className={`text-[11px] px-2 py-0.5 rounded-full shrink-0 ${badge.chip}`}>
            {badge.word}
          </span>
        </div>
        <p className="text-text-muted text-xs">{check.description}</p>
      </div>

      {broken && (
        <div className="px-4 pb-3 space-y-3 border-t border-white/5 pt-3">
          {check.detail && <p className="text-text-secondary text-xs">{check.detail}</p>}

          {check.fix_steps.length > 0 && (
            <ol className="space-y-1.5">
              {check.fix_steps.map((step, i) => (
                <li key={i} className="flex gap-2 text-xs text-text-secondary">
                  <span className="shrink-0 w-4 h-4 rounded-full bg-white/10 text-text-muted text-[10px] flex items-center justify-center mt-px">
                    {i + 1}
                  </span>
                  <span>{step}</span>
                </li>
              ))}
            </ol>
          )}

          <div className="flex flex-wrap gap-2">
            {check.resettable && (
              <button
                onClick={() => onReset(check.resettable!)}
                disabled={busy}
                className="py-1.5 px-3 bg-accent text-white rounded-md text-xs font-medium hover:bg-accent/90 transition-colors disabled:opacity-50"
              >
                Reset &amp; ask again
              </button>
            )}
            {check.settings_url && (
              <button
                onClick={() => onOpen(check.settings_url!)}
                disabled={busy}
                className="py-1.5 px-3 bg-surface border border-white/15 rounded-md text-xs text-text-secondary hover:text-text-primary transition-colors disabled:opacity-50"
              >
                Open Settings
              </button>
            )}
          </div>

          {notice && <p className="text-accent text-xs">{notice}</p>}
        </div>
      )}
    </div>
  );
}

export default function Permissions({
  onContinue,
  continueLabel = "Continue",
  showContinue = true,
  title = "Permissions",
}: {
  onContinue: () => void;
  continueLabel?: string;
  showContinue?: boolean;
  title?: string;
}) {
  const [checks, setChecks] = useState<PermissionCheck[] | null>(null);
  const [checking, setChecking] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notices, setNotices] = useState<Record<string, string>>({});

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
    setBusy(true);
    try {
      await invoke("open_permission_settings", { url });
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const reset = async (service: string) => {
    setBusy(true);
    setError(null);
    try {
      const msg = await invoke<string>("reset_permission", { service });
      setNotices((n) => ({ ...n, [service]: msg }));
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const blockers = (checks ?? []).filter((c) => c.required && c.state !== "granted");
  const ready = checks !== null && blockers.length === 0;

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between gap-3">
        <div>
          <h2 className="text-text-primary font-semibold">{title}</h2>
          <p className="text-text-muted text-xs mt-0.5">
            macOS has to allow each of these separately. They are tied to one exact
            build, so installing a new version switches them back off.
          </p>
        </div>
        <button
          onClick={runChecks}
          disabled={checking || busy}
          className="shrink-0 py-1.5 px-3 bg-surface-dark border border-white/15 rounded-md text-xs text-text-secondary hover:text-text-primary transition-colors disabled:opacity-50"
        >
          {checking ? "Checking…" : "Re-check"}
        </button>
      </div>

      {checks !== null && (
        <div
          className={`rounded-lg px-4 py-2.5 text-xs ${
            ready
              ? "bg-green-400/10 text-green-400"
              : "bg-red-400/10 text-red-400"
          }`}
        >
          {ready
            ? "All set — dictation and auto-paste should work."
            : `${blockers.length} of ${checks.length} not working. Dictation will not paste correctly until these are fixed.`}
        </div>
      )}

      {checks === null && <p className="text-text-muted text-sm">Checking…</p>}

      {checks !== null && (
        <div className="space-y-2">
          {checks.map((c) => (
            <Row
              key={c.id}
              check={c}
              onOpen={openSettings}
              onReset={reset}
              busy={busy}
              notice={c.resettable ? notices[c.resettable] ?? null : null}
            />
          ))}
        </div>
      )}

      {error && <p className="text-red-400 text-xs">{error}</p>}

      {showContinue && (
        <div className="flex gap-2 pt-1">
          <button
            onClick={onContinue}
            disabled={!ready}
            className="flex-1 py-2 px-4 bg-accent text-white rounded-lg text-sm font-medium hover:bg-accent/90 transition-colors disabled:opacity-50"
          >
            {continueLabel}
          </button>
        </div>
      )}

      {showContinue && checks !== null && !ready && (
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
