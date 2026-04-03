interface Props {
  label: string;
  value: string | number;
  sublabel?: string;
}

export default function StatCard({ label, value, sublabel }: Props) {
  return (
    <div className="bg-surface-dark rounded-xl p-4 border border-white/5">
      <p className="text-2xl font-bold text-text-primary">{value}</p>
      <p className="text-xs text-text-secondary mt-1">{label}</p>
      {sublabel && <p className="text-xs text-text-muted">{sublabel}</p>}
    </div>
  );
}
