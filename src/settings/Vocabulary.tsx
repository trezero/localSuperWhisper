import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface VocabularyEntry {
  id: number;
  term: string;
}

export default function Vocabulary() {
  const [terms, setTerms] = useState<VocabularyEntry[]>([]);
  const [newTerm, setNewTerm] = useState("");
  const [error, setError] = useState("");

  const loadTerms = async () => {
    const t = await invoke<VocabularyEntry[]>("get_vocabulary");
    setTerms(t);
  };

  useEffect(() => {
    loadTerms();
  }, []);

  const addTerm = async () => {
    const trimmed = newTerm.trim();
    if (!trimmed) return;
    try {
      await invoke("add_vocabulary_term", { term: trimmed });
      setNewTerm("");
      setError("");
      loadTerms();
    } catch {
      setError("Term already exists.");
    }
  };

  const removeTerm = async (id: number) => {
    await invoke("remove_vocabulary_term", { id });
    loadTerms();
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") {
      e.preventDefault();
      addTerm();
    }
  };

  return (
    <div className="space-y-6 max-w-lg">
      <div>
        <h2 className="text-text-primary font-semibold">Vocabulary</h2>
        <p className="text-xs text-text-muted mt-1">
          Custom words, names, and terms to improve transcription accuracy.
          These are injected into Whisper's initial prompt.
        </p>
      </div>

      {/* Add term */}
      <div className="flex gap-2">
        <input
          type="text"
          value={newTerm}
          onChange={(e) => setNewTerm(e.target.value)}
          onKeyDown={handleKeyDown}
          className="flex-1 bg-surface-dark border border-white/10 rounded-lg px-3 py-2 text-sm text-text-primary focus:outline-none focus:border-accent"
          placeholder="Add a word or phrase..."
        />
        <button
          onClick={addTerm}
          disabled={!newTerm.trim()}
          className="px-4 py-2 bg-accent hover:bg-accent-hover disabled:opacity-40 rounded-lg text-sm text-white font-medium transition-colors"
        >
          Add
        </button>
      </div>
      {error && <p className="text-xs text-red-400">{error}</p>}

      {/* Term list */}
      {terms.length === 0 ? (
        <p className="text-text-muted text-sm">No vocabulary terms yet.</p>
      ) : (
        <div className="space-y-1">
          {terms.map((entry) => (
            <div
              key={entry.id}
              className="flex items-center justify-between bg-surface-dark rounded-lg px-3 py-2 border border-white/5 group"
            >
              <span className="text-sm text-text-primary">{entry.term}</span>
              <button
                onClick={() => removeTerm(entry.id)}
                className="text-text-muted hover:text-red-400 opacity-0 group-hover:opacity-100 transition-opacity text-xs"
              >
                Remove
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
