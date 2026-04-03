import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import StatCard from "../components/StatCard";
import ChecklistItem from "../components/ChecklistItem";

interface Stats {
  avg_wpm: number;
  words_this_week: number;
  time_saved_minutes: number;
}

interface HistoryEntry {
  id: number;
  text: string;
  word_count: number;
  duration_ms: number;
  wpm: number;
  created_at: string;
}

interface ChecklistStep {
  step_id: string;
  completed: boolean;
  completed_at: string | null;
}

const CHECKLIST_META: Record<string, { label: string; description: string }> = {
  start_recording: { label: "Start recording", description: "Tap your voice to text with your hotkey." },
  customize_shortcuts: { label: "Customize your shortcuts", description: "Change the keyboard shortcut for SuperWhisper." },
  add_vocabulary: { label: "Add vocabulary", description: "Teach SuperWhisper custom words, names, or industry terms." },
  configure_api: { label: "Configure API", description: "Set up your Faster-Whisper endpoint." },
};

export default function Home() {
  const [stats, setStats] = useState<Stats>({ avg_wpm: 0, words_this_week: 0, time_saved_minutes: 0 });
  const [history, setHistory] = useState<HistoryEntry[]>([]);
  const [checklist, setChecklist] = useState<ChecklistStep[]>([]);

  const loadData = async () => {
    const [s, h, c] = await Promise.all([
      invoke<Stats>("get_stats"),
      invoke<HistoryEntry[]>("get_history", { limit: 5 }),
      invoke<ChecklistStep[]>("get_checklist"),
    ]);
    setStats(s);
    setHistory(h);
    setChecklist(c);
  };

  useEffect(() => {
    loadData();
  }, []);

  const completeStep = async (stepId: string) => {
    await invoke("complete_checklist_step", { stepId });
    loadData();
  };

  return (
    <div className="space-y-8">
      {/* Stats */}
      <div>
        <div className="grid grid-cols-3 gap-4">
          <StatCard label="Average speed" value={`${Math.round(stats.avg_wpm)} WPM`} />
          <StatCard label="Words this week" value={stats.words_this_week.toLocaleString()} />
          <StatCard
            label="Time saved"
            value={`${Math.max(0, Math.round(stats.time_saved_minutes))} min`}
            sublabel="this week"
          />
        </div>
      </div>

      {/* Get Started */}
      <div>
        <h2 className="text-text-primary font-semibold text-sm mb-3">Get started</h2>
        <div className="space-y-1">
          {checklist.map((step) => {
            const meta = CHECKLIST_META[step.step_id];
            if (!meta) return null;
            return (
              <ChecklistItem
                key={step.step_id}
                label={meta.label}
                description={meta.description}
                completed={step.completed}
                onComplete={() => completeStep(step.step_id)}
              />
            );
          })}
        </div>
      </div>

      {/* Recent History */}
      <div>
        <h2 className="text-text-primary font-semibold text-sm mb-3">Recent transcriptions</h2>
        {history.length === 0 ? (
          <p className="text-text-muted text-sm">No transcriptions yet. Press your hotkey to get started.</p>
        ) : (
          <div className="space-y-2">
            {history.map((entry) => (
              <div key={entry.id} className="bg-surface-dark rounded-lg p-3 border border-white/5">
                <p className="text-sm text-text-primary line-clamp-2">{entry.text}</p>
                <div className="flex gap-4 mt-1">
                  <span className="text-xs text-text-muted">{entry.word_count} words</span>
                  <span className="text-xs text-text-muted">{Math.round(entry.wpm)} WPM</span>
                  <span className="text-xs text-text-muted">{new Date(entry.created_at).toLocaleString()}</span>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
