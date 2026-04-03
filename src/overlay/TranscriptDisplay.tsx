import { useEffect, useState } from "react";

interface Props {
  text: string;
}

export default function TranscriptDisplay({ text }: Props) {
  const [opacity, setOpacity] = useState(1);

  useEffect(() => {
    // Start fade out after 1.5 seconds
    const timer = setTimeout(() => setOpacity(0), 1500);
    return () => clearTimeout(timer);
  }, []);

  return (
    <div
      className="text-text-primary text-sm leading-relaxed px-2 max-h-20 overflow-hidden transition-opacity duration-1000"
      style={{ opacity }}
    >
      {text}
    </div>
  );
}
