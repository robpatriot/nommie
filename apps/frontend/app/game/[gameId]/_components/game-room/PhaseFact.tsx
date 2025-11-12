export function PhaseFact({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <p className="text-xs uppercase tracking-wide text-subtle">{label}</p>
      <p className="text-sm font-medium text-foreground">{value}</p>
    </div>
  )
}
