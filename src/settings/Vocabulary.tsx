import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface VocabularyEntry {
  id: number;
  term: string;
}

interface CorrectionEntry {
  id: number;
  from_text: string;
  to_text: string;
}

export default function Vocabulary() {
  const [terms, setTerms] = useState<VocabularyEntry[]>([]);
  const [newTerm, setNewTerm] = useState("");
  const [termError, setTermError] = useState("");

  const [corrections, setCorrections] = useState<CorrectionEntry[]>([]);
  const [fromText, setFromText] = useState("");
  const [toText, setToText] = useState("");

  const loadTerms = async () => {
    const t = await invoke<VocabularyEntry[]>("get_vocabulary");
    setTerms(t);
  };

  const loadCorrections = async () => {
    const c = await invoke<CorrectionEntry[]>("get_corrections");
    setCorrections(c);
  };

  useEffect(() => {
    loadTerms();
    loadCorrections();
  }, []);

  const addTerm = async () => {
    const trimmed = newTerm.trim();
    if (!trimmed) return;
    try {
      await invoke("add_vocabulary_term", { term: trimmed });
      setNewTerm("");
      setTermError("");
      loadTerms();
    } catch {
      setTermError("Term already exists.");
    }
  };

  const removeTerm = async (id: number) => {
    await invoke("remove_vocabulary_term", { id });
    loadTerms();
  };

  const addCorrection = async () => {
    const from = fromText.trim();
    const to = toText.trim();
    if (!from || !to) return;
    await invoke("add_correction", { fromText: from, toText: to });
    setFromText("");
    setToText("");
    loadCorrections();
  };

  const removeCorrection = async (id: number) => {
    await invoke("remove_correction", { id });
    loadCorrections();
  };

  return (
    <div className="space-y-10 max-w-lg">

      {/* Vocabulary hints */}
      <section className="space-y-4">
        <div>
          <h2 className="text-text-primary font-semibold">Vocabulary Hints</h2>
          <p className="text-xs text-text-muted mt-1">
            Words and phrases sent to Whisper as context hints. Helps with uncommon
            names, acronyms, and domain-specific terms.
          </p>
        </div>

        <div className="flex gap-2">
          <input
            type="text"
            value={newTerm}
            onChange={(e) => setNewTerm(e.target.value)}
            onKeyDown={(e) => { if (e.key === "Enter") { e.preventDefault(); addTerm(); } }}
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
        {termError && <p className="text-xs text-red-400">{termError}</p>}

        {terms.length === 0 ? (
          <p className="text-text-muted text-sm">No vocabulary hints yet.</p>
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
      </section>

      {/* Corrections */}
      <section className="space-y-4">
        <div>
          <h2 className="text-text-primary font-semibold">Corrections</h2>
          <p className="text-xs text-text-muted mt-1">
            Find-and-replace rules applied after every transcription. Use this to fix
            names or words Whisper consistently gets wrong.
          </p>
        </div>

        <div className="flex gap-2 items-center">
          <input
            type="text"
            value={fromText}
            onChange={(e) => setFromText(e.target.value)}
            onKeyDown={(e) => { if (e.key === "Enter") { e.preventDefault(); addCorrection(); } }}
            className="flex-1 bg-surface-dark border border-white/10 rounded-lg px-3 py-2 text-sm text-text-primary focus:outline-none focus:border-accent"
            placeholder="Whisper writes..."
          />
          <span className="text-text-muted text-sm flex-shrink-0">→</span>
          <input
            type="text"
            value={toText}
            onChange={(e) => setToText(e.target.value)}
            onKeyDown={(e) => { if (e.key === "Enter") { e.preventDefault(); addCorrection(); } }}
            className="flex-1 bg-surface-dark border border-white/10 rounded-lg px-3 py-2 text-sm text-text-primary focus:outline-none focus:border-accent"
            placeholder="Replace with..."
          />
          <button
            onClick={addCorrection}
            disabled={!fromText.trim() || !toText.trim()}
            className="px-4 py-2 bg-accent hover:bg-accent-hover disabled:opacity-40 rounded-lg text-sm text-white font-medium transition-colors flex-shrink-0"
          >
            Add
          </button>
        </div>

        {corrections.length === 0 ? (
          <p className="text-text-muted text-sm">No corrections yet.</p>
        ) : (
          <div className="space-y-1">
            {corrections.map((entry) => (
              <div
                key={entry.id}
                className="flex items-center gap-2 bg-surface-dark rounded-lg px-3 py-2 border border-white/5 group"
              >
                <span className="text-sm text-red-400/80 flex-1 truncate">{entry.from_text}</span>
                <span className="text-text-muted text-xs flex-shrink-0">→</span>
                <span className="text-sm text-green-400/80 flex-1 truncate">{entry.to_text}</span>
                <button
                  onClick={() => removeCorrection(entry.id)}
                  className="text-text-muted hover:text-red-400 opacity-0 group-hover:opacity-100 transition-opacity text-xs flex-shrink-0"
                >
                  Remove
                </button>
              </div>
            ))}
          </div>
        )}
      </section>

    </div>
  );
}
