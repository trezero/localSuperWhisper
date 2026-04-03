interface Props {
  label: string;
  description: string;
  completed: boolean;
  onComplete: () => void;
}

export default function ChecklistItem({ label, description, completed, onComplete }: Props) {
  return (
    <button
      onClick={completed ? undefined : onComplete}
      className={`flex items-start gap-3 w-full text-left p-3 rounded-lg transition-colors ${
        completed ? "opacity-50" : "hover:bg-surface-hover"
      }`}
      disabled={completed}
    >
      <div
        className={`mt-0.5 w-5 h-5 rounded-full border-2 flex items-center justify-center flex-shrink-0 ${
          completed ? "border-accent bg-accent" : "border-text-muted"
        }`}
      >
        {completed && (
          <svg className="w-3 h-3 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={3}>
            <path strokeLinecap="round" strokeLinejoin="round" d="M5 13l4 4L19 7" />
          </svg>
        )}
      </div>
      <div>
        <p className="text-sm text-text-primary font-medium">{label}</p>
        <p className="text-xs text-text-secondary">{description}</p>
      </div>
    </button>
  );
}
