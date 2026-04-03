import { useCallback, useRef, useState } from "react";
import { useTauriEvent } from "../hooks/useTauriEvent";

export default function Waveform() {
  const [levels, setLevels] = useState<number[]>(new Array(24).fill(0));
  const indexRef = useRef(0);

  const handleLevel = useCallback((level: number) => {
    setLevels((prev) => {
      const next = [...prev];
      next[indexRef.current % 24] = Math.min(level * 8, 1); // Normalize and cap
      indexRef.current += 1;
      return next;
    });
  }, []);

  useTauriEvent<number>("audio-level", handleLevel);

  return (
    <div className="flex items-center justify-center gap-[3px] h-12">
      {levels.map((level, i) => (
        <div
          key={i}
          className="w-[4px] rounded-full bg-accent transition-all duration-75"
          style={{
            height: `${Math.max(4, level * 48)}px`,
            opacity: 0.5 + level * 0.5,
          }}
        />
      ))}
    </div>
  );
}
