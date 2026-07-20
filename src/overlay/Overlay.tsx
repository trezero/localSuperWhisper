import { useCallback, useState } from "react";
import { useTauriEvent } from "../hooks/useTauriEvent";
import Waveform from "./Waveform";
import TranscriptDisplay from "./TranscriptDisplay";

type OverlayState = "idle" | "recording" | "transcribing" | "result" | "error";

export default function Overlay() {
  const [state, setState] = useState<OverlayState>("idle");
  const [resultText, setResultText] = useState("");
  const [errorText, setErrorText] = useState("");

  const [pasteWarning, setPasteWarning] = useState("");

  useTauriEvent("recording-started", useCallback(() => {
    setPasteWarning("");
    setState("recording");
  }, []));
  // The transcription still succeeded and is on the clipboard, so this is shown
  // alongside the result rather than replacing it with an error.
  useTauriEvent<string>("paste-error", useCallback((msg: string) => setPasteWarning(msg), []));
  useTauriEvent("recording-transcribing", useCallback(() => setState("transcribing"), []));
  useTauriEvent<string>("recording-result", useCallback((text: string) => {
    setResultText(text);
    setState("result");
  }, []));
  useTauriEvent<string>("recording-error", useCallback((err: string) => {
    setErrorText(err);
    setState("error");
  }, []));
  useTauriEvent("recording-idle", useCallback(() => setState("idle"), []));

  if (state === "idle") {
    return null;
  }

  return (
    <div className="flex items-center justify-center w-full h-full">
      <div className="bg-surface-dark/80 backdrop-blur-xl rounded-2xl px-6 py-4 min-w-[280px] max-w-[400px] shadow-2xl border border-white/5">
        {state === "recording" && (
          <div className="text-center">
            <Waveform />
            <p className="text-text-secondary text-xs mt-2">Listening...</p>
          </div>
        )}

        {state === "transcribing" && (
          <div className="text-center py-2">
            <div className="inline-block w-5 h-5 border-2 border-accent border-t-transparent rounded-full animate-spin" />
            <p className="text-text-secondary text-xs mt-2">Transcribing...</p>
          </div>
        )}

        {state === "result" && (
          <>
            <TranscriptDisplay text={resultText} />
            {pasteWarning && (
              <p className="text-yellow-400 text-[11px] leading-snug mt-2 pt-2 border-t border-white/10">
                {pasteWarning}
              </p>
            )}
          </>
        )}

        {state === "error" && (
          <div className="text-center py-2">
            <p className="text-red-400 text-xs">{errorText || "Transcription failed"}</p>
          </div>
        )}
      </div>
    </div>
  );
}
