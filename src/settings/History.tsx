import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface HistoryEntry {
  id: number;
  text: string;
  word_count: number;
  duration_ms: number;
  wpm: number;
  created_at: string;
}

export default function History() {
  const [entries, setEntries] = useState<HistoryEntry[]>([]);
  const [copiedId, setCopiedId] = useState<number | null>(null);

  useEffect(() => {
    invoke<HistoryEntry[]>("get_history", { limit: 500 }).then(setEntries);
  }, []);

  const copyText = async (entry: HistoryEntry) => {
    await navigator.clipboard.writeText(entry.text);
    setCopiedId(entry.id);
    setTimeout(() => setCopiedId(null), 1500);
  };

  const formatDuration = (ms: number) => {
    const seconds = Math.round(ms / 1000);
    if (seconds < 60) return `${seconds}s`;
    return `${Math.floor(seconds / 60)}m ${seconds % 60}s`;
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="text-text-primary font-semibold">History</h2>
        <span className="text-xs text-text-muted">{entries.length} entries (max 500)</span>
      </div>

      {entries.length === 0 ? (
        <p className="text-text-muted text-sm">No transcriptions yet.</p>
      ) : (
        <div className="space-y-2">
          {entries.map((entry) => (
            <button
              key={entry.id}
              onClick={() => copyText(entry)}
              className="w-full text-left bg-surface-dark rounded-lg p-3 border border-white/5 hover:border-accent/30 transition-colors group"
            >
              <p className="text-sm text-text-primary">{entry.text}</p>
              <div className="flex items-center gap-4 mt-2">
                <span className="text-xs text-text-muted">{entry.word_count} words</span>
                <span className="text-xs text-text-muted">{Math.round(entry.wpm)} WPM</span>
                <span className="text-xs text-text-muted">{formatDuration(entry.duration_ms)}</span>
                <span className="text-xs text-text-muted">{new Date(entry.created_at).toLocaleString()}</span>
                <span className="text-xs text-accent ml-auto opacity-0 group-hover:opacity-100 transition-opacity">
                  {copiedId === entry.id ? "Copied!" : "Click to copy"}
                </span>
              </div>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
